"""Core archive contracts and separately selected mandatory actual-artifact proof.

Whole-file discovery includes the real groups: missing explicit inputs are
errors, never skips. Select CoreArchiveTests and CoreStageArgumentTests for
local-only work. Tiny ELF version tools exercise packaging, not compiler,
Python-wheel, Amiga or relocation compatibility.
"""

from __future__ import annotations

import os
import shutil
import tarfile
import tempfile
import unittest
from pathlib import Path

from core_package_fixtures import (
    BINARIES, CoreFixture, current_version, digest_file, inspect_core, invoke, scratch_parent,
)
from core_package_stage_spec import CorePackageArtifactAcceptance
from core_package_installed_spec import CoreInstalledAcceptance
from core_package_snapshot_spec import CoreSnapshotTests


class CoreArchiveTests(unittest.TestCase):
    """Exercise the public archive command using real executable ELF test data."""

    def setUp(self) -> None:
        """Own all test files and register cleanup before compiling fixtures."""
        scratch = tempfile.TemporaryDirectory(prefix="core-spec-", dir=scratch_parent())
        self.addCleanup(scratch.cleanup)
        self.fixture = CoreFixture(Path(scratch.name))

    def _archive(self, output: Path | None = None):
        """Require unchanged caller inputs and cleanup after every invocation."""
        fixture = self.fixture
        before = fixture.binary_state()
        result = fixture.archive(self, output)
        self.assertEqual(fixture.binary_state(), before, "producer changed binary inputs")
        fixture.assert_no_side_effects(self)
        return result

    def _rejected(self, identity: str) -> None:
        """Require a named content failure, not an incidental parser/launch error."""
        result = self._archive()
        self._failed(result, identity)
        self.assertFalse(self.fixture.output.exists(), "failed archive was published")
        self.assertEqual(list(self.fixture.published.iterdir()), [], "failed archive left private work")

    def _failed(self, result, identity: str) -> None:
        """Name the failed tool or output, without prescribing an English sentence."""
        self.assertEqual(result.returncode, 1, result.stderr)
        names = [identity]
        if identity == "output":
            names.append(self.fixture.output.name)
        self.assertTrue(any(name.lower() in result.stderr.lower() for name in names), result.stderr)

    def test_should_archive_exact_current_binaries_notices_and_metadata(self) -> None:
        """Preserve the eight inputs and describe their bytes in the ninth file."""
        fixture = self.fixture
        result = self._archive()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(str(fixture.output), result.stdout)
        inspect_core(fixture.output, fixture.expected, current_version())

    def test_should_query_the_supplied_binaries_not_path_decoys(self) -> None:
        """Private query snapshots preserve supplied and archived executable bytes."""
        fixture = self.fixture
        result = self._archive()
        self.assertEqual(result.returncode, 0, result.stderr)
        inspect_core(fixture.output, fixture.expected, fixture.version)
        with tarfile.open(fixture.output, "r:") as archive:
            for name in BINARIES:
                fields = (fixture.queries / (name + ".argv")).read_bytes().split(b"\0")
                self.assertEqual(fields[2], b"--version")
                self.assertEqual(len(fields), 4, "exactly one version argument")
                supplied = (fixture.binaries / name).resolve()
                observed = fixture.queries / (name + ".elf")
                self.assertEqual(observed.read_bytes(), supplied.read_bytes(),
                                 f"{name}: queried executable differs from supplied bytes")
                self.assertEqual(observed.read_bytes(), archive.extractfile("bin/" + name).read(),
                                 f"{name}: queried executable differs from archived bytes")
                self.assertEqual(digest_file(observed), fixture.expected["bin/" + name]["sha256"])
                actual = Path(fields[0].decode())
                self.assertTrue(actual.is_absolute(), "record the actual executed inode's path")
                self.assertNotEqual(actual, supplied, "query a private snapshot, not mutable input")
                self.assertFalse(actual.exists(), "private queried executable was not cleaned")
                self.assertFalse(actual.parent.exists(), "private snapshot directory was not cleaned")

    def test_should_be_deterministic_despite_input_mtime_and_permissions(self) -> None:
        """Identical file bytes have stable tar bytes, independent of host metadata."""
        fixture = self.fixture
        first = self._archive()
        self.assertEqual(first.returncode, 0, first.stderr)
        identity = digest_file(fixture.output)
        for name in BINARIES:
            path = fixture.binaries / name
            os.utime(path, (1_234_567_890, 1_234_567_890))
            path.chmod(0o700)
        moved = fixture.root / "moved binary inputs ; literal"
        moved.mkdir()
        for name in reversed(BINARIES):
            shutil.copy2(fixture.binaries / name, moved / name)
        fixture.binaries = moved
        second_path = fixture.published / "again.tar"
        second = self._archive(second_path)
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual(digest_file(second_path), identity)
        inspect_core(second_path, fixture.expected, fixture.version)

    def test_should_exclude_adjacent_caches_media_and_untrusted_notice_copies(self) -> None:
        """Only three named binaries come from the input directory."""
        fixture = self.fixture
        for name in ("extra.c", "kickstart.rom", "LICENSE", "core.json"):
            (fixture.binaries / name).write_bytes(b"must not enter the archive\n")
        (fixture.binaries / "cache").mkdir()
        (fixture.binaries / "cache/object.o").write_bytes(b"unrelated object\n")
        result = self._archive()
        self.assertEqual(result.returncode, 0, result.stderr)
        inspect_core(fixture.output, fixture.expected, fixture.version)

    def test_should_reject_each_missing_binary_without_path_fallback(self) -> None:
        """A PATH tool cannot replace a missing explicitly supplied input."""
        fixture = self.fixture
        for name in BINARIES:
            with self.subTest(name=name):
                path = fixture.binaries / name
                backup = fixture.root / ("saved-" + name)
                path.rename(backup)
                try:
                    self._rejected(name)
                finally:
                    backup.rename(path)

    def test_should_reject_non_executable_binary(self) -> None:
        """The producer must not repair input permissions to make them eligible."""
        (self.fixture.binaries / "clc").chmod(0o644)
        self._rejected("clc")

    def test_should_reject_symlink_binary_input(self) -> None:
        """Even an internal alias is not one of the required ordinary files."""
        fixture = self.fixture
        path = fixture.binaries / "clc"
        path.rename(fixture.binaries / "original-clc")
        path.symlink_to("original-clc")
        self._rejected("clc")

    def test_should_reject_directory_and_fifo_binary_inputs(self) -> None:
        """Reject special/non-file inputs without blocking on a FIFO read."""
        fixture = self.fixture
        path = fixture.binaries / "clc"
        saved = fixture.root / "saved-clc"
        path.rename(saved)
        try:
            os.mkfifo(path)
            self._rejected("clc")
            path.unlink()
            path.mkdir()
            self._rejected("clc")
            path.rmdir()
        finally:
            if not path.exists():
                saved.rename(path)

    def test_should_reject_script_even_if_it_reports_the_right_version(self) -> None:
        """A successful banner does not turn a script into Linux x86_64 ELF."""
        path = self.fixture.binaries / "clc"
        path.write_text(f"#!/bin/sh\nprintf 'clc {self.fixture.version}\\n'\n", encoding="utf-8")
        path.chmod(0o755)
        self._rejected("clc")

    def test_should_reject_wrong_elf_class_and_machine(self) -> None:
        """Inspect actual ELF identity, not just the executable bit or filename."""
        path = self.fixture.binaries / "clc"
        original = path.read_bytes()
        for offset, data in ((4, b"\x01"), (18, (183).to_bytes(2, "little"))):
            with self.subTest(offset=offset):
                changed = bytearray(original)
                changed[offset:offset + len(data)] = data
                path.write_bytes(changed)
                self._rejected("clc")
        path.write_bytes(original)

    def test_should_reject_version_mismatch_for_each_tool(self) -> None:
        """All three binaries must agree with the current manifest version."""
        for name in BINARIES:
            with self.subTest(name=name):
                key = "CORE_VERSION_" + name
                self.fixture.environment[key] = "97.98.99"
                try:
                    self._rejected(name)
                finally:
                    del self.fixture.environment[key]

    def test_should_reject_a_banner_naming_another_tool(self) -> None:
        """Matching numbers alone do not identify a compiler component."""
        self.fixture.environment["CORE_NAME_clc"] = "unrelated-tool"
        self._rejected("clc")

    def test_should_reject_matching_banners_that_differ_from_manifest(self) -> None:
        """Mutual agreement is not permission to ignore the current Cargo version."""
        fixture = self.fixture
        for name in BINARIES:
            fixture.environment["CORE_VERSION_" + name] = "97.98.99"
        result = self._archive()
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertTrue(any(value in result.stderr
                            for value in ("version", "Cargo.toml", fixture.version, "97.98.99")),
                        result.stderr)
        self.assertFalse(fixture.output.exists())
        self.assertEqual(list(fixture.published.iterdir()), [])

    def test_should_reject_an_empty_version_banner(self) -> None:
        """A zero process code without tool/version text is insufficient."""
        self.fixture.environment["CORE_EMPTY_clc"] = "1"
        self._rejected("clc")

    def test_should_reject_nonzero_version_query(self) -> None:
        """A failed version process is a named content/prerequisite failure."""
        self.fixture.environment["CORE_STATUS_clc"] = "19"
        self._rejected("clc")

    def test_should_reject_signalled_version_query(self) -> None:
        """A version query terminated without a code must not publish an archive."""
        self.fixture.environment["CORE_SIGNAL_clc"] = "1"
        self._rejected("clc")

    def test_should_refuse_existing_archive_without_replacement(self) -> None:
        """Publication cannot overwrite user bytes or leave sibling temporary files."""
        fixture = self.fixture
        fixture.output.write_bytes(b"existing user archive\n")
        result = self._archive()
        self._failed(result, "output")
        self.assertEqual(fixture.output.read_bytes(), b"existing user archive\n")
        self.assertEqual(list(fixture.published.iterdir()), [fixture.output])

    def test_should_refuse_existing_directory_output(self) -> None:
        """A pre-existing directory and its contents are not disposable staging."""
        fixture = self.fixture
        fixture.output.mkdir()
        sentinel = fixture.output / "keep"
        sentinel.write_bytes(b"keep directory contents\n")
        result = self._archive()
        self._failed(result, "output")
        self.assertEqual(sentinel.read_bytes(), b"keep directory contents\n")
        self.assertEqual(list(fixture.published.iterdir()), [fixture.output])

    def test_should_refuse_existing_and_dangling_output_symlinks(self) -> None:
        """Do not follow or replace aliases, including aliases to absent files."""
        fixture = self.fixture
        target = fixture.root / "outside archive"
        for present in (False, True):
            with self.subTest(present=present):
                if present:
                    target.write_bytes(b"outside archive contents\n")
                fixture.output.symlink_to(target)
                try:
                    result = self._archive()
                    self._failed(result, "output")
                    self.assertTrue(fixture.output.is_symlink())
                    if present:
                        self.assertEqual(target.read_bytes(), b"outside archive contents\n")
                    else:
                        self.assertFalse(target.exists())
                finally:
                    fixture.output.unlink()

    def test_should_classify_archive_argument_errors_as_usage(self) -> None:
        """Unknown options and incomplete required options are exit two."""
        fixture = self.fixture
        cases = [
            [], ["--binaries"], ["--binaries", fixture.binaries, "--output"],
            ["--binaries", fixture.binaries, "--output", fixture.output, "--unknown-core-option"],
        ]
        for arguments in cases:
            with self.subTest(arguments=arguments):
                result = invoke(self, "archive", arguments, fixture.root, fixture.environment)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertTrue(result.stderr)
                self.assertFalse(fixture.output.exists())
                fixture.assert_no_side_effects(self)


