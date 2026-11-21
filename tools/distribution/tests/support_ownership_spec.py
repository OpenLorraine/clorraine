"""Actual-file ownership rules; only the external dpkg-query process is replaced."""

from __future__ import annotations

import os
from pathlib import Path
import subprocess
import tempfile
import unittest

import support_artifact_spec


class SupportOwnershipTests(unittest.TestCase):
    """Keep canonical-file ownership positive and negative evidence independent."""

    def setUp(self) -> None:
        """Use real files and symlinks so canonicalisation is not simulated."""
        scratch = tempfile.TemporaryDirectory(
            prefix="support-owner-spec-", dir=os.environ.get("CODEOPS_TMPDIR"))
        self.addCleanup(scratch.cleanup)
        root = Path(scratch.name)
        self.target = root / "usr/lib/loader.so"
        self.target.parent.mkdir(parents=True)
        self.target.write_bytes(b"Ownership protocol fixture; not an ELF artifact.\n")
        self.alias = root / "lib64/loader.so"
        self.alias.parent.mkdir()
        self.alias.symlink_to("../usr/lib/loader.so")
        self.calls = []
        self.responses = {}

    def _query(self, arguments: list, check: bool = True) -> subprocess.CompletedProcess:
        """Replace the true external dpkg process, not path or filesystem behaviour."""
        self.assertEqual([str(value) for value in arguments[:2]], ["dpkg-query", "-S"])
        self.assertFalse(check)
        path = str(arguments[2])
        self.calls.append(path)
        code, output = self.responses.get(path, (1, b""))
        return subprocess.CompletedProcess(arguments, code, stdout=output, stderr=b"")

    def _owners(self, path: Path | None = None) -> set[str]:
        """Exercise the same owner resolution used by mandatory artifact acceptance."""
        resolver = getattr(support_artifact_spec, "loaded_file_owners", None)
        self.assertIsNotNone(resolver, "Missing actual-loaded-file ownership resolver")
        return resolver(self.alias if path is None else path, self._query)

    def test_should_check_the_actual_resolved_file_instead_of_its_alias(self) -> None:
        """A normal loader symlink is owned through its genuine canonical target."""
        self.responses[str(self.alias)] = (
            0, f"diversion by libc6 from: {self.alias}\n".encode())
        self.responses[str(self.target)] = (
            0, f"libc6:amd64: {self.target}\n".encode())
        self.assertEqual(self._owners(), {"libc6"})
        self.assertEqual(self.calls, [str(self.target)])

    def test_should_never_treat_diversion_commentary_as_an_owner(self) -> None:
        """Successful query status with only redirection records proves no owner."""
        self.responses[str(self.target)] = (
            0, (f"diversion by libc6 from: {self.target}\n"
                f"diversion by libc6 to: {self.target}.usr-is-merged\n").encode())
        self.assertEqual(self._owners(), set())

    def test_should_reject_a_missing_owner_despite_an_existing_file(self) -> None:
        """Filesystem existence cannot replace dpkg ownership evidence."""
        self.assertEqual(self._owners(), set())

    def test_should_not_search_for_an_expected_owner_after_a_wrong_canonical_owner(self) -> None:
        """A genuine wrong owner remains wrong even when an alias suggests libc6."""
        self.responses[str(self.target)] = (
            0, f"other-package:amd64: {self.target}\n".encode())
        self.responses[str(self.alias)] = (0, f"libc6:amd64: {self.alias}\n".encode())
        actual = self._owners()
        self.assertEqual(actual, {"other-package"})
        self.assertNotIn("libc6", actual)
        self.assertEqual(self.calls, [str(self.target)])

    def test_should_reject_owner_records_for_a_different_file(self) -> None:
        """Valid package syntax does not authenticate a different destination."""
        self.responses[str(self.target)] = (
            0, f"libc6:amd64: {self.target}.other\n".encode())
        self.assertEqual(self._owners(), set())

    def test_should_preserve_real_multiple_owner_records_and_normalise_architecture(self) -> None:
        """Genuine Debian owner names retain plus signs and optional architecture suffixes."""
        self.responses[str(self.target)] = (
            0, f"libstdc++6:amd64, other-package:amd64: {self.target}\n".encode())
        self.assertEqual(self._owners(), {"libstdc++6", "other-package"})
