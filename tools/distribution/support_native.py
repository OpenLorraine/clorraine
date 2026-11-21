"""Inspect actual native payload bytes without trusting filenames or audit labels."""

from __future__ import annotations

from pathlib import Path
import re
import stat
import subprocess
import tempfile
import zipfile

from support import SupportError


def safe_relative(value: object, identity: str) -> str:
    """Require a UTF-8 canonical relative file/member name or raise SupportError."""
    if not isinstance(value, str) or not value:
        raise SupportError(f"{identity}: expected a relative path")
    try:
        value.encode("utf-8")
    except UnicodeError as error:
        raise SupportError(f"{identity!r}: invalid UTF-8 path") from error
    if (value.startswith("/") or "\\" in value or "!" in value
            or any(part in ("", ".", "..") for part in value.split("/"))
            or any(ord(char) < 32 or 127 <= ord(char) <= 159 for char in value)):
        raise SupportError(f"{identity}: unsafe relative path {value!r}")
    return value


def _run(arguments: list[str], identity: str, *, required: bool = True) -> str:
    """Use literal inspection-tool arguments and propagate named, bounded failures."""
    result = subprocess.run(arguments, capture_output=True, timeout=180, check=False)
    if result.returncode:
        if not required:
            return ""
        raise SupportError(f"{identity}: native inspection failed ({arguments[0]})")
    return result.stdout.decode("utf-8")


def _elf(path: Path, identity: str, *, archive: bool = False) -> dict | None:
    """Read ELF headers, dynamic records and version requirements, including ar members."""
    header = _run(["readelf", "--file-header", "--wide", str(path)],
                  identity, required=not archive)
    if not header:
        return None
    machines = re.findall(r"^\s*Machine:\s*(.+)$", header, re.M)
    known = {"Advanced Micro Devices X86-64": "x86_64", "MC68000": "m68k"}
    if not machines or any(machine not in known for machine in machines):
        raise SupportError(f"{identity}: unsupported or missing ELF architecture")
    architectures = {known[machine] for machine in machines}
    if len(architectures) != 1:
        raise SupportError(f"{identity}: mixed archive architectures")
    dynamic = _run(["readelf", "--dynamic", "--wide", str(path)], identity)
    versions = _run(["readelf", "--version-info", "--wide", str(path)], identity)
    if "Name: GLIBC_PRIVATE" in versions:
        raise SupportError(f"{identity}: private GLIBC ABI cannot meet the supported baseline")
    glibc = sorted(set(re.findall(r"Name: GLIBC_([0-9.]+)", versions)))
    if any(tuple(map(int, version.split("."))) > (2, 35) for version in glibc):
        raise SupportError(f"{identity}: GLIBC requirement exceeds 2.35")
    sonames = re.findall(r"\(SONAME\).*?\[(.*?)\]", dynamic)
    if len(sonames) > 1:
        raise SupportError(f"{identity}: ambiguous SONAME")
    return {
        "format": "ar" if archive else "elf", "machine": architectures.pop(),
        "soname": sonames[0] if sonames else None,
        "needed": sorted(set(re.findall(r"\(NEEDED\).*?\[(.*?)\]", dynamic))),
        "glibc": glibc,
        "rpaths": re.findall(r"\((?:RPATH|RUNPATH)\).*?\[(.*?)\]", dynamic),
    }


def _target(path: Path, identity: str, tool: Path | None) -> dict:
    """Let the audited cross-objdump distinguish real Amiga archives from objects."""
    if tool is None:
        raise SupportError(f"{identity}: target inspection requires the audited cross-objdump")
    report = _run([str(tool), "-f", str(path)], identity)
    formats = re.findall(r"file format\s+(\S+)", report)
    architectures = re.findall(r"^architecture:\s*([^,]+),", report, re.M)
    if (not formats or len(formats) != len(architectures)
            or any(value not in {"amiga", "a.out-amiga"} for value in formats)
            or any(re.fullmatch(r"m68k(?::[a-zA-Z0-9]+)?", value) is None
                   for value in architectures)):
        raise SupportError(f"{identity}: expected complete m68k Amiga object contents")
    return {
        "format": "ar" if report.lstrip().startswith("In archive ") else "amiga-object",
        "machine": "m68k", "soname": None, "needed": [], "glibc": [], "rpaths": [],
    }


def _signature(path: Path) -> bytes:
    """Read a short content signature without loading a large target archive."""
    with path.open("rb") as stream:
        return stream.read(8)


