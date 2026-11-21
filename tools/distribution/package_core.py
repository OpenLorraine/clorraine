#!/usr/bin/env python3
"""Archive current native executables or assemble an inactive local candidate."""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile

from prepare_ndk import validate_archive_members
from support import SupportError, load_lock
from support_audit import audit_payload, load_audit
from support_build import inventory, verify_wheels
from support_native import safe_relative


BINARIES = ("clc", "clorraine", "clorraine-lsp")
"""The only executable inputs accepted by the core archive producer."""
NOTICES = ("LICENSE", "LICENSE-EXCEPTION.md", "NOTICE", "TRADEMARKS.md", "THIRD_PARTY_NOTICES.md")
"""First-party terms and notices copied from the producer's own checkout."""
PAYLOAD = tuple("bin/" + name for name in BINARIES) + NOTICES
"""Exact indexed core members; the core descriptor itself is not self-indexed."""


def encoded(value: object) -> bytes:
    """Encode deterministic UTF-8 metadata, including the inventory's final newline."""
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def json_object(data: bytes, identity: str) -> dict:
    """Read bounded JSON and reject duplicates before interpreting any fields."""
    def unique(pairs):
        """Never let a later key replace an already checked value."""
        result = {}
        for key, value in pairs:
            if key in result:
                raise SupportError(f"{identity}: duplicate JSON key {key!r}")
            result[key] = value
        return result
    if len(data) > 16 * 1024 * 1024:
        raise SupportError(f"{identity}: metadata exceeds 16 MiB")
    value = json.loads(data.decode("utf-8"), object_pairs_hook=unique)
    if not isinstance(value, dict):
        raise SupportError(f"{identity}: expected a JSON object")
    return value


def ordinary(path: Path):
    """Open an ordinary non-aliased input without blocking on replacement FIFOs."""
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        if not stat.S_ISREG(os.fstat(descriptor).st_mode):
            raise SupportError(f"{path}: expected an ordinary file")
        return os.fdopen(descriptor, "rb")
    except BaseException:
        os.close(descriptor)
        raise


def digest_stream(stream) -> str:
    """Hash archive-sized content with bounded memory."""
    digest = hashlib.sha256()
    for block in iter(lambda: stream.read(1024 * 1024), b""):
        digest.update(block)
    return digest.hexdigest()


def file_record(path: Path, executable: bool) -> dict:
    """Measure actual ordinary input bytes without changing their permissions."""
    with ordinary(path) as stream:
        size = os.fstat(stream.fileno()).st_size
        if size <= 0:
            raise SupportError(f"{path}: empty input")
        return {"bytes": size, "sha256": digest_stream(stream), "executable": executable}


def elf_executable(path: Path) -> None:
    """Require a supplied Linux x86_64 executable, not an executable script/banner."""
    with ordinary(path) as stream:
        header = stream.read(64)
        status = os.fstat(stream.fileno())
    if (not os.access(path, os.X_OK) or not status.st_mode & 0o111
            or len(header) != 64 or header[:6] != b"\x7fELF\x02\x01"
            or int.from_bytes(header[16:18], "little") not in (2, 3)
            or int.from_bytes(header[18:20], "little") != 62):
        raise SupportError(f"{path}: expected an executable Linux x86_64 ELF file")


def run(arguments: list, cwd: Path, environment: dict | None = None) -> str:
    """Run a bounded literal process, keeping large output in owned scratch files.

    Only a small tail is read for a failure diagnostic. Dependencies are never
    built or downloaded by a process launched from this producer.
    """
    with tempfile.TemporaryFile(dir=cwd) as stdout, tempfile.TemporaryFile(dir=cwd) as stderr:
        result = subprocess.run(
            [str(arg) for arg in arguments], cwd=cwd, env=environment,
            stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr, timeout=180,
            preexec_fn=limit_output)
        if result.returncode:
            stderr.seek(max(0, stderr.tell() - 8192))
            detail = stderr.read(8192).decode("utf-8", errors="replace")
            raise SupportError(f"{arguments[0]}: process failed ({result.returncode}): {detail}")
        stdout.seek(0)
        data = stdout.read(1024 * 1024 + 1)
        if len(data) > 1024 * 1024:
            raise SupportError(f"{arguments[0]}: process output exceeds 1 MiB")
        return data.decode("utf-8")


