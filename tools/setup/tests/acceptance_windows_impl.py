"""Implementation tests for the platform-keyed acceptance checker.

Edge cases behind the Windows evidence contract: the FS-UAE source shapes,
the values the producer writes when a step did not hold, the installer name
per platform, and the candidate's single archive version.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE))

import acceptance  # noqa: E402
import acceptance_windows_spec  # noqa: E402


class WindowsAcceptanceImplTests(acceptance_windows_spec.WindowsAcceptanceSpecTests):
    """Reuses the valid Windows record and candidate from the spec suite."""

    def rejected(self, mutate, key: str) -> None:
        self.assert_rejected(mutate, key)

    def test_should_reject_a_downloaded_fs_uae_with_another_zip(self) -> None:
        self.rejected(lambda record: record["fs_uae"].update(sha256="0" * 64), "fs_uae")
        self.rejected(lambda record: record["fs_uae"].update(bytes=1), "fs_uae")

    def test_should_reject_a_download_record_without_the_zip_pin(self) -> None:
        self.rejected(lambda record: record["fs_uae"].pop("sha256"), "fs_uae")

    def test_should_reject_an_existing_fs_uae_that_carries_a_zip_pin(self) -> None:
        self.rejected(lambda record: record["fs_uae"].update(source="existing"), "fs_uae")

    def test_should_reject_an_unknown_fs_uae_source_or_an_old_version(self) -> None:
        self.rejected(lambda record: record["fs_uae"].update(source="bundled"), "fs_uae.source")
        self.rejected(lambda record: record["fs_uae"].update(version="3.1.66"), "3.2.0")

    def test_should_reject_a_plain_fs_uae_version_string_on_windows(self) -> None:
        self.rejected(lambda record: record.update(fs_uae="3.2.35"), "fs_uae")

    def test_should_reject_the_values_written_when_a_step_did_not_hold(self) -> None:
        self.rejected(lambda record: record.update(existing_registration="not-reused"),
                      "existing_registration")
        self.rejected(lambda record: record.update(standard_account="administrator"),
                      "standard_account")
        self.rejected(lambda record: record["manual"].update(editor_build_run=False), "manual")
        self.rejected(lambda record: record["host_hello"].update(result="failed"), "host_hello")

    def test_should_reject_a_truthy_value_that_is_not_true(self) -> None:
        self.rejected(lambda record: record["manual"].update(uac_prompt_seen=1), "manual")
        self.rejected(lambda record: record.update(os_build=True), "os_build")

    def test_should_reject_the_linux_installer_in_a_windows_record(self) -> None:
        # The file exists with matching bytes, so only the name rule rejects it.
        linux = self.candidate / "install.sh"
        linux.write_bytes(b"sh installer\n")
        self.rejected(lambda record: record.update(installer={
            "name": "install.sh", "sha256": acceptance.sha256_file(linux)}), "installer name")

    def test_should_reject_the_linux_archive_name_in_a_windows_record(self) -> None:
        linux = self.candidate / "clorraine-1.2.0-x86_64-linux.tar.gz"
        linux.write_bytes(b"linux archive\n")
        self.rejected(lambda record: record.update(archive={
            "name": linux.name, "sha256": acceptance.sha256_file(linux),
            "bytes": linux.stat().st_size}), "archive name")

    def test_should_reject_a_glibc_key_in_a_windows_record(self) -> None:
        self.rejected(lambda record: record.update(glibc="2.39"), "glibc")

    def test_should_reject_a_candidate_with_two_archive_versions(self) -> None:
        (self.candidate / "clorraine-1.2.1-x86_64-linux.tar.gz").write_bytes(b"other\n")
        with self.assertRaises(acceptance.AcceptanceError) as caught:
            self.check(json.loads(json.dumps(self.evidence)))
        self.assertIn("version", str(caught.exception))

    def test_should_read_the_version_from_a_full_five_asset_candidate(self) -> None:
        (self.candidate / "clorraine-1.2.0-x86_64-linux.tar.gz").write_bytes(b"linux\n")
        (self.candidate / "install.sh").write_bytes(b"sh\n")
        self.assertEqual(acceptance.candidate_version(self.candidate), "1.2.0")
        self.check(self.evidence)


if __name__ == "__main__":
    unittest.main()
