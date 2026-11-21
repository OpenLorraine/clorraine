"""Focused support internals; synthetic files do not replace real archive proof."""

from __future__ import annotations

import copy
import hashlib
import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import prepare_ndk
from support import SupportError, load_lock, validate_lock
from support_build import inventory
from support_fixtures import SupportFixture, digest


class SupportLockImplementationTests(unittest.TestCase):
    """Exercise loader identity and schema behaviour independently of command parsing."""

    def setUp(self) -> None:
        """Own a valid synthetic lock and its staged source fixtures."""
        scratch = tempfile.TemporaryDirectory(
            prefix="support-lock-impl-", dir=os.environ.get("CODEOPS_TMPDIR"))
        self.addCleanup(scratch.cleanup)
        self.fixture = SupportFixture(Path(scratch.name))

    def test_should_hash_exact_lock_bytes_including_whitespace(self) -> None:
        """Equivalent JSON may parse equally but must not share a provenance digest."""
        path = self.fixture.lock_path
        first, first_hash = load_lock(path)
        raw = path.read_bytes() + b" \n"
        path.write_bytes(raw)
        second, second_hash = load_lock(path)
        self.assertEqual(first, second)
        self.assertNotEqual(first_hash, second_hash)
        self.assertEqual(second_hash, hashlib.sha256(raw).hexdigest())

    def test_should_validate_without_rewriting_nested_records(self) -> None:
        """Validation retains the caller's original object and exact source metadata."""
        value = copy.deepcopy(self.fixture.lock)
        before = copy.deepcopy(value)
        self.assertIs(validate_lock(value), value)
        self.assertEqual(value, before)

    def test_should_reject_duplicate_keys_inside_nested_records(self) -> None:
        """Duplicate protection applies below the root, not only to schema keys."""
        raw = self.fixture.lock_path.read_bytes()
        self.assertEqual(raw.count(b'"kind":"git"'), 1)
        self.fixture.lock_path.write_bytes(
            raw.replace(b'"kind":"git"', b'"kind":"archive","kind":"git"'))
        with self.assertRaisesRegex(SupportError, "duplicate JSON key 'kind'"):
            load_lock(self.fixture.lock_path)

    def test_should_reject_non_utf8_lock_bytes_with_a_named_error(self) -> None:
        """Invalid encoding is a user-facing metadata error rather than a traceback."""
        self.fixture.lock_path.write_bytes(b"\xff")
        with self.assertRaisesRegex(SupportError, "cannot read UTF-8 JSON lock"):
            load_lock(self.fixture.lock_path)


class SupportInventoryImplementationTests(unittest.TestCase):
    """Inspect actual files and links without a container or permission to escape the root."""

    def setUp(self) -> None:
        """Create one ordinary payload and keep every mutation inside owned scratch."""
        scratch = tempfile.TemporaryDirectory(
            prefix="support-inventory-impl-", dir=os.environ.get("CODEOPS_TMPDIR"))
        self.addCleanup(scratch.cleanup)
        self.parent = Path(scratch.name)
        self.root = self.parent / "payload"
        self.root.mkdir()
        (self.root / "data").write_bytes(b"payload\n")

    def test_should_record_file_bytes_and_directory_aliases_without_dereferencing(self) -> None:
        """Directory aliases are inventory links, not a second traversal of their contents."""
        (self.root / "include").mkdir()
        (self.root / "include/types.h").write_bytes(b"header\n")
        (self.root / "headers").symlink_to("include", target_is_directory=True)
        expected = {
            "data": {"type": "file", "sha256": digest(b"payload\n")},
            "include/types.h": {"type": "file", "sha256": digest(b"header\n")},
            "headers": {"type": "symlink", "target": "include"},
        }
        self.assertEqual(inventory(self.root), expected)

    def test_should_detect_changed_bytes_without_relying_on_name_or_length(self) -> None:
        """Same-length file replacement cannot preserve an old inventory identity."""
        before = inventory(self.root)
        (self.root / "data").write_bytes(b"changed\n")
        self.assertNotEqual(inventory(self.root), before)

    def test_should_reject_dangling_and_cyclic_links(self) -> None:
        """Unresolvable aliases must fail before a successful inventory can be published."""
        path = self.root / "alias"
        for target in ("absent", "alias"):
            with self.subTest(target=target):
                path.symlink_to(target)
                try:
                    with self.assertRaisesRegex(SupportError, "alias: dangling or cyclic"):
                        inventory(self.root)
                finally:
                    path.unlink()

    def test_should_reject_a_relative_link_escaping_to_a_real_sibling(self) -> None:
        """An existing outside target does not make a relative link safe."""
        outside = self.parent / "outside"
        outside.write_bytes(b"outside unchanged\n")
        (self.root / "alias").symlink_to("../outside")
        with self.assertRaisesRegex(SupportError, "alias: escaping payload link"):
            inventory(self.root)
        self.assertEqual(outside.read_bytes(), b"outside unchanged\n")

    def test_should_reject_special_permissions_and_ambiguous_names(self) -> None:
        """Executable files may be normal, but privilege bits and unsafe names are not."""
        data = self.root / "data"
        data.chmod(0o4755)
        with self.assertRaisesRegex(SupportError, "special permission bits"):
            inventory(self.root)
        data.chmod(0o755)
        self.assertIn("data", inventory(self.root))
        for name in ("bad!name", "back\\slash", "line\nbreak", "control\u0085"):
            with self.subTest(name=name):
                path = self.root / name
                path.write_bytes(b"untrusted\n")
                try:
                    with self.assertRaisesRegex(SupportError, "unsafe payload path"):
                        inventory(self.root)
                finally:
                    path.unlink()

    def test_should_reject_a_symlink_payload_root(self) -> None:
        """A root alias must not redirect inspection to a caller-selected tree."""
        alias = self.parent / "alias"
        alias.symlink_to(self.root, target_is_directory=True)
        with self.assertRaisesRegex(SupportError, "ordinary payload directory"):
            inventory(alias)