def limit_output() -> None:
    """Cap child-created files at one GiB without raising a stricter caller cap."""
    current = [value for value in resource.getrlimit(resource.RLIMIT_FSIZE) if value >= 0]
    limit = min([1024 ** 3, *current])
    resource.setrlimit(resource.RLIMIT_FSIZE, (limit, limit))


def absent(output: Path) -> None:
    """Refuse files, directories and dangling aliases alike; never repair them."""
    if os.path.lexists(output):
        raise SupportError(f"output {output}: refusing an existing destination")


def core_metadata(archive: tarfile.TarFile) -> dict:
    """Validate every core tar header, record and content hash before extraction."""
    members = archive.getmembers()
    names = [member.name for member in members]
    if len(names) != len(set(names)) or set(names) != set(PAYLOAD) | {"core.json"}:
        raise SupportError("core archive: duplicate, missing or unexpected member")
    for member in members:
        executable = member.name.startswith("bin/")
        if (not member.isfile() or member.mode != (0o755 if executable else 0o644)
                or (member.uid, member.gid, member.mtime) != (0, 0, 0)
                or member.uname or member.gname):
            raise SupportError(f"core {member.name}: invalid type/mode/owner/time")
    member = archive.getmember("core.json")
    if member.size > 16 * 1024 * 1024:
        raise SupportError("core.json: metadata exceeds 16 MiB")
    metadata = json_object(archive.extractfile(member).read(), "core.json")
    if (set(metadata) != {"schema", "core_version", "files"}
            or type(metadata["schema"]) is not int or metadata["schema"] != 1
            or not isinstance(metadata["core_version"], str)
            or re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", metadata["core_version"]) is None
            or not isinstance(metadata["files"], dict) or set(metadata["files"]) != set(PAYLOAD)):
        raise SupportError("core.json: invalid schema/version/file coverage")
    for name, record in metadata["files"].items():
        if (not isinstance(record, dict) or set(record) != {"bytes", "sha256", "executable"}
                or type(record["bytes"]) is not int or record["bytes"] <= 0
                or not isinstance(record["sha256"], str)
                or re.fullmatch(r"[0-9a-f]{64}", record["sha256"]) is None
                or type(record["executable"]) is not bool
                or record["executable"] != name.startswith("bin/")):
            raise SupportError(f"core.json {name}: invalid file record")
        member = archive.getmember(name)
        if member.size != record["bytes"] or digest_stream(archive.extractfile(member)) != record["sha256"]:
            raise SupportError(f"core {name}: byte length/hash mismatch")
    return metadata


