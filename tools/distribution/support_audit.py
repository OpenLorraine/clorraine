"""Validate closed payload evidence and compare it with actual filesystem/native bytes."""

from __future__ import annotations

import json
import os
from pathlib import Path, PurePosixPath
import posixpath
import re
import stat
import zipfile

from support import SupportError, validate_lock
from support_build import inventory, verify_wheels
from support_native import inspect_payload, safe_relative


def _fail(identity: str, message: str) -> None:
    """Keep a rejected obligation tied to its source, member or field."""
    raise SupportError(f"{identity}: {message}")


def _fields(value: object, fields: set[str], identity: str) -> dict:
    """Require the exact record shape before reading its values."""
    if not isinstance(value, dict) or any(not isinstance(key, str) for key in value):
        _fail(identity, "expected an object")
    if value.keys() != fields:
        _fail(identity, f"missing/unknown fields: {sorted(value.keys() ^ fields)!r}")
    return value


def _token(value: object, pattern: str, identity: str) -> str:
    """Validate a small metadata grammar without coercing numbers or booleans."""
    if not isinstance(value, str) or re.fullmatch(pattern, value) is None:
        _fail(identity, f"invalid value {value!r}")
    return value


def _paths(value: object, identity: str, *, nonempty: bool = False) -> list[str]:
    """Validate unique canonical relative obligation paths."""
    if not isinstance(value, list) or (nonempty and not value):
        _fail(identity, "expected an array" + (" with at least one path" if nonempty else ""))
    paths = [safe_relative(item, identity) for item in value]
    if len(paths) != len(set(paths)):
        _fail(identity, "duplicate paths")
    return paths