class CoreStageArgumentTests(unittest.TestCase):
    """Parse the exact stage interface without inventing Python probe responses."""

    def setUp(self) -> None:
        """Create passive argument paths; content validation belongs to real groups."""
        scratch = tempfile.TemporaryDirectory(prefix="core-stage-args-", dir=scratch_parent())
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        self.environment = {
            "PATH": "/usr/bin:/bin", "HOME": str(self.root), "LANG": "C.UTF-8",
            "PYTHONDONTWRITEBYTECODE": "1", "TMPDIR": str(self.root),
            "CODEOPS_TMPDIR": str(self.root),
        }
        self.flags = ("--core", "--lock", "--sdk-archive", "--sdk-audit",
                      "--preparation-archive", "--preparation-audit",
                      "--wheels-archive", "--wheels-audit", "--ndk", "--python", "--output")

    def _usage(self, arguments: list) -> None:
        """Require usage classification before any passive fixture path is consumed."""
        result = invoke(self, "stage", arguments, self.root, self.environment)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertTrue(result.stderr)
        self.assertEqual(list(self.root.iterdir()), [], "usage error created output or private work")

    def test_should_require_every_stage_option(self) -> None:
        """Every stage input and destination is required by the public signature."""
        for omitted in self.flags:
            with self.subTest(omitted=omitted):
                arguments = []
                for flag in self.flags:
                    if flag != omitted:
                        arguments.extend([flag, self.root / flag[2:]])
                self._usage(arguments)

    def test_should_require_a_value_for_each_stage_option(self) -> None:
        """A trailing flag must not become an implicit default or a path."""
        for flag in self.flags:
            with self.subTest(flag=flag):
                arguments = []
                for supplied in self.flags:
                    if supplied != flag:
                        arguments.extend([supplied, self.root / supplied[2:]])
                self._usage([*arguments, flag])

    def test_should_reject_unknown_stage_options(self) -> None:
        """Do not accept a guessed alternate stage protocol."""
        arguments = []
        for flag in self.flags:
            arguments.extend([flag, self.root / flag[2:]])
        self._usage([*arguments, "--unknown-core-stage-option"])