def archive_core(binaries: Path, output: Path) -> None:
    """Publish a deterministic nine-member archive from current input bytes.

    Inventory, version queries and archiving use the same private input copies.
    Reopening caller paths between those steps could validate a different
    executable from the one whose bytes enter the archive.
    """
    absent(output)
    repository = Path(__file__).resolve().parents[2]
    manifest = (repository / "compiler/Cargo.toml").read_text(encoding="utf-8")
    version = re.search(r'^version = "([0-9]+\.[0-9]+\.[0-9]+)"$', manifest, re.M)
    if version is None:
        raise SupportError("compiler/Cargo.toml: missing current core version")
    binaries = binaries.absolute()
    sources = {name: binaries / name[4:] if name.startswith("bin/") else repository / name
               for name in PAYLOAD}
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".core-archive-", dir=output.parent) as temporary:
        work = Path(temporary)
        snapshots = {}
        for name, source in sources.items():
            snapshot = work / "inputs" / name
            snapshot.parent.mkdir(parents=True, exist_ok=True)
            with ordinary(source) as stream:
                mode = os.fstat(stream.fileno()).st_mode & 0o777
                # Copying must not repair a caller's non-executable input.
                if name.startswith("bin/") and (not mode & 0o111 or not os.access(source, os.X_OK)):
                    raise SupportError(f"{source}: expected an executable Linux x86_64 ELF file")
                with snapshot.open("xb") as destination:
                    shutil.copyfileobj(stream, destination)
            snapshot.chmod(mode)
            snapshots[name] = snapshot
        records = {name: file_record(path, name.startswith("bin/"))
                   for name, path in snapshots.items()}
        for name in BINARIES:
            path = snapshots["bin/" + name]
            elf_executable(path)
            banner = run([path, "--version"], work).strip()
            if banner != f"{name} {version[1]}":
                raise SupportError(f"{name}: version query must report '{name} {version[1]}'")
        metadata = {"schema": 1, "core_version": version[1], "files": records}
        candidate = work / "core.tar"
        with tarfile.open(candidate, "w", format=tarfile.PAX_FORMAT) as archive:
            for name in sorted((*PAYLOAD, "core.json")):
                info = tarfile.TarInfo(name)
                info.mode = 0o755 if name.startswith("bin/") else 0o644
                if name == "core.json":
                    data = encoded(metadata)
                    info.size = len(data)
                    archive.addfile(info, io.BytesIO(data))
                else:
                    info.size = records[name]["bytes"]
                    with ordinary(snapshots[name]) as source:
                        archive.addfile(info, source)
        with tarfile.open(candidate, "r:") as archive:
            core_metadata(archive)
        os.link(candidate, output)


def extract(archive: tarfile.TarFile, root: Path, records: dict | None = None) -> None:
    """Validate a complete safe listing before writing any file or link.

    Support content is compared with the audit before and after extraction.
    Implicit directories are allowed, but no member may be extracted through
    an alias; symlinks are created only after all ordinary files.
    """
    members = archive.getmembers()
    listing = []
    found = {}
    for member in members:
        name = member.name.rstrip("/") if member.isdir() else member.name
        safe_relative(name, "archive")
        kind = ("file" if member.isfile() else "directory" if member.isdir()
                else "symlink" if member.issym() else None)
        if kind is None or member.mode & 0o7000 or (kind != "symlink" and member.mode & 0o022):
            raise SupportError(f"{name}: unsupported archive type or unsafe mode")
        listing.append({"path": name, "type": kind,
                        "target": member.linkname if kind == "symlink" else None})
        if kind == "file":
            found[name] = {"type": kind, "sha256": digest_stream(archive.extractfile(member))}
        elif kind == "symlink":
            found[name] = {"type": kind, "target": member.linkname}
    validate_archive_members(listing)
    if records is not None:
        if set(found) != set(records):
            raise SupportError("support audit: archive file/link coverage mismatch")
        for name, observed in found.items():
            if (not isinstance(records[name], dict)
                    or any(records[name].get(key) != value for key, value in observed.items())):
                raise SupportError(f"{name}: support audit bytes/link mismatch")
    root.mkdir(parents=True, exist_ok=True)
    for member, entry in zip(members, listing):
        path = root / entry["path"]
        path.parent.mkdir(parents=True, exist_ok=True)
        if entry["type"] == "directory":
            path.mkdir(exist_ok=True)
        elif entry["type"] == "file":
            with archive.extractfile(member) as source, path.open("xb") as target:
                shutil.copyfileobj(source, target)
            path.chmod(member.mode & 0o777)
            with ordinary(path) as source:
                if digest_stream(source) != found[entry["path"]]["sha256"]:
                    raise SupportError(f"{path.name}: archive changed during extraction")
    for entry in listing:
        if entry["type"] == "symlink":
            (root / entry["path"]).symlink_to(entry["target"])


