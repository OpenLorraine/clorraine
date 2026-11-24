"""Validate a local Clorraine acceptance-evidence record.

The acceptance run writes a JSON record beside the raw output. This module checks
the record against the exact candidate files and the repository's expected
corpus bytes. It never downloads, builds, or runs anything. It proves the
record's format and the candidate/corpus bytes, not that the recorded run
happened; retain the raw acceptance directory and treat publication as a
separate approval.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

HELLO = "Hello, Amiga, from Clorraine!"
VERSION = "1.2.0"
ARCHIVE_NAME = f"clorraine-{VERSION}-x86_64-linux.tar.gz"
VSIX_NAME = f"clorraine-{VERSION}.vsix"
CORPUS = [
    ("file-copier", "hosted-amiga"),
    ("string-parser", "hosted-amiga"),
    ("buffer-container", "hosted-amiga"),
    ("volatile-mmio", "hosted-amiga"),
    ("file-copier", "hosted-amiga-13"),
]
TOOLCHAIN_TAG = "v16.2-rc24"
TOOLCHAIN_BYTES = 69_868_548
TOOLCHAIN_SHA256 = "b93b070ac47d2f01d126f57c03a8405f71fb470c8397b0bda51041217d25c100"
MINIMUM_GLIBC = (2, 38)
MINIMUM_FS_UAE = (3, 2, 0)
HASH = re.compile(r"[0-9a-f]{64}")

TOP_KEYS = {
    "version", "platform", "architecture", "glibc", "archive", "installer",
    "vsix", "toolchain", "kickstart", "fs_uae", "guest", "negative", "corpus",
    "timestamp",
}
ARCHIVE_KEYS = {"name", "sha256", "bytes"}
VSIX_KEYS = ARCHIVE_KEYS
INSTALLER_KEYS = {"sha256"}
TOOLCHAIN_KEYS = {"tag", "sha256", "bytes"}
KICKSTART_KEYS = {"name", "sha256"}
GUEST_KEYS = {"status", "rc", "output", "output_sha256"}
NEGATIVE_KEYS = {"status", "rc"}
CORPUS_KEYS = {"name", "profile", "status", "output_sha256"}


class AcceptanceError(ValueError):
    """A record does not prove the expected candidate run."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise AcceptanceError(message)


def _keys(value: object, expected: set[str], context: str) -> dict:
    _require(isinstance(value, dict), f"{context} must be an object")
    assert isinstance(value, dict)
    missing = expected - set(value)
    extra = set(value) - expected
    _require(not missing, f"{context} is missing {sorted(missing)}")
    _require(not extra, f"{context} has unknown keys {sorted(extra)}")
    return value


def _text(value: object, context: str) -> str:
    _require(isinstance(value, str) and value != "", f"{context} must be a nonempty string")
    assert isinstance(value, str)
    return value


def _integer(value: object, context: str) -> int:
    _require(isinstance(value, int) and not isinstance(value, bool),
             f"{context} must be an integer")
    assert isinstance(value, int)
    return value


def _digest(value: object, context: str) -> str:
    text = _text(value, context)
    _require(HASH.fullmatch(text) is not None, f"{context} must be a lowercase SHA-256")
    return text


def _basename(value: object, context: str) -> str:
    name = _text(value, context)
    _require("/" not in name and name not in {".", ".."}, f"{context} must be a basename")
    return name


def sha256_file(path: Path) -> str:
    """Hashes a file without loading it all into memory."""
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def normalized_corpus_sha(path: Path) -> str:
    """Hashes corpus bytes after removing carriage returns."""
    return hashlib.sha256(path.read_bytes().replace(b"\r", b"")).hexdigest()


def _candidate_file(candidate: Path, name: str, context: str) -> Path:
    path = candidate / name
    _require(not path.is_symlink(), f"{context} must not be a symlink: {path}")
    _require(path.is_file(), f"{context} is missing {path}")
    return path


def _check_candidate_archive(record: dict, candidate: Path, key: str) -> None:
    entry = _keys(record[key], ARCHIVE_KEYS if key != "installer" else INSTALLER_KEYS,
                  key)
    name = _basename(entry["name"], f"{key}.name") if "name" in entry else "install.sh"
    path = _candidate_file(candidate, name, key)
    if "bytes" in entry:
        _require(path.stat().st_size == _integer(entry["bytes"], f"{key}.bytes"),
                 f"{key} byte count does not match {path}")
    _require(sha256_file(path) == _digest(entry["sha256"], f"{key}.sha256"),
             f"{key} digest does not match {path}")


def _check_glibc(value: object) -> None:
    text = _text(value, "glibc")
    match = re.match(r"(\d+)\.(\d+)", text)
    _require(match is not None, f"glibc has no major.minor version: {text!r}")
    assert match is not None
    version = (int(match.group(1)), int(match.group(2)))
    _require(version >= MINIMUM_GLIBC, f"glibc {text} is below 2.38")


def _check_fs_uae(value: object) -> None:
    text = _text(value, "fs_uae")
    match = re.match(r"(\d+)\.(\d+)(?:\.(\d+))?", text)
    _require(match is not None, f"fs_uae has no version: {text!r}")
    assert match is not None
    version = (int(match.group(1)), int(match.group(2)),
               int(match.group(3) or 0))
    _require(version >= MINIMUM_FS_UAE, f"FS-UAE {text} is older than 3.2.0")


