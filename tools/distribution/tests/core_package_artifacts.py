"""Independent real-input identities and assembled-layout observations."""

from __future__ import annotations

import hashlib
import json
import os
import platform
import posixpath
import re
import resource
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path, PurePosixPath

from core_package_fixtures import (
    BINARIES, PAYLOAD, digest_file, digest_stream, elf_x64,
    inspect_core, json_bytes, read_json, read_json_bytes,
)


def required_environment(name: str) -> Path:
    """Require an absolute explicit real-proof path; absent inputs are errors."""
    value = os.environ.get(name)
    if not value or not Path(value).is_absolute() or not Path(value).exists():
        raise AssertionError(f"{name} must name an existing absolute actual-proof input")
    return Path(value)


def _relative(name: str) -> None:
    """Reject unsafe archive names before a test interprets their inventories."""
    if (not name or "\\" in name or any(part in ("", ".", "..") for part in name.split("/"))
            or name.startswith("/") or any(ord(char) < 32 or ord(char) == 127 for char in name)):
        raise AssertionError(f"unsafe relative archive path: {name!r}")


def audit_inventory(archive_path: Path, audit_path: Path, lock_hash: str) -> dict:
    """Compare actual members with their complete independent audit records.

    Native dependency/licence acceptance remains the earlier real support proof.
    No production validator is called to decide which bytes are expected.
    """
    audit = read_json(audit_path)
    if (set(audit) != {"schema", "lock_sha256", "components", "files", "native"}
            or type(audit["schema"]) is not int or audit["schema"] != 1
            or audit["lock_sha256"] != lock_hash):
        raise AssertionError(f"invalid or differently bound audit: {audit_path}")
    found = {}
    with tarfile.open(archive_path) as archive:
        members = archive.getmembers()
        names = set()
        for member in members:
            name = member.name.rstrip("/") if member.isdir() else member.name
            _relative(name)
            if name in names or member.mode & 0o7000:
                raise AssertionError(f"duplicate member or unsafe mode: {name}")
            names.add(name)
            if member.isdir():
                continue
            if member.isfile():
                found[name] = {"type": "file", "sha256": digest_stream(archive.extractfile(member))}
            elif member.issym():
                target = member.linkname
                resolved = posixpath.normpath(posixpath.join(posixpath.dirname(name), target))
                if target.startswith("/") or resolved == ".." or resolved.startswith("../"):
                    raise AssertionError(f"escaping support link: {name}")
                found[name] = {"type": "symlink", "target": target}
            else:
                raise AssertionError(f"unsupported support member: {name}")
        if set(found) != set(audit["files"]):
            raise AssertionError(f"support audit coverage mismatch: {archive_path}")
        for name, observed in found.items():
            record = audit["files"][name]
            if any(record.get(key) != value for key, value in observed.items()):
                raise AssertionError(f"support audit bytes/link mismatch: {name}")
            for parent in PurePosixPath(name).parents:
                if found.get(parent.as_posix(), {}).get("type") == "symlink":
                    raise AssertionError(f"support member has a symlink parent: {name}")
    return found


def tree(root: Path) -> dict:
    """Describe actual file/link bytes without walking directory aliases."""
    result = {}
    for directory, folders, files in os.walk(root, followlinks=False):
        for name in folders + files:
            path = Path(directory) / name
            relative = path.relative_to(root).as_posix()
            if path.is_symlink():
                result[relative] = {"type": "symlink", "target": os.readlink(path)}
            elif path.is_file():
                result[relative] = {"type": "file", "sha256": digest_file(path)}
            elif not path.is_dir():
                raise AssertionError(f"unsupported assembled file: {path}")
    return result


def _limit_files() -> None:
    """Keep child-created logs and other individual outputs below one GiB."""
    limits = [value for value in resource.getrlimit(resource.RLIMIT_FSIZE) if value >= 0]
    limit = min([1024 ** 3, *limits])
    resource.setrlimit(resource.RLIMIT_FSIZE, (limit, limit))


