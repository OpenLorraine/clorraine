#!/usr/bin/env python3
"""Prepare an inactive SDK using authenticated local data and prebuilt helpers only."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import subprocess
import sys
import tempfile
from typing import NoReturn


def _fail(identity: str, message: str) -> NoReturn:
    """Keep failures tied to the metadata field, archive member or local file."""
    raise ValueError(f"{identity}: {message}")


def _text(value: object, identity: str) -> str:
    """Require nonempty UTF-8 text without terminal or filename control characters."""
    if not isinstance(value, str) or not value:
        _fail(identity, "expected nonempty text")
    try:
        value.encode("utf-8")
    except UnicodeError as error:
        raise ValueError(f"{identity}: invalid UTF-8 text") from error
    if any(ord(char) < 32 or 127 <= ord(char) <= 159 for char in value):
        _fail(identity, "control characters are not permitted")
    return value


def _relative(value: object, identity: str) -> str:
    """Require a canonical relative resource/member path, never a host path."""
    value = _text(value, identity)
    if (value.startswith("/") or "\\" in value or "!" in value
            or any(part in ("", ".", "..") for part in value.split("/"))):
        _fail(identity, f"unsafe path {value!r}")
    return value


def _token(value: object, identity: str, pattern: str) -> str:
    """Apply the exact field grammar before interpreting metadata."""
    value = _text(value, identity)
    if re.fullmatch(pattern, value) is None:
        _fail(identity, f"invalid value {value!r}")
    return value


def _object(value: object, identity: str, fields: set[str] | None = None) -> dict:
    """Reject wrong record shapes, missing fields and unrecognised fields."""
    if not isinstance(value, dict) or any(not isinstance(key, str) for key in value):
        _fail(identity, "expected an object")
    if fields is not None and value.keys() != fields:
        _fail(identity, f"missing/unknown fields: {sorted(value.keys() ^ fields)!r}")
    return value


def validate_archive_members(members: object) -> None:
    """Validate a complete member listing without mutation or filesystem access.

    Implicit parent directories are allowed. Members cannot overwrite one
    another, traverse an alias parent, introduce special files or point outside
    the archive tree. Link targets must resolve against this complete listing.
    Raises ValueError naming the offending member.

    Example: validate_archive_members([
        {"path": "include/types.h", "type": "file", "target": None}])
    """
    if not isinstance(members, list):
        _fail("members", "expected an array")
    entries = {}
    for member in members:
        member = _object(member, "member", {"path", "type", "target"})
        path = _relative(member["path"], repr(member["path"]))
        kind = member["type"]
        if kind not in ("file", "directory", "symlink"):
            _fail(path, f"unsupported member type {kind!r}")
        if path in entries:
            _fail(path, "duplicate member destination")
        if kind == "symlink":
            target = _text(member["target"], path)
            if target.startswith("/") or "\\" in target:
                _fail(path, "absolute or alternate-separator link target")
        elif member["target"] is not None:
            _fail(path, "only a link can have a target")
        entries[path] = member
    implicit = {parent.as_posix() for name in entries
                for parent in PurePosixPath(name).parents if parent.as_posix() != "."}
    for parent in implicit:
        if parent in entries and entries[parent]["type"] != "directory":
            _fail(parent, "extraction through a non-directory or symlink parent")
    directories = implicit | {name for name, entry in entries.items()
                              if entry["type"] == "directory"} | {""}
    for name, entry in entries.items():
        if entry["type"] != "symlink":
            continue
        current = list(PurePosixPath(name).parts[:-1])
        pending = entry["target"].split("/")
        states = set()
        followed = 0
        while pending:
            state = (tuple(current), tuple(pending))
            if state in states:
                _fail(name, "cyclic link")
            states.add(state)
            part = pending.pop(0)
            if part in ("", "."):
                continue
            if part == "..":
                if not current:
                    _fail(name, "escaping link")
                current.pop()
                continue
            current.append(part)
            target = "/".join(current)
            node = entries.get(target)
            if node and node["type"] == "symlink":
                followed += 1
                # Linux cannot resolve more than forty aliases in one lookup.
                # This also bounds cycles whose pending path keeps growing.
                if followed > 40:
                    _fail(name, "cyclic or unresolvable link chain")
                current.pop()
                pending = node["target"].split("/") + pending
            elif target not in directories and (node is None or pending):
                _fail(name, f"dangling or non-directory link target {target!r}")


def _json(path: Path) -> dict:
    """Read bounded ordinary UTF-8 JSON while rejecting duplicate keys at every depth."""
    def unique(pairs):
        """A duplicate cannot silently replace a selected hash or helper."""
        result = {}
        for key, value in pairs:
            if key in result:
                _fail(str(path), f"duplicate JSON key {key!r}")
            result[key] = value
        return result

    descriptor = os.open(path, os.O_RDONLY | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            _fail(str(path), "metadata must be an ordinary file")
        content = stream.read(16 * 1024 * 1024 + 1)
    if len(content) > 16 * 1024 * 1024:
        _fail(str(path), "metadata exceeds 16 MiB")
    return json.loads(content.decode("utf-8"), object_pairs_hook=unique)


def _hash(path: Path) -> str:
    """Hash only ordinary files, without opening a FIFO or accepting a resource symlink."""
    if path.is_symlink():
        _fail(str(path), "expected an ordinary file, not a link")
    descriptor = os.open(path, os.O_RDONLY | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            _fail(str(path), "expected an ordinary file")
        digest = hashlib.sha256()
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _metadata(value: object) -> dict:
    """Validate the self-contained preparation record without invoking any helper."""
    root = _object(value, "preparation", {
        "schema", "lock_sha256", "archive", "base", "helpers", "inputs", "inventory", "notices"})
    if type(root["schema"]) is not int or root["schema"] != 1:
        _fail("schema", "unsupported schema")
    _token(root["lock_sha256"], "lock_sha256", r"[0-9a-f]{64}")
    archive = _object(root["archive"], "archive", {"source", "bytes", "sha256"})
    _token(archive["source"], "archive.source", r"[a-z0-9][a-z0-9._-]*")
    if type(archive["bytes"]) is not int or archive["bytes"] <= 0:
        _fail("archive.bytes", "expected a positive integer")
    _token(archive["sha256"], "archive.sha256", r"[0-9a-f]{64}")
    for field in ("base", "helpers", "inputs", "inventory"):
        _object(root[field], field)
    for name, record in root["base"].items():
        _relative(name, name)
        _object(record, name)
        if record.get("type") == "file":
            _object(record, name, {"type", "sha256"})
            _token(record["sha256"], name, r"[0-9a-f]{64}")
        elif record.get("type") == "symlink":
            _object(record, name, {"type", "target"})
            target = _text(record["target"], name)
            if target.startswith("/") or "\\" in target:
                _fail(name, "unsafe baseline link target")
        else:
            _fail(name, "unknown baseline file type")
    for name, helper in root["helpers"].items():
        _token(name, name, r"[a-z0-9][a-z0-9._-]*")
        _object(helper, name, {"path", "sha256"})
        _relative(helper["path"], helper["path"])
        _token(helper["sha256"], name, r"[0-9a-f]{64}")
    for field in ("inputs", "inventory"):
        for name, digest in root[field].items():
            _relative(name, name)
            _token(digest, name, r"[0-9a-f]{64}")
    if not root["inventory"]:
        _fail("inventory", "prepared inventory must not be empty")
    notices = root["notices"]
    if not isinstance(notices, list) or not notices:
        _fail("notices", "expected retained notice paths")
    for path in notices:
        _relative(path, "notices")
        if path not in root["inventory"]:
            _fail(path, "notice must belong to the prepared inventory")
    if len(set(notices)) != len(notices):
        _fail("notices", "duplicate notice path")
    return root


def _tree(root: Path) -> dict:
    """Inventory actual regular files and safe links; propagate unreadable subtrees."""
    records = {}

    def unreadable(error):
        """Unlisted contents cannot be treated as verified."""
        raise error

    for directory, folders, files in os.walk(root, followlinks=False, onerror=unreadable):
        if not folders and not files:
            _fail(str(Path(directory).relative_to(root)), "unexpected empty directory")
        for name in sorted(folders + files):
            path = Path(directory) / name
            relative = _relative(path.relative_to(root).as_posix(), repr(name))
            mode = path.lstat().st_mode
            if mode & 0o7000:
                _fail(relative, "special permissions are not permitted")
            if stat.S_ISLNK(mode):
                target = os.readlink(path)
                if target.startswith("/") or "\\" in target:
                    _fail(relative, "unsafe link")
                resolved = path.resolve(strict=True)
                if resolved != root and root not in resolved.parents:
                    _fail(relative, "escaping link")
                records[relative] = {"type": "symlink", "target": target}
            elif stat.S_ISREG(mode):
                records[relative] = {"type": "file", "sha256": _hash(path)}
            elif not stat.S_ISDIR(mode):
                _fail(relative, "special file")
    return records


def _compare(actual: dict, expected: dict, identity: str) -> None:
    """Name the first missing, extra or changed file instead of accepting a marker alone."""
    for name in sorted(actual.keys() | expected.keys()):
        if actual.get(name) != expected.get(name):
            _fail(name, f"{identity} inventory mismatch")


def _resource(bundle: Path, name: str, digest: str) -> Path:
    """Authenticate a bundle-relative file before it can affect execution or output."""
    path = bundle / name
    if not path.resolve(strict=True).is_relative_to(bundle) or _hash(path) != digest:
        _fail(name, "preparation resource hash/location mismatch")
    return path


def _run(helper: Path, arguments: list[str], cwd: Path) -> bytes:
    """Invoke only a preverified helper with literal arguments and a clean environment."""
    result = subprocess.run(
        [str(helper), *arguments], cwd=cwd, env={
            "PATH": "/usr/bin:/bin", "LC_ALL": "C", "TMPDIR": str(cwd)},
        capture_output=True, timeout=180, check=False)
    if result.returncode:
        _fail(helper.name, f"helper failed (exit {result.returncode})")
    return result.stdout


def _extract(archive: Path, lha: Path, work: Path) -> Path:
    """Validate the complete fixed-format LHA listing before extracting selected data."""
    report = _run(lha, ["vq", str(archive)], work).decode("utf-8")
    pattern = re.compile(
        r"^\[unknown\]\s+(\d+)\s+(\d+)\s+(\d+\.\d+%|\*{6})\s+(-lh[05d]-)\s+"
        r"[0-9a-f]{4}\s+[A-Z][a-z]{2}\s+\d{1,2}\s+(?:\d{4}|\d{2}:\d{2})\s+(.+)$")
    members = []
    for line in report.splitlines():
        match = pattern.fullmatch(line)
        if match is None:
            _fail("lha", f"unsupported or ambiguous listing record {line!r}")
        packed, size, ratio, method, name = match.groups()
        # LHA prints stars instead of a compression ratio for an empty file.
        # They must not conceal an unparseable nonempty member.
        if ratio == "******" and (int(packed) != 0 or int(size) != 0):
            _fail(name, "invalid empty-member ratio")
        members.append({"path": name, "type": "directory" if method == "-lhd-" else "file",
                        "target": None})
    if not members:
        _fail("lha", "empty archive listing")
    validate_archive_members(members)
    allowed = [record["path"] for record in members if record["type"] == "file"
               and record["path"].startswith(("Include_H/", "Include_I/", "FD/", "SFD/", "lib/"))]
    if not allowed:
        _fail("lha", "required NDK data is absent")
    extracted = work / "archive"
    extracted.mkdir()
    _run(lha, ["xfq", str(archive), *allowed], extracted)
    actual = _tree(extracted)
    _compare({name: record["type"] for name, record in actual.items()},
             {name: "file" for name in allowed}, "extracted archive")
    return extracted


def _patch(path: Path, patch: Path) -> None:
    """Apply reviewed unified hunks at exact offsets, with no fuzzy context matching."""
    original = path.read_bytes().splitlines(keepends=True)
    lines = patch.read_bytes().splitlines(keepends=True)
    if len(lines) < 3 or not lines[0].startswith(b"--- ") or not lines[1].startswith(b"+++ "):
        _fail(patch.name, "expected one unified file patch")
    output, cursor, index = [], 0, 2
    while index < len(lines):
        match = re.fullmatch(rb"@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@[^\n]*\n", lines[index])
        if match is None:
            _fail(patch.name, "unsupported patch hunk")
        start = int(match[1]) - 1
        old_count = int(match[2] or b"1")
        new_count = int(match[4] or b"1")
        if start < cursor or start > len(original):
            _fail(patch.name, "overlapping or out-of-range patch hunk")
        output.extend(original[cursor:start])
        cursor, index, removed, added = start, index + 1, 0, 0
        while index < len(lines) and not lines[index].startswith(b"@@ "):
            line = lines[index]
            if line[:1] not in (b" ", b"-", b"+"):
                _fail(patch.name, "unsupported patch record")
            if line[:1] in (b" ", b"-"):
                if cursor >= len(original) or original[cursor] != line[1:]:
                    _fail(patch.name, "patch context does not match authenticated input")
                cursor, removed = cursor + 1, removed + 1
            if line[:1] in (b" ", b"+"):
                output.append(line[1:])
                added += 1
            index += 1
        if (removed, added) != (old_count, new_count):
            _fail(patch.name, "patch hunk length mismatch")
    output.extend(original[cursor:])
    path.write_bytes(b"".join(output))


def _transform(extracted: Path, destination: Path, bundle: Path,
               inputs: dict, helpers: dict, work: Path) -> None:
    """Reproduce the fixed header/SFD transformations using only prebuilt tools.

    The copied inputs are independently hashed resources. Temporary paths in
    fd2sfd's three-line comment banner are canonicalised to the fixed reference
    spelling; the generated interface body is never rewritten.
    """
    resources = bundle / "ndk-inputs"
    for name in sorted(inputs):
        prefix = "ndk-inputs/patches/"
        if name.startswith(prefix) and name.endswith(".diff"):
            relative = name[len(prefix):-5]
            path = extracted / relative
            if relative == "SFD/exec_lib.sfd":
                # The fixed upstream build retains this pre-edit backup as
                # part of its measured data inventory, not as executable code.
                shutil.copy2(path, path.with_name(path.name + ".orig"))
            _patch(path, bundle / name)

    def copy_tree(source: Path, target: Path, exclude=()) -> None:
        """Copy regular source data only, creating no empty directories."""
        for path in sorted(source.rglob("*")):
            relative = path.relative_to(source)
            if relative.parts[0] in exclude or path.is_dir():
                continue
            if path.is_symlink() or not path.is_file():
                _fail(str(path), "expected ordinary preparation data")
            output = target / relative
            output.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, output)

    stdargs = re.compile(rb"([a-zA-Z0-9 _]*)([ \t]+|\*)([a-zA-Z0-9_]+)\(")

    def calling_convention(root: Path) -> None:
        """Match the fixed byte-level declaration transformation on prototype headers."""
        for path in sorted((root / "clib").glob("*protos.h")):
            path.write_bytes(stdargs.sub(rb"\1\2 __stdargs \3(", path.read_bytes()))

    ndk = destination / "m68k-amigaos/ndk-include"
    old = destination / "m68k-amigaos/ndk13-include"
    libraries = destination / "m68k-amigaos/ndk/lib"
    copy_tree(extracted / "Include_H", ndk, ("proto", "inline"))
    calling_convention(ndk)
    copy_tree(extracted / "Include_I", ndk)
    for source, target in (("FD", "fd"), ("SFD", "sfd"), ("lib", "libs")):
        copy_tree(extracted / source, libraries / target)
    (ndk / "proto").mkdir(exist_ok=True)
    (ndk / "inline").mkdir(exist_ok=True)
    for name in ("alib.h", "cardres.h"):
        shutil.copy2(extracted / "Include_H/proto" / name, ndk / "proto" / name)
    for name in ("alib.h", "stubs.h", "macros.h"):
        shutil.copy2(resources / "helper-headers" / name, ndk / "inline" / name)

    def generate(sfd: Path, root: Path, vbcc: bool = False) -> None:
        """Generate only the same inline/prototype/LVO families as the fixed reference."""
        stem = sfd.name.removesuffix("_lib.sfd")
        modes = [("m68k-gcc-amigaos", "macros", f"inline/{stem}.h"),
                 ("m68k-amigaos", "proto", f"proto/{stem}.h")]
        if vbcc:
            modes += [("m68kvbcc-amigaos", "macros", f"inline/{stem}_protos.h"),
                      ("m68k-amigaos", "lvo", f"lvo/{stem}_lib.i")]
        for target, mode, name in modes:
            output = root / name
            output.parent.mkdir(parents=True, exist_ok=True)
            _run(helpers["sfdc"], ["--target=" + target, "--mode=" + mode,
                                 "--output=" + str(output), str(sfd)], work)

    for sfd in sorted((extracted / "SFD").glob("*_lib.sfd")):
        generate(sfd, ndk, vbcc=True)

    def names(filename: str) -> list[str]:
        """Read a hash-verified fixed path list, never a shell expression."""
        return [_relative(line, filename) for line in
                (resources / "ndk13" / filename).read_text(encoding="utf-8").splitlines()]

    def write(path: Path, content: bytes) -> None:
        """Create only parent directories required by a generated regular file."""
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)

    for name in names("hfiles"):
        write(old / name, (ndk / name).read_bytes())
    for name in names("chfiles"):
        content = (ndk / name).read_bytes()
        if b"V36" in content:
            lines = content.splitlines(keepends=True)
            start = next(index for index, line in enumerate(lines)
                         if re.search(rb"#ifndef[ \t]*CLIB", line))
            stop = next(index for index in range(start, len(lines)) if b"V36" in lines[index])
            content = b"".join(lines[start:stop + 1]).replace(b"__stdargs", b"")
            content += b"#ifdef __cplusplus\n}\n#endif /* __cplusplus */\n#endif\n"
        else:
            content = content.replace(b"__stdargs", b"")
        write(old / name, content)
    for name in names("ehfiles"):
        write(old / name, b"\n")
    write(old / "exec/execname.h", b'#undef\tEXECNAME\n#define\tEXECNAME\t"exec.library"\n')
    for name in names("fdfiles"):
        lines = (libraries / "fd" / name).read_bytes().splitlines(keepends=True)
        start = next(index for index, line in enumerate(lines) if b"##base" in line)
        stop = next((index for index in range(start, len(lines)) if b"V36" in lines[index]), len(lines) - 1)
        write(libraries / "fd13" / name, b"".join(lines[start:stop + 1]))
    for fd in sorted((libraries / "fd13").glob("*")):
        name = fd.name.removesuffix("_lib.fd")
        prototypes = old / "clib" / (name + "_protos.h")
        content = _run(helpers["fd2sfd"], [str(fd), str(prototypes)], work)
        banner = (f"* This SFD file was automatically generated by fd2sfd from\n"
                  f"* {fd} and\n* {prototypes}.\n").encode("utf-8")
        if not content.startswith(banner):
            _fail(fd.name, "unexpected fd2sfd banner")
        fixed = "/work/reference/sdk/"
        canonical = (f"* This SFD file was automatically generated by fd2sfd from\n"
                     f"* {fixed}{fd.relative_to(destination)} and\n"
                     f"* {fixed}{prototypes.relative_to(destination)}.\n").encode("utf-8")
        sfd = libraries / "sfd13" / (fd.stem + ".sfd")
        write(sfd, canonical + content[len(banner):])
        generate(sfd, old)
    calling_convention(old)
    copy_tree(resources / "netinclude", ndk)


def main(arguments: list[str] | None = None) -> int:
    """Authenticate all preconditions, prepare privately and write the completion marker last."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    args = parser.parse_args(arguments)
    try:
        bundle = Path(__file__).resolve().parent
        metadata = _metadata(_json(bundle / "preparation.json"))
        if args.destination.is_symlink() or not args.destination.is_dir():
            _fail("destination", "expected an existing ordinary inactive SDK tree")
        destination = args.destination.resolve(strict=True)
        marker = destination / ".ndk-prepared.json"
        if marker.exists() or marker.is_symlink():
            _fail(marker.name, "refusing an already prepared SDK")
        _compare(_tree(destination), metadata["base"], "SDK baseline")
        archive = args.archive.resolve(strict=True)
        identity = metadata["archive"]["source"]
        if archive.stat().st_size != metadata["archive"]["bytes"] or _hash(archive) != metadata["archive"]["sha256"]:
            _fail(identity, "archive length/hash mismatch")
        helpers = {}
        for name in ("lha", "sfdc", "fd2sfd"):
            if name not in metadata["helpers"]:
                _fail(name, "required helper is absent")
        for name, record in metadata["helpers"].items():
            try:
                path = _resource(bundle, record["path"], record["sha256"])
            except OSError as error:
                raise ValueError(f"{name}: missing helper {record['path']!r}") from error
            if not os.access(path, os.X_OK):
                _fail(name, "helper is not executable")
            helpers[name] = path
        for name, digest in metadata["inputs"].items():
            _resource(bundle, name, digest)
        resources = bundle / "ndk-inputs"
        if resources.is_symlink() or not resources.is_dir():
            _fail("ndk-inputs", "missing ordinary fixed transformation inputs")
        for name in _tree(resources):
            if "ndk-inputs/" + name not in metadata["inputs"]:
                _fail(name, "undeclared preparation input")
        required = [
            *(f"ndk-inputs/helper-headers/{name}" for name in ("alib.h", "stubs.h", "macros.h")),
            *(f"ndk-inputs/ndk13/{name}" for name in ("hfiles", "chfiles", "ehfiles", "fdfiles")),
            *(f"ndk-inputs/patches/{name}.diff" for name in (
                "Include_H/devices/timer.h", "Include_H/graphics/gfxbase.h",
                "Include_H/hardware/cia.h", "Include_H/hardware/custom.h",
                "SFD/dos_lib.sfd", "SFD/exec_lib.sfd", "SFD/locale_lib.sfd")),
        ]
        for name in required:
            if name not in metadata["inputs"]:
                _fail(name, "missing fixed preparation input")
        if not any(name.startswith("ndk-inputs/netinclude/") for name in metadata["inputs"]):
            _fail("netinclude", "missing fixed networking overlay")
        with tempfile.TemporaryDirectory(prefix="ndk-prepare-",
                                         dir=os.environ.get("CODEOPS_TMPDIR") or os.environ.get("TMPDIR")) as temporary:
            work = Path(temporary).resolve()
            extracted = _extract(archive, helpers["lha"], work)
            _transform(extracted, destination, bundle, metadata["inputs"], helpers, work)
        expected = dict(metadata["base"])
        expected.update({name: {"type": "file", "sha256": digest}
                         for name, digest in metadata["inventory"].items()})
        _compare(_tree(destination), expected, "prepared SDK")
        encoded = (json.dumps(metadata["inventory"], sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")
        completion = {"schema": 1, "lock_sha256": metadata["lock_sha256"],
                      "ndk_sha256": metadata["archive"]["sha256"],
                      "inventory_sha256": hashlib.sha256(encoded).hexdigest()}
        with marker.open("x", encoding="utf-8") as stream:
            try:
                stream.write(json.dumps(completion, sort_keys=True, separators=(",", ":")) + "\n")
            except BaseException:
                marker.unlink()
                raise
    except (OSError, ValueError, RuntimeError, RecursionError, StopIteration,
            subprocess.TimeoutExpired) as error:
        print(f"prepare-ndk: {error}", file=sys.stderr)
        return 1
    print("prepare-ndk: exact prepared inventory verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