def support_archive(path: Path, audit_path: Path, root: Path, lock: dict, digest: str) -> None:
    """Authenticate complete support bytes and reuse the existing payload audit."""
    with ordinary(audit_path):
        pass
    evidence = load_audit(audit_path)
    if (not isinstance(evidence, dict)
            or set(evidence) != {"schema", "lock_sha256", "components", "files", "native"}
            or type(evidence["schema"]) is not int or evidence["schema"] != 1
            or evidence["lock_sha256"] != digest or not isinstance(evidence["files"], dict)):
        raise SupportError(f"{audit_path.name}: invalid support audit schema/lock")
    with ordinary(path) as stream, tarfile.open(fileobj=stream, mode="r:") as archive:
        extract(archive, root, evidence["files"])
    audit_payload(lock, digest, root, evidence)


def publish_directory(candidate: Path, output: Path) -> None:
    """Atomically publish on Linux without replacing even a raced empty directory.

    Ordinary rename can replace an empty directory. Ubuntu's renameat2 with
    RENAME_NOREPLACE instead fails for every existing destination type.
    """
    libc = ctypes.CDLL(None, use_errno=True)
    rename = libc.renameat2
    rename.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
    rename.restype = ctypes.c_int
    if rename(-100, os.fsencode(candidate), -100, os.fsencode(output), 1) != 0:
        error = ctypes.get_errno()
        raise OSError(error, os.strerror(error), str(output))


def install_venv(python: Path, venv: Path, wheelset: Path, work: Path, environment: dict) -> None:
    """Install only the audited pinned binary wheels with network/index disabled."""
    metadata = load_audit(wheelset / "wheels.json")
    requirements = work / "wheel-pins.txt"
    requirements.write_text("".join(
        f"{name}=={record['version']} --hash=sha256:{record['sha256']}\n"
        for name, record in sorted(metadata["packages"].items())), encoding="utf-8")
    run([python, "-I", "-m", "venv", venv], work, environment)
    runner = venv / "bin/python"
    run([runner, "-I", "-m", "pip", "--isolated", "--no-cache-dir", "install",
         "--no-index", "--find-links", wheelset / "wheels", "--only-binary=:all:",
         "--no-deps", "--require-hashes", "--force-reinstall", "-r", requirements],
        work, environment)
    run([runner, "-I", "-m", "pip", "--isolated", "check"], work, environment)
    # Check the executing prefix, isolation and exact installed versions, not
    # just a launcher banner or the source receipt.
    probe = (
        "import importlib.metadata,json,site,sys;"
        "print(json.dumps([sys.prefix,sys.flags.isolated,site.ENABLE_USER_SITE,"
        "{n:importlib.metadata.version(n) for n in sys.argv[1:]}]))"
    )
    observed = json.loads(run([runner, "-I", "-c", probe, *sorted(metadata["packages"])],
                              work, environment))
    expected = [str(venv), 1, False,
                {name: record["version"] for name, record in metadata["packages"].items()}]
    if observed != expected:
        raise SupportError("python: installed venv prefix/isolation/package versions differ")
    shutil.copyfile(wheelset / "wheels.json", venv / "wheels.json")


