"""Validate a local Clorraine acceptance-evidence record.

The acceptance run writes a JSON record beside the raw output. This module checks
the record against the exact candidate files and the repository's expected
corpus bytes. Records are platform-keyed: a shared core (including the VS Code
extension the installer took from the Marketplace and the in-person editor
confirmation) plus the facts Linux (glibc) or Windows (build, rc24 setup,
FS-UAE source, PC hello, standard account, the UAC confirmation) adds. The
version is the candidate's. It never downloads, builds, or runs anything. It proves the
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
ARCHIVE_PATTERN = re.compile(r"clorraine-(.+)-x86_64-(?:linux\.tar\.gz|windows\.zip)")
CORPUS = [
    ("file-copier", "hosted-amiga"),
    ("string-parser", "hosted-amiga"),
    ("buffer-container", "hosted-amiga"),
    ("volatile-mmio", "hosted-amiga"),
    ("file-copier", "hosted-amiga-13"),
]
TOOLCHAIN_TAG = "v16.2-rc24"
# The rc24 pin per platform: the Linux archive and the Windows setup.
TOOLCHAIN_PINS = {
    "linux": (69_868_548, "b93b070ac47d2f01d126f57c03a8405f71fb470c8397b0bda51041217d25c100",
              "the pinned rc24 archive"),
    "windows": (48_065_003, "9dd6973539bb2d514fc1b1e6c0a107cac6f30a9c4076cf4014cbe13013c95512",
                "the pinned rc24 Windows setup"),
}
FS_UAE_DOWNLOAD_VERSION = "3.2.35"
FS_UAE_ZIP_BYTES = 49_123_213
FS_UAE_ZIP_SHA256 = "b402e2802d518a085c878be96c5c0a656fa3dec57dcf0019d226f3285069063e"
MINIMUM_GLIBC = (2, 38)
MINIMUM_FS_UAE = (3, 2, 0)
MINIMUM_WINDOWS_BUILD = 22_000
HASH = re.compile(r"[0-9a-f]{64}")

CORE_KEYS = {
    "version", "platform", "architecture", "archive", "installer",
    "extension", "toolchain", "kickstart", "fs_uae", "guest", "negative", "corpus",
    "timestamp", "manual",
}
TOP_KEYS = {
    "linux": CORE_KEYS | {"glibc"},
    "windows": CORE_KEYS | {"os_build", "host_hello", "existing_registration",
                            "standard_account"},
}
ARCHIVE_SUFFIX = {"linux": "x86_64-linux.tar.gz", "windows": "x86_64-windows.zip"}
INSTALLER_NAME = {"linux": "install.sh", "windows": "install.ps1"}
ARCHIVE_KEYS = {"name", "sha256", "bytes"}
# The VS Code extension comes from the Marketplace, so a record names what the
# installer installed and where it was published from, not candidate bytes.
EXTENSION_ID = "openlorraine.clorraine"
EXTENSION_KEYS = {"id", "version", "publish_run", "vsix_sha256"}
INSTALLER_KEYS = {"linux": {"sha256"}, "windows": {"name", "sha256"}}
HOST_HELLO_KEYS = {"compiler", "result"}
# The in-person confirmations: the editor's language server and Build/Run on
# both platforms, and the UAC prompt on Windows.
MANUAL_KEYS = {"linux": {"editor_build_run"}, "windows": {"uac_prompt_seen", "editor_build_run"}}
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


def candidate_version(candidate: Path) -> str:
    """The one version named by the candidate's Linux and Windows archives."""
    versions = set()
    if candidate.is_dir():
        for path in candidate.iterdir():
            match = ARCHIVE_PATTERN.fullmatch(path.name)
            if match:
                versions.add(match.group(1))
    _require(len(versions) == 1,
             f"version: the candidate must name exactly one archive version, found {sorted(versions)}")
    return versions.pop()


def _check_candidate_archive(record: dict, candidate: Path, key: str, platform: str) -> None:
    keys = INSTALLER_KEYS[platform] if key == "installer" else ARCHIVE_KEYS
    entry = _keys(record[key], keys, key)
    name = (_basename(entry["name"], f"{key}.name") if "name" in entry
            else INSTALLER_NAME[platform])
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


def _check_fs_uae_version(value: object, context: str) -> None:
    text = _text(value, context)
    match = re.match(r"(\d+)\.(\d+)(?:\.(\d+))?", text)
    _require(match is not None, f"{context} has no version: {text!r}")
    assert match is not None
    version = (int(match.group(1)), int(match.group(2)),
               int(match.group(3) or 0))
    _require(version >= MINIMUM_FS_UAE, f"FS-UAE {text} is older than 3.2.0")


def _check_fs_uae(value: object) -> None:
    _check_fs_uae_version(value, "fs_uae")


