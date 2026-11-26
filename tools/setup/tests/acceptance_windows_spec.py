"""Contract tests for Windows acceptance evidence.

The checker is platform-keyed: a shared core (version, platform, architecture,
the candidate files, the rc24 pin, FS-UAE, the Kickstart, the guest hello, the
negative, and the corpus) plus the facts each platform adds. A Windows record
names the Windows build, the rc24 setup, the zip and install.ps1, how FS-UAE
was obtained, the PC hello, the reused registration, the standard-account
install, and the two in-person confirmations. The version comes from the
candidate's archive names. The Linux fixture record keeps validating.
"""

from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE))

import acceptance  # noqa: E402
import acceptance_spec  # noqa: E402

VERSION = "1.2.0"
ZIP_NAME = f"clorraine-{VERSION}-x86_64-windows.zip"
VSIX_NAME = f"clorraine-{VERSION}.vsix"
HELLO = "Hello, Amiga, from Clorraine!"
RC24_SETUP_BYTES = 48_065_003
RC24_SETUP_SHA256 = "9dd6973539bb2d514fc1b1e6c0a107cac6f30a9c4076cf4014cbe13013c95512"
FS_UAE_ZIP_BYTES = 49_123_213
FS_UAE_ZIP_SHA256 = "b402e2802d518a085c878be96c5c0a656fa3dec57dcf0019d226f3285069063e"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class WindowsAcceptanceSpecTests(unittest.TestCase):
    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory()
        self.candidate = Path(self.scratch.name) / "candidate"
        self.candidate.mkdir()
        self.archive = self.candidate / ZIP_NAME
        self.vsix = self.candidate / VSIX_NAME
        self.installer = self.candidate / "install.ps1"
        self.archive.write_bytes(b"windows zip fixture bytes\n")
        self.vsix.write_bytes(b"vsix fixture bytes\n")
        self.installer.write_bytes(b"powershell installer fixture bytes\n")
        self.evidence = self._valid_record()

    def tearDown(self) -> None:
        self.scratch.cleanup()

    def _valid_record(self) -> dict:
        archive_bytes = self.archive.read_bytes()
        vsix_bytes = self.vsix.read_bytes()
        corpus = [
            {"name": name, "profile": "hosted-amiga", "status": "ok",
             "output_sha256": acceptance_spec.expected_corpus(name)}
            for name in ("file-copier", "string-parser", "buffer-container", "volatile-mmio")
        ] + [{"name": "file-copier", "profile": "hosted-amiga-13", "status": "ok",
              "output_sha256": acceptance_spec.expected_corpus("file-copier")}]
        return {
            "version": VERSION,
            "platform": "windows",
            "architecture": "x86_64",
            "os_build": 26100,
            "archive": {"name": ZIP_NAME, "sha256": digest(archive_bytes),
                        "bytes": len(archive_bytes)},
            "installer": {"name": "install.ps1",
                          "sha256": digest(self.installer.read_bytes())},
            "vsix": {"name": VSIX_NAME, "sha256": digest(vsix_bytes),
                     "bytes": len(vsix_bytes)},
            "toolchain": {"tag": "v16.2-rc24", "sha256": RC24_SETUP_SHA256,
                          "bytes": RC24_SETUP_BYTES},
            "kickstart": {
                "name": "amiga-os-310-a1200.rom",
                "sha256": "541f7467bce4f4b07a6eb5250d4586a4679c2da51ad30b48ba93421a901b1bb0",
            },
            "fs_uae": {"version": "3.2.35", "source": "downloaded",
                       "sha256": FS_UAE_ZIP_SHA256, "bytes": FS_UAE_ZIP_BYTES},
            "guest": {"status": "ok", "rc": 0, "output": HELLO,
                      "output_sha256": digest(HELLO.encode())},
            "negative": {"status": "failed", "rc": 1},
            "corpus": corpus,
            "host_hello": {"compiler": "gcc 15.2.0", "result": "ok"},
            "existing_registration": "reused",
            "standard_account": "installed",
            "manual": {"uac_prompt_seen": True, "editor_build_run": True},
            "timestamp": "2026-11-27T10:00:00Z",
        }

    def check(self, record: dict) -> None:
        acceptance.check(record, self.candidate)

    def assert_rejected(self, mutate, key: str) -> None:
        record = json.loads(json.dumps(self.evidence))
        mutate(record)
        with self.assertRaises(acceptance.AcceptanceError) as caught:
            self.check(record)
        self.assertIn(key, str(caught.exception))

    # ST-70: a valid Windows record passes, and so does the Linux fixture record.
    def test_should_accept_a_valid_windows_record(self) -> None:
        self.check(self.evidence)

    def test_should_accept_an_fs_uae_the_person_already_had(self) -> None:
        record = json.loads(json.dumps(self.evidence))
        record["fs_uae"] = {"version": "3.2.35", "source": "existing"}
        self.check(record)

    def test_should_keep_accepting_the_linux_fixture_record(self) -> None:
        linux = acceptance_spec.AcceptanceSpecTests("test_valid_record_passes")
        linux.setUp()
        try:
            acceptance.check(linux.evidence, linux.candidate)
        finally:
            linux.tearDown()

    # ST-71: each wrong Windows fact fails and names its key.
    def test_should_reject_the_linux_rc24_pin_in_a_windows_record(self) -> None:
        self.assert_rejected(lambda record: record.update(toolchain={
            "tag": "v16.2-rc24",
            "sha256": "b93b070ac47d2f01d126f57c03a8405f71fb470c8397b0bda51041217d25c100",
            "bytes": 69_868_548,
        }), "toolchain")

    def test_should_reject_a_windows_build_older_than_windows_11(self) -> None:
        self.assert_rejected(lambda record: record.update(os_build=19045), "os_build")

    def test_should_reject_a_record_without_the_uac_confirmation(self) -> None:
        self.assert_rejected(lambda record: record["manual"].pop("uac_prompt_seen"), "manual")

    def test_should_reject_an_unknown_platform(self) -> None:
        self.assert_rejected(lambda record: record.update(platform="macos"), "platform")

    # ST-72: the record's version must be the candidate's archive version.
    def test_should_reject_a_version_other_than_the_candidates(self) -> None:
        self.assert_rejected(lambda record: record.update(version="1.2.1"), "version")


if __name__ == "__main__":
    unittest.main()