def load_audit(path: Path) -> dict:
    """Read bounded ordinary UTF-8 JSON, rejecting duplicate keys at every depth."""
    def unique(pairs):
        """Never let a repeated origin/hash silently replace earlier evidence."""
        result = {}
        for key, value in pairs:
            if key in result:
                _fail(str(path), f"duplicate JSON key {key!r}")
            result[key] = value
        return result

    descriptor = os.open(path, os.O_RDONLY | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            _fail(str(path), "audit must be an ordinary file")
        data = stream.read(16 * 1024 * 1024 + 1)
    if len(data) > 16 * 1024 * 1024:
        _fail(str(path), "audit exceeds 16 MiB")
    try:
        return json.loads(data.decode("utf-8"), object_pairs_hook=unique)
    except (ValueError, RecursionError) as error:
        raise SupportError(f"{path}: invalid audit JSON: {error}") from error


def _native_key(value: object) -> str:
    """Validate either a payload name or a single wheel/member native identity."""
    if not isinstance(value, str) or value.count("!") > 1:
        _fail(repr(value), "invalid native identity")
    for side in value.split("!"):
        safe_relative(side, value)
    return value


def _native_record(value: object, identity: str) -> dict:
    """Validate declared native fields before comparing them with measured content."""
    record = _fields(value, {"format", "machine", "soname", "needed", "glibc"}, identity)
    if not isinstance(record["format"], str) or record["format"] not in {"elf", "ar", "amiga-object"}:
        _fail(identity, "unsupported native format")
    if not isinstance(record["machine"], str) or record["machine"] not in {"x86_64", "m68k"}:
        _fail(identity, "unsupported native architecture")
    if record["soname"] is not None:
        _token(record["soname"], r"[A-Za-z0-9_+.-]+", identity)
    if not isinstance(record["needed"], dict):
        _fail(identity, "needed must be a provider map")
    for soname, provider in record["needed"].items():
        _token(soname, r"[A-Za-z0-9_+.-]+", identity)
        if not isinstance(provider, str):
            _fail(identity, "provider must be text")
        if provider.startswith("package:"):
            _token(provider[8:], r"[a-z0-9][a-z0-9+.-]+", identity)
        elif provider.startswith("file:"):
            _native_key(provider[5:])
        else:
            _fail(identity, f"unknown provider {provider!r}")
    if not isinstance(record["glibc"], list):
        _fail(identity, "glibc must be an array")
    for version in record["glibc"]:
        _token(version, r"[0-9]+(?:\.[0-9]+)+", identity)
    if len(set(record["glibc"])) != len(record["glibc"]):
        _fail(identity, "duplicate GLIBC versions")
    return record


def _rpaths(root: Path, name: str, paths: list[str], lock: dict) -> None:
    """Require loader paths to be internal or supplied by both declared system cohorts."""
    for entry in paths:
        for value in entry.split(":"):
            if not value or any(prefix in value for prefix in lock["output"]["builder_prefixes"]):
                _fail(name, "executable builder-prefix or empty loader path")
            if value == "$ORIGIN" or value.startswith("$ORIGIN/"):
                suffix = value[len("$ORIGIN"):].lstrip("/")
                if "!" in name:
                    wheel, member = name.split("!")
                    target = posixpath.normpath(posixpath.join(posixpath.dirname(member), suffix))
                    if target.startswith("../") or target.startswith("/"):
                        _fail(name, "escaping wheel loader path")
                    # A wheel's installed directory is represented by its member
                    # namespace, not by the temporary inspection directory.
                    try:
                        with zipfile.ZipFile(root / wheel) as archive:
                            if target != "." and not any(
                                    key.startswith(target.rstrip("/") + "/")
                                    for key in archive.namelist()):
                                _fail(name, "unresolved internal wheel loader directory")
                    except zipfile.BadZipFile as error:
                        raise SupportError(f"{name}: wheel changed during inspection") from error
                else:
                    target = ((root / name).parent / suffix).resolve(strict=True)
                    if not target.is_dir() or (target != root and root not in target.parents):
                        _fail(name, "escaping or non-directory loader path")
            elif value.startswith("/"):
                for cohort in lock["output"]["runtime_packages"].values():
                    if not any(str(PurePosixPath(path).parent) == value
                               for package in cohort.values() for path in package["paths"]):
                        _fail(name, f"undeclared system loader directory {value!r}")
            else:
                _fail(name, "relative loader path depends on the caller's working directory")


def audit_payload(lock: dict, lock_digest: str, root: Path, evidence: object) -> dict:
    """Audit complete files, obligations and actual native content; return accepted file records.

    This proves evidence consistency, not a new licence grant. Component
    origin tracing and legal review remain separate required release work.
    The caller must only publish bytes that still match these accepted hashes.
    """
    validate_lock(lock)
    root = root.resolve(strict=True)
    audit = _fields(evidence, {"schema", "lock_sha256", "components", "files", "native"}, "audit")
    if type(audit["schema"]) is not int or audit["schema"] != 1:
        _fail("schema", "unsupported audit schema")
    if audit["lock_sha256"] != lock_digest:
        _fail("lock_sha256", "audit does not match the exact lock bytes")
    for field in ("components", "files", "native"):
        if not isinstance(audit[field], dict):
            _fail(field, "expected a map")
    files = audit["files"]
    try:
        actual = inventory(root)
    except UnicodeError as error:
        raise SupportError(f"{error.object!r}: invalid UTF-8 payload path") from error
    for name in sorted(actual.keys() | files.keys()):
        safe_relative(name, name)
        if name not in actual or name not in files:
            _fail(name, "payload file/link coverage mismatch")
        record = files[name]
        if not isinstance(record, dict):
            _fail(name, "expected a file record")
        kind = record.get("type")
        if not isinstance(kind, str) or kind not in {"file", "symlink"}:
            _fail(name, "unknown file type")
        field = "sha256" if kind == "file" else "target"
        _fields(record, {"type", field, "origins", "role"}, name)
        if actual[name] != {"type": kind, field: record[field]}:
            _fail(name, "payload type/hash/link target mismatch")
        origins = record["origins"]
        if not isinstance(origins, list) or not origins or any(
                not isinstance(origin, str) or origin not in lock["sources"] for origin in origins):
            _fail(name, "missing or unknown locked source origin")
        if len(origins) != len(set(origins)):
            _fail(name, "duplicate origins")
        if any(origin not in audit["components"] for origin in origins):
            _fail(name, "missing component obligations")
        if not isinstance(record["role"], str):
            _fail(name, "role must be text")
        if kind == "symlink":
            if record["role"] != "link":
                _fail(name, "link must have the link role")
        elif record["role"] not in {
                "program", "header", "library", "wheel", "preparation",
                "notice", "review", "source"}:
            _fail(name, "unknown regular-file role")
    for origin, component in audit["components"].items():
        if origin not in lock["sources"]:
            _fail(origin, "unknown component source")
        _fields(component, {"review", "notices", "source"}, origin)
        review = safe_relative(component["review"], origin)
        notices = _paths(component["notices"], origin, nonempty=True)
        sources = _paths(component["source"], origin)
        terms = lock["sources"][origin]["licence"]
        if terms["source_required"] and not sources:
            _fail(origin, "required corresponding source is absent")
        for name, role in [(review, "review"), *[(name, "notice") for name in notices],
                           *[(name, "source") for name in sources]]:
            record = files.get(name)
            if (record is None or record["type"] != "file"
                    or record["role"] != role or origin not in record["origins"]):
                _fail(name, f"missing matching {role} obligation for {origin}")
        if files[review]["sha256"] != lock["inputs"][terms["review"]]:
            _fail(review, f"review bytes do not match the locked input for {origin}")
    forbidden = set(lock["output"]["forbidden_sha256"])
    archive = lock["preparation"]["archive"]
    forbidden.add(lock["sources"][archive]["sha256"])
    excluded_programs = {
        "fs-uae", "gdb", "vbcc", "vasm", "vlink", "cc1plus", "cc1obj",
        "m68k-amigaos-gdb", "m68k-amigaos-gdb-add-index", "m68k-amigaos-gstack",
        "m68k-amigaos-c++", "m68k-amigaos-g++", "m68k-amigaos-lto-dump",
        "m68k-amigaos-gcov", "m68k-amigaos-gcov-dump", "m68k-amigaos-gcov-tool",
    }
    for name, record in files.items():
        if record["type"] != "file":
            continue
        if record["sha256"] in forbidden:
            _fail(name, "known excluded or NDK archive bytes")
        if archive in record["origins"] and record["role"] not in {"notice", "review"}:
            _fail(name, "NDK-origin content is not redistributable support")
        if Path(name).suffix.lower() in {".rom", ".adf", ".hdf"}:
            _fail(name, "licensed-media payloads are excluded")
        if record["role"] == "program" and Path(name).name in excluded_programs:
            _fail(name, "unneeded SDK or emulator program")
    for name, record in audit["native"].items():
        _native_key(name)
        _native_record(record, name)
    if any(record["role"] == "wheel" for record in files.values()):
        manifest = load_audit(root / "wheels.json")
        if not isinstance(manifest, dict) or manifest.get("abi") not in ("cp310", "cp312"):
            _fail("wheels.json", "missing or unsupported wheel ABI")
        verify_wheels(root, lock, lock_digest, manifest["abi"], files)
        for name, record in manifest["packages"].items():
            if record["source"] not in files[record["filename"]]["origins"]:
                _fail(name, "wheel source differs from audited origin")
    measured = inspect_payload(root, files, lock["output"]["builder_prefixes"], audit["native"])
    for name in sorted(measured.keys() | audit["native"].keys()):
        if name not in measured or name not in audit["native"]:
            _fail(name, "native record coverage mismatch")
        expected, observed = audit["native"][name], measured[name]
        for field in ("format", "machine", "soname"):
            if expected[field] != observed[field]:
                _fail(name, f"actual native {field} differs from its record")
        if set(expected["glibc"]) != set(observed["glibc"]):
            _fail(name, "actual GLIBC requirements differ from their record")
        if set(expected["needed"]) != set(observed["needed"]):
            _fail(name, "actual NEEDED closure differs from its record")
        _rpaths(root, name, observed["rpaths"], lock)
        for soname, provider in expected["needed"].items():
            if provider.startswith("package:"):
                package = provider[8:]
                for cohort in lock["output"]["runtime_packages"].values():
                    if package not in cohort or soname not in cohort[package]["sonames"]:
                        _fail(name, f"missing declared package provider for {soname}")
            else:
                target = provider[5:]
                if target not in measured and "!" not in target and target in files:
                    target = (root / target).resolve(strict=True).relative_to(root).as_posix()
                if target not in measured:
                    _fail(name, f"missing native bundled provider {target!r}")
                actual_soname = measured[target]["soname"]
                if actual_soname != soname and not (
                        actual_soname is None and target.rsplit("/", 1)[-1] == soname):
                    _fail(name, f"bundled provider has a different SONAME: {target}")
    return files