def stage_core(args: argparse.Namespace) -> None:
    """Assemble verified inputs privately; publish no activation or installer state."""
    absent(args.output)
    lock, digest = load_lock(args.lock)
    ndk = lock["sources"][lock["preparation"]["archive"]]
    with ordinary(args.ndk) as stream:
        if os.fstat(stream.fileno()).st_size != ndk["bytes"] or digest_stream(stream) != ndk["sha256"]:
            raise SupportError("ndk: archive byte length/hash mismatch")
    python = args.python.absolute()
    if not python.is_file() or not os.access(python, os.X_OK):
        raise SupportError("python: selected interpreter is missing or not executable")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".core-stage-", dir=args.output.parent) as temporary:
        work = Path(temporary)
        environment = {name: value for name, value in os.environ.items()
                       if not name.startswith(("PYTHON", "PIP_", "LD_"))}
        home = work / "home"
        home.mkdir()
        environment.update(HOME=str(home), TMPDIR=str(work), CODEOPS_TMPDIR=str(work),
                           PYTHONDONTWRITEBYTECODE="1", PATH="/usr/bin:/bin", LC_ALL="C")
        abi = run([python, "-I", "-c",
                   "import sys;print(sys.implementation.name);"
                   "print('cp%d%d'%sys.version_info[:2]);print(sys.prefix==sys.base_prefix)"],
                  work, environment).splitlines()
        if len(abi) != 3 or abi[0] != "cpython" or abi[1] not in ("cp310", "cp312") or abi[2] != "True":
            raise SupportError("python: require system CPython 3.10 or 3.12")
        candidate = work / "candidate"
        with ordinary(args.core) as stream, tarfile.open(fileobj=stream, mode="r:") as archive:
            core = core_metadata(archive)
            extract(archive, candidate)
        for name in BINARIES:
            elf_executable(candidate / "bin" / name)
        sdk = candidate / "sdk/amiga"
        support_archive(args.sdk_archive, args.sdk_audit, sdk, lock, digest)
        preparation = work / "preparation"
        support_archive(args.preparation_archive, args.preparation_audit, preparation, lock, digest)
        wheelset = work / "wheelset"
        support_archive(args.wheels_archive, args.wheels_audit, wheelset, lock, digest)
        verify_wheels(wheelset, lock, digest, abi[1], inventory(wheelset))
        run([python, "-I", preparation / "prepare_ndk.py",
             "--archive", args.ndk.absolute(), "--destination", sdk], work, environment)
        prepared = dict(inventory(sdk))
        receipt = load_audit(sdk / ".ndk-prepared.json")
        del prepared[".ndk-prepared.json"]
        expected = load_audit(args.sdk_audit)["files"]
        expected = {name: {key: value for key, value in record.items() if key in ("type", "sha256", "target")}
                    for name, record in expected.items()}
        expected.update({name: {"type": "file", "sha256": value}
                         for name, value in lock["preparation"]["inventory"].items()})
        inventory_hash = hashlib.sha256(encoded(lock["preparation"]["inventory"])).hexdigest()
        if prepared != expected or receipt != {
                "schema": 1, "lock_sha256": digest, "ndk_sha256": ndk["sha256"],
                "inventory_sha256": inventory_hash}:
            raise SupportError("preparation: SDK inventory/completion differs from verified inputs")
        install_venv(python, candidate / "sdk/vamos", wheelset, work, environment)
        for name, record in core["files"].items():
            if file_record(candidate / name, name.startswith("bin/")) != record:
                raise SupportError(f"core {name}: staged bytes changed")
        descriptor = {"schema": 1, "core_version": core["core_version"],
                      "support_lock_sha256": digest, "ndk_sha256": ndk["sha256"],
                      "prepared_inventory_sha256": inventory_hash, "python_abi": abi[1]}
        (candidate / "installation.json").write_bytes(encoded(descriptor))
        publish_directory(candidate, args.output)


def main(arguments: list[str] | None = None) -> int:
    """Validate the public archive/stage signatures and report named failures."""
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    operations = parser.add_subparsers(dest="operation", required=True)
    archive = operations.add_parser("archive", allow_abbrev=False)
    archive.add_argument("--binaries", type=Path, required=True)
    archive.add_argument("--output", type=Path, required=True)
    stage = operations.add_parser("stage", allow_abbrev=False)
    for name in ("core", "lock", "sdk-archive", "sdk-audit", "preparation-archive",
                 "preparation-audit", "wheels-archive", "wheels-audit", "ndk", "python", "output"):
        stage.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args(arguments)
    try:
        if sys.platform != "linux" or os.uname().machine != "x86_64" or os.getuid() == 0:
            raise SupportError("require an ordinary-user Linux x86_64 maintainer")
        args.output = args.output.absolute()
        if args.operation == "archive":
            archive_core(args.binaries, args.output)
        else:
            stage_core(args)
    except (OSError, ValueError, SupportError, RuntimeError, RecursionError,
            tarfile.TarError, subprocess.SubprocessError, AttributeError) as error:
        # Paths and failed subprocess output are untrusted terminal text.
        message = str(error)
        message = "".join(char if char.isprintable() else f"\\u{ord(char):04x}" for char in message)
        print(f"package-core: {message}", file=sys.stderr)
        return 1
    print(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