class ActualInputs:
    """Reuse the explicit support-proof record and own disposable process work.

    ``inputs`` uses the existing support acceptance schema. ``paths`` contains
    verified archive/audit/lock/NDK paths; ``inventories`` contains independently
    hashed support members, and ``identities`` pins the original input bytes.
    ``lock`` is the supplied support lock. ``root`` and ``environment`` isolate
    test work. ``count`` names process logs; ``keep_reason`` protects an input
    that could not be restored after a runtime relocation.
    Construction never stages a candidate or runs a production validator.
    """

    def __init__(self, runtime: bool = False) -> None:
        """Require genuine matching cohorts and immutable input identities."""
        self.inputs = read_json(required_environment("CLORRAINE_SUPPORT_ACCEPTANCE"))
        expected = {"schema", "ubuntu", "python_abi", "lock", "amiga", "preparation",
                    "wheels", "ndk_archive", "fixtures", "work_root"}
        if (set(self.inputs) != expected or type(self.inputs["schema"]) is not int
                or self.inputs["schema"] != 1):
            raise AssertionError("invalid existing support acceptance record")
        actual = platform.freedesktop_os_release()
        cohort = (self.inputs["ubuntu"], self.inputs["python_abi"])
        if (cohort not in (("22.04", "cp310"), ("24.04", "cp312"))
                or actual.get("ID") != "ubuntu" or actual.get("VERSION_ID") != cohort[0]
                or platform.machine() != "x86_64" or os.getuid() == 0
                or sys_abi() != cohort[1] or sys.prefix != sys.base_prefix):
            raise AssertionError("actual ordinary-user Ubuntu/CPython cohort must match support inputs")
        if Path("/opt/amiga").exists() or Path("/var/run/docker.sock").exists():
            raise AssertionError("original SDK and Docker socket must be unavailable")
        for interface in Path("/sys/class/net").iterdir():
            if interface.name != "lo" and int((interface / "flags").read_text(), 16) & 1:
                raise AssertionError(f"network must be disabled: {interface.name}")
        self.paths = {}
        self.identities = {}
        for name in ("lock", "amiga", "preparation", "wheels"):
            record = self.inputs[name]
            keys = {"path", "bytes", "sha256"} | ({"audit"} if name != "lock" else set())
            if (set(record) != keys or type(record["bytes"]) is not int or record["bytes"] <= 0
                    or re.fullmatch(r"[0-9a-f]{64}", record["sha256"]) is None):
                raise AssertionError(f"invalid actual {name} input record")
            path = Path(record["path"])
            if (not path.is_absolute() or not path.is_file() or path.is_symlink()
                    or path.stat().st_size != record["bytes"] or digest_file(path) != record["sha256"]):
                raise AssertionError(f"actual {name} input bytes differ from their recorded identity")
            self.paths[name] = path
            self.identities[path] = record["sha256"]
            if name != "lock":
                audit = Path(record["audit"])
                if not audit.is_absolute():
                    raise AssertionError("support audit paths must be absolute")
                self.paths[name + "_audit"] = audit
                self.identities[audit] = digest_file(audit)
        self.lock = read_json(self.paths["lock"])
        for prefix in self.lock["output"]["builder_prefixes"]:
            if Path(prefix).exists():
                raise AssertionError(f"builder prefix must be unavailable: {prefix}")
        ndk = Path(self.inputs["ndk_archive"])
        ndk_record = self.lock["sources"][self.lock["preparation"]["archive"]]
        if (not ndk.is_absolute() or not ndk.is_file() or ndk.stat().st_size != ndk_record["bytes"]
                or digest_file(ndk) != ndk_record["sha256"]):
            raise AssertionError("actual NDK bytes differ from the lock")
        self.paths["ndk"] = ndk
        self.identities[ndk] = ndk_record["sha256"]
        self.inventories = {
            name: audit_inventory(self.paths[name], self.paths[name + "_audit"],
                                  self.inputs["lock"]["sha256"])
            for name in ("amiga", "preparation", "wheels")
        }
        if runtime:
            observed = [Path(__file__).resolve(), *self.paths.values()]
            for path in observed:
                for parent in path.parents:
                    if (parent / ".git").exists() or (parent / "compiler/Cargo.toml").exists():
                        raise AssertionError(f"runtime proof cannot use a source checkout: {parent}")
        work = Path(self.inputs["work_root"])
        if (not work.is_absolute() or work.is_symlink() or not work.is_dir()
                or work.stat().st_uid != os.getuid() or any(work.iterdir())):
            raise AssertionError("support work_root must be an initially empty real directory")
        self.root = Path(tempfile.mkdtemp(prefix="core-actual-", dir=work))
        self.keep_reason = ""
        home = self.root / "home"
        try:
            home.mkdir()
        except OSError:
            self.cleanup()
            raise
        self.environment = {
            "PATH": "/usr/bin:/bin", "HOME": str(home), "TMPDIR": str(self.root),
            "CODEOPS_TMPDIR": str(self.root), "PYTHONDONTWRITEBYTECODE": "1",
            "LANG": "C.UTF-8",
        }
        self.count = 0
        print(json.dumps({
            "ubuntu": actual, "python": sys.version, "uid": os.getuid(),
            "input_sha256": {str(path): identity for path, identity in self.identities.items()},
            "mounts": Path("/proc/self/mountinfo").read_text(),
        }), flush=True)

    def cleanup(self) -> None:
        """Remove only owned work, unless it still holds an unrestored user input."""
        if self.keep_reason:
            print(f"retained {self.root}: {self.keep_reason}", flush=True)
            return
        shutil.rmtree(self.root)

    def run(self, command: list, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
        """Capture bounded real processes as owned files rather than unbounded pipes."""
        self.count += 1
        out = self.root / f"command-{self.count}.stdout"
        err = self.root / f"command-{self.count}.stderr"
        with out.open("wb") as stdout, err.open("wb") as stderr:
            result = subprocess.run(
                [str(value) for value in command], cwd=cwd or self.root,
                env=self.environment, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr,
                timeout=300, preexec_fn=_limit_files)
        for path in (out, err):
            if path.stat().st_size > 2 * 1024 * 1024:
                raise AssertionError(f"verification output exceeded its bounded read budget: {path.name}")
        result.stdout = out.read_text(encoding="utf-8", errors="replace")
        result.stderr = err.read_text(encoding="utf-8", errors="replace")
        print(json.dumps({"argv": [str(value) for value in command],
                          "exit": result.returncode,
                          "stdout": result.stdout, "stderr": result.stderr}), flush=True)
        return result

    def unchanged(self) -> None:
        """All provided input identities must survive success and failure."""
        for path, identity in self.identities.items():
            if digest_file(path) != identity:
                raise AssertionError(f"producer changed an actual input: {path}")


def sys_abi() -> str:
    """Report the actual interpreter cohort without accepting a fake version banner."""
    if sys.implementation.name != "cpython":
        raise AssertionError("actual CPython is required")
    return f"cp{sys.version_info.major}{sys.version_info.minor}"


def archive_file(archive: Path, name: str) -> bytes:
    """Read one independently accounted metadata file from a proved archive."""
    with tarfile.open(archive) as source:
        member = source.getmember(name)
        if not member.isfile() or member.size > 16 * 1024 * 1024:
            raise AssertionError(f"bounded ordinary metadata required: {name}")
        return source.extractfile(member).read()


def assembled_layout(candidate: Path, core_archive: Path, inputs: ActualInputs) -> dict:
    """Verify real core bytes, exact prepared SDK and paired descriptor/venv receipts."""
    core = inspect_core(core_archive)
    for relative in PAYLOAD:
        path = candidate / relative
        if path.is_symlink() or not path.is_file() or digest_file(path) != core["files"][relative]["sha256"]:
            raise AssertionError(f"assembled core input changed: {relative}")
    for name in BINARIES:
        elf_x64(candidate / "bin" / name)
    if (candidate / "core.json").read_bytes() != archive_file(core_archive, "core.json"):
        raise AssertionError("assembled core.json was rewritten")
    for relative in ("bin", "sdk", "sdk/amiga", "sdk/vamos"):
        path = candidate / relative
        if path.is_symlink() or not path.is_dir() or not path.resolve().is_relative_to(candidate.resolve()):
            raise AssertionError(f"assembled directory redirects elsewhere: {relative}")
    descriptor = read_json(candidate / "installation.json")
    preparation = read_json_bytes(archive_file(inputs.paths["preparation"], "preparation.json"))
    inventory = inputs.lock["preparation"]["inventory"]
    inventory_hash = hashlib.sha256(json_bytes(inventory)).hexdigest()
    expected = {
        "schema": 1, "core_version": core["core_version"],
        "support_lock_sha256": inputs.inputs["lock"]["sha256"],
        "ndk_sha256": inputs.identities[inputs.paths["ndk"]],
        "prepared_inventory_sha256": inventory_hash,
        "python_abi": inputs.inputs["python_abi"],
    }
    if descriptor != expected or type(descriptor.get("schema")) is not int:
        raise AssertionError("assembled six-field native descriptor differs from verified inputs")
    receipt = read_json(candidate / "sdk/amiga/.ndk-prepared.json")
    if (receipt != {"schema": 1, "lock_sha256": expected["support_lock_sha256"],
                   "ndk_sha256": expected["ndk_sha256"], "inventory_sha256": inventory_hash}
            or type(receipt.get("schema")) is not int):
        raise AssertionError("assembled NDK receipt differs from verified preparation")
    if preparation["inventory"] != inventory:
        raise AssertionError("proved preparation and lock inventory differ")
    expected_sdk = dict(inputs.inventories["amiga"])
    expected_sdk.update({name: {"type": "file", "sha256": identity}
                         for name, identity in inventory.items()})
    actual_sdk = tree(candidate / "sdk/amiga")
    del actual_sdk[".ndk-prepared.json"]
    if actual_sdk != expected_sdk:
        raise AssertionError("assembled SDK bytes/links differ from base plus prepared inventory")
    wheel_bytes = archive_file(inputs.paths["wheels"], "wheels.json")
    if (candidate / "sdk/vamos/wheels.json").read_bytes() != wheel_bytes:
        raise AssertionError("copied wheel receipt differs from the proved wheel manifest")
    wheels = read_json_bytes(wheel_bytes)
    if (wheels["abi"] != expected["python_abi"]
            or wheels["lock_sha256"] != expected["support_lock_sha256"]
            or not {"amitools", "machine68k", "greenlet", "pysdl2", "pip"} <= set(wheels["packages"])):
        raise AssertionError("assembled wheel cohort/support pairing differs")
    python = candidate / "sdk/vamos/bin/python"
    if not python.is_file() or not os.access(python, os.X_OK):
        raise AssertionError("missing actual paired venv interpreter")
    read_cfg = candidate / "sdk/vamos/pyvenv.cfg"
    if read_cfg.is_symlink() or not read_cfg.is_file():
        raise AssertionError("paired venv needs an ordinary pyvenv.cfg")
    return core