def _kind(signature: bytes) -> str | None:
    """Recognise native encodings independently of their filename extensions."""
    if signature.startswith(b"\x7fELF"):
        return "elf"
    if signature == b"!<arch>\n":
        return "ar"
    if signature[:4] in {b"\0\0\x03\xe7", b"\0\0\x03\xf3", b"\0\0\x03\xfa"}:
        return "amiga"
    return None


def _inspect(path: Path, identity: str, kind: str, tool: Path | None) -> dict:
    """Inspect one recognised native encoding, not a record-selected interpretation."""
    if kind == "elf":
        return _elf(path, identity)
    if kind == "ar":
        result = _elf(path, identity, archive=True)
        if result is not None:
            return result
    return _target(path, identity, tool)


def inspect_payload(root: Path, files: dict, builder_prefixes: list[str],
                    declared: dict | None = None) -> dict:
    """Return measured native records for every payload file and native wheel member.

    Files and component obligations must be authenticated by the caller first.
    The only payload program invoked is the fixed cross-objdump path, after
    its actual host ELF shape, audit role and executable bit are checked.
    Wheel members are inspected as private data files, never executed.
    """
    measured = {}
    pending = []
    tool_name = "bin/m68k-amigaos-objdump"
    tool = None
    for name, record in files.items():
        if record["type"] != "file":
            continue
        path = root / name
        signature = _signature(path)
        kind = _kind(signature)
        if kind == "elf":
            measured[name] = _inspect(path, name, kind, None)
        elif kind is not None:
            pending.append((name, path, kind))
        if record["role"] not in {"source", "notice", "review"} and signature.startswith(b"#!"):
            with path.open("rb") as stream:
                shebang = stream.readline(4097)
            if len(shebang) > 4096:
                raise SupportError(f"{name}: oversized executable shebang")
            text = shebang.decode("utf-8")
            if any(prefix in text for prefix in builder_prefixes):
                raise SupportError(f"{name}: executable builder-prefix shebang")
    if tool_name in measured:
        candidate = root / tool_name
        if (files[tool_name]["role"] != "program"
                or measured[tool_name]["machine"] != "x86_64"
                or not candidate.stat().st_mode & stat.S_IXUSR):
            raise SupportError(f"{tool_name}: expected an audited executable host inspector")
        if declared is not None:
            expected = declared.get(tool_name)
            observed = measured[tool_name]
            if (expected is None or any(expected[field] != observed[field]
                                        for field in ("format", "machine", "soname"))
                    or set(expected["needed"]) != set(observed["needed"])
                    or set(expected["glibc"]) != set(observed["glibc"])):
                raise SupportError(f"{tool_name}: inspector native record mismatch")
        tool = candidate
    for name, path, kind in pending:
        measured[name] = _inspect(path, name, kind, tool)
    for name, record in files.items():
        if record["type"] != "file" or record["role"] != "wheel":
            continue
        try:
            with zipfile.ZipFile(root / name) as archive, tempfile.TemporaryDirectory(
                    prefix="support-native-") as directory:
                seen = set()
                for member in archive.infolist():
                    key = safe_relative(member.filename.rstrip("/") if member.is_dir()
                                        else member.filename, name)
                    if key in seen:
                        raise SupportError(f"{name}: duplicate wheel member {key!r}")
                    seen.add(key)
                    mode = member.external_attr >> 16
                    if mode & 0o7000 or stat.S_IFMT(mode) not in {0, stat.S_IFREG, stat.S_IFDIR}:
                        raise SupportError(f"{name}!{key}: special wheel member")
                    if member.is_dir():
                        continue
                    with archive.open(member) as stream:
                        signature = stream.read(8)
                    kind = _kind(signature)
                    if kind is None:
                        continue
                    if member.file_size > 512 * 1024 * 1024:
                        raise SupportError(f"{name}!{key}: native member exceeds 512 MiB")
                    path = Path(directory) / key
                    path.parent.mkdir(parents=True, exist_ok=True)
                    with archive.open(member) as stream, path.open("xb") as output:
                        while block := stream.read(1024 * 1024):
                            output.write(block)
                    measured[name + "!" + key] = _inspect(path, name + "!" + key, kind, tool)
        except (zipfile.BadZipFile, RuntimeError) as error:
            raise SupportError(f"{name}: invalid wheel archive") from error
    return measured
