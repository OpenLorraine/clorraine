"""Contract tests for the local Clorraine acceptance-evidence checker.

The checker validates a recorded run against the exact local candidate. These
tests build a valid record and mutate one field at a time, so every required
check has a failing control.
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

import acceptance  # noqa: E402

ARCHIVE_NAME = "clorraine-1.2.0-x86_64-linux.tar.gz"
VSIX_NAME = "clorraine-1.2.0.vsix"
HELLO = "Hello, Amiga, from Clorraine!"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def repo_root() -> Path:
    return HERE.parents[2]


def expected_corpus(name: str) -> str:
    data = (repo_root() / "spec/tests/corpus" / name / "expected-output.txt").read_bytes()
    return digest(data.replace(b"\r", b""))


class AcceptanceSpecTests(unittest.TestCase):
    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory()
        self.candidate = Path(self.scratch.name) / "candidate"
        self.candidate.mkdir()
        self.archive = self.candidate / ARCHIVE_NAME
        self.vsix = self.candidate / VSIX_NAME
        self.installer = self.candidate / "install.sh"
        self.archive.write_bytes(b"archive fixture bytes\n")
        self.vsix.write_bytes(b"vsix fixture bytes\n")
        self.installer.write_bytes(b"installer fixture bytes\n")
        self.evidence = self._valid_record()

    def tearDown(self) -> None:
        self.scratch.cleanup()

    def _valid_record(self) -> dict:
        archive_bytes = self.archive.read_bytes()
        vsix_bytes = self.vsix.read_bytes()
        hello_bytes = HELLO.encode()
        return {
            "version": "1.2.0",
            "platform": "linux",
            "architecture": "x86_64",
            "glibc": "2.39",
            "archive": {
                "name": ARCHIVE_NAME,
                "sha256": digest(archive_bytes),
                "bytes": len(archive_bytes),
            },
            "installer": {"sha256": digest(self.installer.read_bytes())},
            "vsix": {
                "name": VSIX_NAME,
                "sha256": digest(vsix_bytes),
                "bytes": len(vsix_bytes),
            },
            "toolchain": {
                "tag": "v16.2-rc24",
                "sha256": "b93b070ac47d2f01d126f57c03a8405f71fb470c8397b0bda51041217d25c100",
                "bytes": 69_868_548,
            },
            "kickstart": {
                "name": "amiga-os-310-a1200.rom",
                "sha256": "541f7467bce4f4b07a6eb5250d4586a4679c2da51ad30b48ba93421a901b1bb0",
            },
            "fs_uae": "3.2.35",
            "guest": {
                "status": "ok",
                "rc": 0,
                "output": HELLO,
                "output_sha256": digest(hello_bytes),
            },
            "negative": {"status": "failed", "rc": 1},
            "corpus": [
                {"name": name, "profile": "hosted-amiga", "status": "ok",
                 "output_sha256": expected_corpus(name)}
                for name in ("file-copier", "string-parser", "buffer-container", "volatile-mmio")
            ] + [{
                "name": "file-copier",
                "profile": "hosted-amiga-13",
                "status": "ok",
                "output_sha256": expected_corpus("file-copier"),
            }],
            "timestamp": "2026-11-24T23:15:00Z",
        }

    def reject(self, mutate, label: str) -> None:
        record = json.loads(json.dumps(self.evidence))
        mutate(record)
        with self.assertRaises(acceptance.AcceptanceError, msg=label):
            acceptance.check(record, self.candidate)

    def test_valid_record_passes(self) -> None:
        acceptance.check(self.evidence, self.candidate)

    def test_wrong_version_is_rejected(self) -> None:
        self.reject(lambda record: record.update(version="1.1.2"), "version")

    def test_non_ok_guest_status_is_rejected(self) -> None:
        self.reject(lambda record: record["guest"].update(status="failed"), "guest status")

    def test_non_ok_corpus_status_is_rejected(self) -> None:
        self.reject(
            lambda record: record["corpus"][0].update(status="failed"),
            "corpus status",
        )

    def test_path_like_kickstart_name_is_rejected(self) -> None:
        self.reject(
            lambda record: record["kickstart"].update(name="../x.rom"),
            "kickstart name",
        )

    def test_malformed_kickstart_hash_is_rejected(self) -> None:
        self.reject(
            lambda record: record["kickstart"].update(sha256="not-a-hash"),
            "kickstart hash",
        )

    def test_empty_timestamp_is_rejected(self) -> None:
        self.reject(lambda record: record.update(timestamp=""), "timestamp")

    def test_wrong_archive_name_is_rejected(self) -> None:
        self.reject(
            lambda record: record["archive"].update(name="anything.tgz"),
            "archive name",
        )

    def test_wrong_vsix_name_is_rejected(self) -> None:
        self.reject(
            lambda record: record["vsix"].update(name="anything.vsix"),
            "vsix name",
        )

    def test_symlink_candidate_file_is_rejected(self) -> None:
        target = self.candidate / "real-archive"
        target.write_bytes(self.archive.read_bytes())
        self.archive.unlink()
        self.archive.symlink_to(target)
        with self.assertRaises(acceptance.AcceptanceError):
            acceptance.check(self.evidence, self.candidate)

    def test_wrong_platform_is_rejected(self) -> None:
        self.reject(lambda record: record.update(platform="darwin"), "platform")

    def test_wrong_architecture_is_rejected(self) -> None:
        self.reject(lambda record: record.update(architecture="arm64"), "architecture")

    def test_old_glibc_is_rejected(self) -> None:
        self.reject(lambda record: record.update(glibc="2.35"), "glibc")

    def test_wrong_toolchain_tag_is_rejected(self) -> None:
        self.reject(lambda record: record["toolchain"].update(tag="v16.2-rc25"), "tag")

    def test_wrong_toolchain_hash_is_rejected(self) -> None:
        self.reject(lambda record: record["toolchain"].update(sha256="0" * 64), "hash")

    def test_wrong_toolchain_size_is_rejected(self) -> None:
        self.reject(lambda record: record["toolchain"].update(bytes=1), "size")

    def test_old_fs_uae_is_rejected(self) -> None:
        self.reject(lambda record: record.update(fs_uae="3.1.66"), "fs uae")

    def test_wrong_hello_output_is_rejected(self) -> None:
        self.reject(
            lambda record: record["guest"].update(output="wrong", output_sha256=digest(b"wrong")),
            "hello",
        )

    def test_wrong_hello_hash_is_rejected(self) -> None:
        self.reject(
            lambda record: record["guest"].update(output_sha256="0" * 64),
            "hello hash",
        )

    def test_nonzero_guest_is_rejected(self) -> None:
        self.reject(lambda record: record["guest"].update(rc=1), "guest rc")

    def test_wrong_corpus_hash_is_rejected(self) -> None:
        self.reject(
            lambda record: record["corpus"][0].update(output_sha256="0" * 64),
            "corpus hash",
        )

    def test_wrong_corpus_profile_is_rejected(self) -> None:
        self.reject(
            lambda record: record["corpus"][0].update(profile="host"),
            "corpus profile",
        )

    def test_missing_corpus_entry_is_rejected(self) -> None:
        self.reject(lambda record: record["corpus"].pop(), "corpus count")

    def test_wrong_negative_is_rejected(self) -> None:
        self.reject(
            lambda record: record["negative"].update(status="ok", rc=0),
            "negative",
        )

    def test_missing_vsix_is_rejected(self) -> None:
        self.vsix.unlink()
        with self.assertRaises(acceptance.AcceptanceError):
            acceptance.check(self.evidence, self.candidate)

    def test_candidate_archive_mismatch_is_rejected(self) -> None:
        self.archive.write_bytes(self.archive.read_bytes() + b"x")
        with self.assertRaises(acceptance.AcceptanceError):
            acceptance.check(self.evidence, self.candidate)

    def test_installer_hash_mismatch_is_rejected(self) -> None:
        self.installer.write_bytes(b"different installer\n")
        with self.assertRaises(acceptance.AcceptanceError):
            acceptance.check(self.evidence, self.candidate)

    def test_missing_required_field_is_rejected(self) -> None:
        self.reject(lambda record: record.pop("guest"), "missing field")


if __name__ == "__main__":
    unittest.main()
