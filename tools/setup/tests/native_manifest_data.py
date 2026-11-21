"""Independent distribution records and deterministic archive test data.

Small structural records are deliberately not executable distributions. Tests
that exercise the producer instead supply the real miniature archives built by
the companion payload fixture.
"""

from __future__ import annotations

import hashlib
import io
import json
import tarfile
import zipfile
from pathlib import Path


DOWNLOAD_ROOT = "https://github.com/OpenLorraine/clorraine/releases/download/"
"""Publisher namespace used by distribution and component assets."""
API_ROOT = "https://api.github.com/repos/OpenLorraine/clorraine"
"""Read-only release lookup endpoint used by the installer."""
VERSION = "1.2.0"
"""Current development version used by the miniature core and editor inputs."""
RELEASE = "v1.2.0-r1"
"""Distribution revision used without changing the development core version."""
COMPONENTS = "fixture-core"
"""Immutable compiler/editor identity in local miniature inputs."""
SUPPORT = "fixture-support"
"""Immutable support identity in local miniature inputs."""
NOTICES = ("LICENSE", "LICENSE-EXCEPTION.md", "NOTICE", "TRADEMARKS.md",
           "THIRD_PARTY_NOTICES.md")
"""Required root notice paths in a core archive."""
PACKAGE_NAMES = ("amitools", "machine68k", "greenlet", "pysdl2", "pip")
"""Complete miniature runtime and bootstrap wheel set."""
BASE_PACKAGES = {"gcc", "libc6-dev", "python3", "python3-venv",
                 "curl", "ca-certificates"}
"""Unversioned system prerequisites added to each locked runtime package set."""


def digest(data: bytes) -> str:
    """Return the lowercase identity of the exact supplied bytes."""
    return hashlib.sha256(data).hexdigest()


def file_digest(path: Path) -> str:
    """Hash actual candidate inputs in fixed-size blocks rather than loading archives."""
    result = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            result.update(block)
    return result.hexdigest()


def encode(value: object) -> bytes:
    """Serialize canonical compact UTF-8 JSON with its terminating newline."""
    return (json.dumps(value, sort_keys=True, separators=(",", ":"),
                       ensure_ascii=False, allow_nan=False) + "\n").encode("utf-8")


def tar_bytes(files: dict[str, bytes], executables: tuple[str, ...] = (),
              extras: tuple[tuple[tarfile.TarInfo, bytes], ...] = ()) -> bytes:
    """Build a deterministic uncompressed PAX tar, including optional hostile data."""
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode="w", format=tarfile.PAX_FORMAT) as archive:
        for name, data in sorted(files.items()):
            member = tarfile.TarInfo(name)
            member.size = len(data)
            member.mode = 0o755 if name in executables else 0o644
            archive.addfile(member, io.BytesIO(data))
        for member, data in extras:
            archive.addfile(member, io.BytesIO(data) if member.isfile() else None)
    return output.getvalue()


def zip_bytes(files: dict[str, bytes]) -> bytes:
    """Build a deterministic ZIP with safe ordinary-file permissions."""
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_STORED) as archive:
        for name, data in sorted(files.items()):
            member = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            member.create_system = 3
            member.external_attr = (0o100644 << 16)
            archive.writestr(member, data)
    return output.getvalue()


def measure(data: bytes, kind: str) -> tuple[int, int]:
    """Count actual archive headers and expanded ordinary-file bytes independently."""
    if kind == "vsix":
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            entries = archive.infolist()
            return len(entries), sum(item.file_size for item in entries if not item.is_dir())
    if kind in ("core", "amiga", "preparation", "wheels-cp310", "wheels-cp312"):
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:") as archive:
            entries = archive.getmembers()
            return len(entries), sum(item.size for item in entries if item.isfile())
    return 0, 0


def asset_rows() -> list[tuple[str, str, str, str | None, str | None, str | None]]:
    """Return every required identity/kind/file/root/audit/release relationship."""
    c, s = COMPONENTS, SUPPORT
    rows = [
        (c + "-core", "core", "core.tar", ".", None, "components-" + c),
        (c + "-vsix", "vsix", "clorraine.vsix", "extension", None, "components-" + c),
        (s + "-lock", "support-lock", "support-lock.json", None, None, "support-" + s),
    ]
    for name, kind in (("amiga", "amiga"), ("preparation", "preparation"),
                       ("cp310", "wheels-cp310"), ("cp312", "wheels-cp312")):
        rows.append((s + "-" + name, kind, name + ".tar", ".",
                     s + "-" + name + "-audit", "support-" + s))
    for name in ("amiga", "preparation", "cp310", "cp312"):
        rows.append((s + "-" + name + "-audit", "support-audit",
                     name + "-audit.json", None, None, "support-" + s))
    rows.append((s + "-ndk", "ndk", "ndk", None, None, None))
    return rows


def make_manifest(raw: dict[str, bytes] | None = None, lock: dict | None = None) -> dict:
    """Construct the complete manifest without using a production schema builder.

    Omitting ``raw`` creates structural-only records. Supplying bytes measures
    the actual tar/ZIP inputs and binds the records to their exact hashes.
    """
    assets = []
    for identity, kind, filename, root, audit, namespace in asset_rows():
        data = raw[filename] if raw is not None else (filename + "\n").encode()
        count, expanded = measure(data, kind) if raw is not None else (
            (1, len(data)) if root is not None else (0, 0))
        url = (DOWNLOAD_ROOT + namespace + "/" + filename if namespace else
               "https://example.invalid/seed.tar")
        if kind == "ndk" and lock is not None:
            url = lock["sources"][lock["preparation"]["archive"]]["url"]
        assets.append(dict(identity=identity, kind=kind, url=url, bytes=len(data),
                           sha256=digest(data), root=root, entries=count,
                           expanded_bytes=expanded, audit=audit))
    inventory = lock["preparation"]["inventory"] if lock is not None else {
        "include/fixture-ndk.h": digest(b"Synthetic excluded NDK-derived header.\n")}
    runtime = lock["output"]["runtime_packages"] if lock is not None else {
        "22.04": {"dash": {}}, "24.04": {"dash": {}}}
    return {
        "schema": 1, "release": RELEASE, "core_version": VERSION,
        "extension_version": VERSION,
        "platform": {"os": "linux", "architecture": "x86_64",
                     "cohorts": {"22.04": "cp310", "24.04": "cp312"}},
        "assets": assets,
        "support": {
            "identity": SUPPORT, "lock": SUPPORT + "-lock",
            "amiga": SUPPORT + "-amiga", "preparation": SUPPORT + "-preparation",
            "wheels": {"cp310": SUPPORT + "-cp310", "cp312": SUPPORT + "-cp312"},
            "ndk": SUPPORT + "-ndk", "profiles": ["hosted-amiga", "hosted-amiga-13"],
            "prepared_inventory_sha256": digest(encode(inventory))},
        "prerequisites": {cohort: sorted(set(packages) | BASE_PACKAGES)
                          for cohort, packages in runtime.items()},
    }


def release_response(tag: str = RELEASE, *, draft: bool = False,
                     prerelease: bool = False, published: str = "2026-11-19T00:00:00Z") -> dict:
    """Describe an external GitHub release without granting manifest authority."""
    return {"tag_name": tag, "draft": draft, "prerelease": prerelease,
            "published_at": published,
            "assets": [{"name": "manifest.json",
                        "browser_download_url": DOWNLOAD_ROOT + tag + "/manifest.json"}]}