class SupportPreparationImplementationTests(unittest.TestCase):
    """Check exact byte patching and complete member-graph handling without licensed data."""

    def setUp(self) -> None:
        """Own original bytes and a small multi-hunk unified patch."""
        scratch = tempfile.TemporaryDirectory(
            prefix="support-preparation-impl-", dir=os.environ.get("CODEOPS_TMPDIR"))
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        self.source = self.root / "header.h"
        self.original = b"first\nold\nmiddle\nbefore\nlast\n"
        self.source.write_bytes(self.original)
        self.patch = self.root / "header.diff"
        self.patch.write_bytes(
            b"--- a/header.h\n+++ b/header.h\n"
            b"@@ -1,2 +1,2 @@\n first\n-old\n+new\n"
            b"@@ -4,2 +4,3 @@\n-before\n+after\n+inserted\n last\n")

    def test_should_apply_exact_multiple_hunks_and_preserve_unmentioned_bytes(self) -> None:
        """Later hunks use original offsets; unchanged middle and suffix bytes survive."""
        prepare_ndk._patch(self.source, self.patch)
        self.assertEqual(self.source.read_bytes(), b"first\nnew\nmiddle\nafter\ninserted\nlast\n")

    def test_should_leave_original_bytes_when_a_later_hunk_does_not_match(self) -> None:
        """A valid first hunk must not leak partial edits when a later hunk fails."""
        self.patch.write_bytes(self.patch.read_bytes().replace(b"-before\n", b"-wrong\n"))
        with self.assertRaisesRegex(ValueError, "context does not match"):
            prepare_ndk._patch(self.source, self.patch)
        self.assertEqual(self.source.read_bytes(), self.original)

    def test_should_refuse_shifted_context_instead_of_searching_for_a_match(self) -> None:
        """Matching text elsewhere cannot authorise patching at an unauthenticated offset."""
        shifted = b"prefix\n" + self.original
        self.source.write_bytes(shifted)
        with self.assertRaisesRegex(ValueError, "context does not match"):
            prepare_ndk._patch(self.source, self.patch)
        self.assertEqual(self.source.read_bytes(), shifted)

    def test_should_reject_wrong_hunk_counts_without_writing(self) -> None:
        """Declared hunk lengths are checked even when every context line matches."""
        self.patch.write_bytes(self.patch.read_bytes().replace(b"+4,3", b"+4,4"))
        with self.assertRaisesRegex(ValueError, "hunk length mismatch"):
            prepare_ndk._patch(self.source, self.patch)
        self.assertEqual(self.source.read_bytes(), self.original)

    def test_should_reject_repeated_patch_application_without_changing_output(self) -> None:
        """The exact-context operation is deliberately not an implicit repair or merge."""
        prepare_ndk._patch(self.source, self.patch)
        expected = self.source.read_bytes()
        with self.assertRaisesRegex(ValueError, "context does not match"):
            prepare_ndk._patch(self.source, self.patch)
        self.assertEqual(self.source.read_bytes(), expected)

    def test_should_resolve_archive_link_chains_against_implicit_directories(self) -> None:
        """Safe nested aliases resolve against the complete listing regardless of entry order."""
        listing = [
            {"path": "alias", "type": "symlink", "target": "second"},
            {"path": "second", "type": "symlink", "target": "include"},
            {"path": "include/types.h", "type": "file", "target": None},
        ]
        self.assertIsNone(prepare_ndk.validate_archive_members(listing))

    def test_should_reject_a_link_chain_that_treats_a_file_as_a_directory(self) -> None:
        """A valid final filename does not excuse traversal through an ordinary file."""
        listing = [
            {"path": "alias", "type": "symlink", "target": "data/child"},
            {"path": "data", "type": "file", "target": None},
        ]
        with self.assertRaisesRegex(ValueError, "alias: dangling or non-directory"):
            prepare_ndk.validate_archive_members(listing)