def _check_toolchain(value: object) -> None:
    entry = _keys(value, TOOLCHAIN_KEYS, "toolchain")
    _require(_text(entry["tag"], "toolchain.tag") == TOOLCHAIN_TAG,
             f"toolchain tag must be {TOOLCHAIN_TAG}")
    _require(_integer(entry["bytes"], "toolchain.bytes") == TOOLCHAIN_BYTES,
             f"toolchain must be {TOOLCHAIN_BYTES} bytes")
    _require(_digest(entry["sha256"], "toolchain.sha256") == TOOLCHAIN_SHA256,
             "toolchain SHA-256 does not match the pinned rc24 archive")


def _check_guest(value: object) -> None:
    entry = _keys(value, GUEST_KEYS, "guest")
    _require(_text(entry["status"], "guest.status") == "ok",
             "guest status must be ok")
    _require(_integer(entry["rc"], "guest.rc") == 0, "guest rc must be zero")
    output = _text(entry["output"], "guest.output")
    _require(output == HELLO, "guest output is not the expected hello line")
    _require(_digest(entry["output_sha256"], "guest.output_sha256") ==
             hashlib.sha256(HELLO.encode()).hexdigest(),
             "guest output hash does not match the hello line")


def _check_negative(value: object) -> None:
    entry = _keys(value, NEGATIVE_KEYS, "negative")
    _require(_text(entry["status"], "negative.status") == "failed",
             "negative status must be failed")
    _require(_integer(entry["rc"], "negative.rc") != 0,
             "negative rc must be nonzero")


def _check_corpus(value: object, repo: Path) -> None:
    _require(isinstance(value, list), "corpus must be an array")
    assert isinstance(value, list)
    _require(len(value) == len(CORPUS), f"corpus must contain {len(CORPUS)} entries")
    seen: set[tuple[str, str]] = set()
    for index, raw in enumerate(value):
        entry = _keys(raw, CORPUS_KEYS, f"corpus[{index}]")
        name = _text(entry["name"], f"corpus[{index}].name")
        profile = _text(entry["profile"], f"corpus[{index}].profile")
        pair = (name, profile)
        _require(pair in CORPUS, f"corpus[{index}] has an unexpected program/profile")
        _require(pair not in seen, f"corpus[{index}] duplicates {pair}")
        seen.add(pair)
        _require(_text(entry["status"], f"corpus[{index}].status") == "ok",
                 f"corpus[{index}] status must be ok")
        expected = repo / "spec/tests/corpus" / name / "expected-output.txt"
        _require(expected.is_file(), f"missing expected output {expected}")
        _require(_digest(entry["output_sha256"], f"corpus[{index}].output_sha256") ==
                 normalized_corpus_sha(expected),
                 f"corpus[{index}] output hash does not match {expected}")
    _require(seen == set(CORPUS), "corpus entries do not match the required set")


def check(record: object, candidate: Path, repo: Path | None = None) -> None:
    """Raises AcceptanceError unless the record proves the expected run."""
    repo = repo or Path(__file__).resolve().parents[2]
    candidate = Path(candidate)
    entry = _keys(record, TOP_KEYS, "record")
    _require(_text(entry["version"], "version") == VERSION,
             f"version must be {VERSION}")
    _require(_text(entry["platform"], "platform") == "linux",
             "platform must be linux")
    _require(_text(entry["architecture"], "architecture") == "x86_64",
             "architecture must be x86_64")
    _check_glibc(entry["glibc"])
    _check_candidate_archive(entry, candidate, "archive")
    _check_candidate_archive(entry, candidate, "vsix")
    _require(entry["archive"]["name"] == ARCHIVE_NAME,
             f"archive name must be {ARCHIVE_NAME}")
    _require(entry["vsix"]["name"] == VSIX_NAME,
             f"VSIX name must be {VSIX_NAME}")
    _check_candidate_archive(entry, candidate, "installer")
    _check_toolchain(entry["toolchain"])
    kickstart = _keys(entry["kickstart"], KICKSTART_KEYS, "kickstart")
    _basename(kickstart["name"], "kickstart.name")
    _digest(kickstart["sha256"], "kickstart.sha256")
    _check_fs_uae(entry["fs_uae"])
    _check_guest(entry["guest"])
    _check_negative(entry["negative"])
    _check_corpus(entry["corpus"], repo)
    _text(entry["timestamp"], "timestamp")


def main(argv: list[str] | None = None) -> int:
    """Runs the checker from `make acceptance-check`."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", required=True)
    parser.add_argument("--candidate", required=True)
    parser.add_argument("--repo", default=None)
    arguments = parser.parse_args(argv)
    try:
        record = json.loads(Path(arguments.evidence).read_text(encoding="utf-8"))
        check(record, Path(arguments.candidate),
              Path(arguments.repo) if arguments.repo else None)
    except (AcceptanceError, OSError, json.JSONDecodeError) as error:
        print(f"acceptance-check: {error}", file=sys.stderr)
        return 1
    print("acceptance-check: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