def _check_windows_fs_uae(value: object) -> None:
    """A Windows record says whether the installer downloaded FS-UAE (then
    with the pinned zip) or used one the person already had."""
    _require(isinstance(value, dict), "fs_uae must be an object")
    assert isinstance(value, dict)
    source = value.get("source")
    _require(source in {"downloaded", "existing"},
             "fs_uae.source must be downloaded or existing")
    keys = {"version", "source"} | ({"sha256", "bytes"} if source == "downloaded" else set())
    entry = _keys(value, keys, "fs_uae")
    _check_fs_uae_version(entry["version"], "fs_uae.version")
    if source == "downloaded":
        _require(entry["version"] == FS_UAE_DOWNLOAD_VERSION,
                 f"fs_uae.version must be {FS_UAE_DOWNLOAD_VERSION} for the pinned download")
        _require(_integer(entry["bytes"], "fs_uae.bytes") == FS_UAE_ZIP_BYTES,
                 f"fs_uae download must be {FS_UAE_ZIP_BYTES} bytes")
        _require(_digest(entry["sha256"], "fs_uae.sha256") == FS_UAE_ZIP_SHA256,
                 "fs_uae SHA-256 does not match the pinned FS-UAE zip")


def _check_toolchain(value: object, platform: str) -> None:
    size, sha256, label = TOOLCHAIN_PINS[platform]
    entry = _keys(value, TOOLCHAIN_KEYS, "toolchain")
    _require(_text(entry["tag"], "toolchain.tag") == TOOLCHAIN_TAG,
             f"toolchain tag must be {TOOLCHAIN_TAG}")
    _require(_integer(entry["bytes"], "toolchain.bytes") == size,
             f"toolchain must be {size} bytes")
    _require(_digest(entry["sha256"], "toolchain.sha256") == sha256,
             f"toolchain SHA-256 does not match {label}")


def _check_windows_facts(entry: dict) -> None:
    """The Windows-only facts: build, PC hello, account, and confirmations."""
    build = _integer(entry["os_build"], "os_build")
    _require(build >= MINIMUM_WINDOWS_BUILD,
             f"os_build {build} is older than Windows 11 ({MINIMUM_WINDOWS_BUILD})")
    hello = _keys(entry["host_hello"], HOST_HELLO_KEYS, "host_hello")
    _require("gcc" in _text(hello["compiler"], "host_hello.compiler").lower(),
             "host_hello.compiler must be GCC")
    _require(_text(hello["result"], "host_hello.result") == "ok",
             "host_hello.result must be ok")
    _require(entry["existing_registration"] == "reused",
             "existing_registration must be reused")
    _require(entry["standard_account"] == "installed",
             "standard_account must be installed")


def _check_manual(value: object, platform: str) -> None:
    """Every in-person confirmation for the platform must be true."""
    manual = _keys(value, MANUAL_KEYS[platform], "manual")
    for key in sorted(MANUAL_KEYS[platform]):
        _require(manual[key] is True, f"manual.{key} must be true")


def _check_extension(value: object, version: str) -> None:
    """The installer installed this release's extension from the Marketplace."""
    entry = _keys(value, EXTENSION_KEYS, "extension")
    _require(_text(entry["id"], "extension.id") == EXTENSION_ID,
             f"extension id must be {EXTENSION_ID}")
    installed = _text(entry["version"], "extension.version")
    _require(installed == version,
             f"extension version {installed} does not match candidate {version}")
    _require(re.fullmatch(r"[0-9]+", _text(entry["publish_run"], "extension.publish_run")) is not None,
             "extension.publish_run must be a GitHub Actions run ID")
    _digest(entry["vsix_sha256"], "extension.vsix_sha256")


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
    _require(isinstance(record, dict), "record must be an object")
    assert isinstance(record, dict)
    platform = record.get("platform")
    _require(platform in TOP_KEYS, "platform must be linux or windows")
    assert isinstance(platform, str)
    entry = _keys(record, TOP_KEYS[platform], "record")
    version = candidate_version(candidate)
    _require(_text(entry["version"], "version") == version,
             f"version must be {version}, the candidate's archive version")
    _require(_text(entry["architecture"], "architecture") == "x86_64",
             "architecture must be x86_64")
    if platform == "linux":
        _check_glibc(entry["glibc"])
    _check_candidate_archive(entry, candidate, "archive", platform)
    archive_name = f"clorraine-{version}-{ARCHIVE_SUFFIX[platform]}"
    _require(entry["archive"]["name"] == archive_name,
             f"archive name must be {archive_name}")
    _check_extension(entry["extension"], version)
    _check_candidate_archive(entry, candidate, "installer", platform)
    if "name" in entry["installer"]:
        _require(entry["installer"]["name"] == INSTALLER_NAME[platform],
                 f"installer name must be {INSTALLER_NAME[platform]}")
    _check_toolchain(entry["toolchain"], platform)
    kickstart = _keys(entry["kickstart"], KICKSTART_KEYS, "kickstart")
    _basename(kickstart["name"], "kickstart.name")
    _digest(kickstart["sha256"], "kickstart.sha256")
    if platform == "linux":
        _check_fs_uae(entry["fs_uae"])
    else:
        _check_windows_fs_uae(entry["fs_uae"])
        _check_windows_facts(entry)
    _check_manual(entry["manual"], platform)
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
