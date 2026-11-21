"""Owned filesystem/process fixtures for the fixed real support-archive proof."""

from __future__ import annotations

import hashlib
import json
import os
import platform
import resource
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path, PurePosixPath

from support_fixtures import digest, json_bytes


CASES = ("hello", "file-copier", "string-parser", "buffer-container", "volatile-mmio")
"""Fixed portable programs; the hardware-register example is Amiga-only."""
PROFILES = ("host", "hosted-amiga", "hosted-amiga-13")
"""Profiles exercised independently before and after relocation."""


def read_json(path: Path) -> dict:
    """Read candidate metadata without silently accepting repeated object keys."""
    def unique(pairs):
        """Keep duplicate metadata keys from changing the selected value."""
        result = {}
        for key, value in pairs:
            if key in result:
                raise AssertionError(f"{path}: duplicate key {key}")
            result[key] = value
        return result
    value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique)
    if not isinstance(value, dict):
        raise AssertionError(f"{path}: expected a JSON object")
    return value


def file_hash(path: Path) -> str:
    """Hash potentially large archives without loading them into memory."""
    result = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(block)
    return result.hexdigest()


def tree(root: Path) -> dict:
    """Describe actual regular files and links without following directory aliases."""
    result = {}
    for directory, folders, files in os.walk(root, followlinks=False):
        for name in folders + files:
            path = Path(directory) / name
            relative = path.relative_to(root).as_posix()
            if path.is_symlink():
                result[relative] = {"type": "symlink", "target": os.readlink(path)}
            elif path.is_file():
                result[relative] = {"type": "file", "sha256": file_hash(path)}
            elif not path.is_dir():
                raise AssertionError(f"unsupported file type: {path}")
    return result


def contained(root: Path, relative: str) -> Path:
    """Resolve one candidate-relative file without accepting a path escape."""
    parts = relative.split("/")
    if (not relative or "\\" in relative or "!" in relative
            or any(part in ("", ".", "..") for part in parts)
            or any(ord(char) < 32 or ord(char) == 127 for char in relative)):
        raise AssertionError(f"unsafe relative path: {relative!r}")
    path = root / relative
    if not path.resolve().is_relative_to(root.resolve()):
        raise AssertionError(f"escaping relative path: {relative}")
    return path


def extract_verified(archive_path: Path, destination: Path) -> None:
    """Extract already hash-verified tar bytes using a fresh, link-safe test directory."""
    destination.mkdir()
    with tarfile.open(archive_path) as archive:
        members = archive.getmembers()
        paths = {}
        links = {member.name.rstrip("/") for member in members if member.issym()}
        for member in members:
            name = member.name.rstrip("/") if member.isdir() else member.name
            path = contained(destination, name)
            if name in paths or not (member.isfile() or member.isdir() or member.issym()):
                raise AssertionError(f"duplicate or unsupported archive member: {name}")
            if member.mode & 0o7000:
                raise AssertionError(f"unsafe archive mode: {name}")
            if any(parent.as_posix() in links for parent in PurePosixPath(name).parents):
                raise AssertionError(f"archive member has a symlink parent: {name}")
            if member.issym() and (os.path.isabs(member.linkname)
                                  or not (path.parent / member.linkname).resolve().is_relative_to(
                                      destination.resolve())):
                raise AssertionError(f"escaping archive link: {name}")
            paths[name] = path
        for member in members:
            path = paths[member.name.rstrip("/") if member.isdir() else member.name]
            path.parent.mkdir(parents=True, exist_ok=True)
            if member.isdir():
                path.mkdir(exist_ok=True)
            elif member.isfile():
                with archive.extractfile(member) as source, path.open("xb") as target:
                    shutil.copyfileobj(source, target)
                path.chmod(member.mode & 0o777)
        for member in members:
            if member.issym():
                paths[member.name].symlink_to(member.linkname)
        for member in members:
            if member.issym():
                try:
                    resolved = paths[member.name].resolve(strict=True)
                except (OSError, RuntimeError) as error:
                    raise AssertionError(f"invalid archive link: {member.name}") from error
                if not resolved.is_relative_to(destination.resolve()):
                    raise AssertionError(f"escaping archive link: {member.name}")


def _limit_output() -> None:
    """Bound each child-created log or compiler output to one GiB."""
    limits = [value for value in resource.getrlimit(resource.RLIMIT_FSIZE) if value >= 0]
    limit = min([1024 ** 3, *limits])
    resource.setrlimit(resource.RLIMIT_FSIZE, (limit, limit))


class ArtifactFixture:
    """Own one real candidate's extracted trees and bounded process evidence.

    ``inputs`` is the explicit acceptance record, ``lock`` its verified lock,
    ``root`` disposable work, and ``payloads``/``audits`` the verified archive
    trees/records. ``fixtures`` holds only emitted C and frozen goldens.
    ``environment`` removes inherited Python/loader settings. ``command_count``
    names owned logs; ``scratch`` must be cleaned by the unittest caller.
    """

    def __init__(self, manifest: Path) -> None:
        """Check actual isolation and required inputs before creating disposable work."""
        self.inputs = read_json(manifest)
        expected = {"schema", "ubuntu", "python_abi", "lock", "amiga", "preparation",
                    "wheels", "ndk_archive", "fixtures", "work_root"}
        if set(self.inputs) != expected or type(self.inputs["schema"]) is not int:
            raise AssertionError("acceptance.json: invalid fields or schema type")
        if self.inputs["schema"] != 1:
            raise AssertionError("acceptance.json: unsupported schema")
        cohort = (self.inputs["ubuntu"], self.inputs["python_abi"])
        if cohort not in (("22.04", "cp310"), ("24.04", "cp312")):
            raise AssertionError("acceptance.json: unsupported Ubuntu/Python cohort")
        actual = platform.freedesktop_os_release()
        if (os.getuid() == 0 or actual.get("ID") != "ubuntu"
                or actual.get("VERSION_ID") != cohort[0] or platform.machine() != "x86_64"
                or sys.implementation.name != "cpython"
                or f"cp{sys.version_info.major}{sys.version_info.minor}" != cohort[1]):
            raise AssertionError(f"actual non-root Ubuntu/Python host must match {cohort}")
        if Path("/opt/amiga").exists() or Path("/var/run/docker.sock").exists():
            raise AssertionError("original SDK and Docker socket must be unavailable")
        for interface in Path("/sys/class/net").iterdir():
            if interface.name != "lo" and int((interface / "flags").read_text().strip(), 16) & 1:
                raise AssertionError(f"network interface remains active: {interface.name}")
        for route in Path("/proc/net/route").read_text().splitlines()[1:]:
            if route.split()[0] != "lo":
                raise AssertionError("non-loopback IPv4 route remains available")
        for route in Path("/proc/net/ipv6_route").read_text().splitlines():
            if route.split()[-1] != "lo":
                raise AssertionError("non-loopback IPv6 route remains available")
        lock_path = self._record("lock")
        self.lock = read_json(lock_path)
        for prefix in self.lock["output"]["builder_prefixes"]:
            if Path(prefix).exists():
                raise AssertionError(f"original builder prefix remains accessible: {prefix}")
        self.fixtures = self._path(self.inputs["fixtures"])
        paths = [Path(__file__).resolve(), self.fixtures, lock_path]
        paths.extend(self._path(self.inputs[name]["path"]) for name in ("amiga", "preparation", "wheels"))
        for path in paths:
            for parent in path.parents:
                if (parent / ".git").exists() or (parent / "compiler/Cargo.toml").exists():
                    raise AssertionError(f"source checkout remains accessible: {parent}")
        for case in CASES:
            golden = self.fixtures / "goldens" / (case + ".txt")
            if not golden.is_file():
                raise AssertionError(f"missing fixed golden: {golden}")
            for profile in PROFILES:
                if profile == "host" and case == "volatile-mmio":
                    continue
                unit = self.fixtures / "units" / profile / (case + ".c")
                if not unit.is_file():
                    raise AssertionError(f"missing fixed emitted unit: {unit}")
        if (self.fixtures / "goldens/hello.txt").read_bytes() != b"Hello, Amiga, from Clorraine!\n":
            raise AssertionError("hello golden is not the fixed hello output")
        work = self._path(self.inputs["work_root"])
        if not work.is_dir() or work.is_symlink() or any(work.iterdir()):
            raise AssertionError("work_root must be an empty, real, writable directory")
        evidence = {
            "ubuntu": actual, "python": sys.version, "uid": os.getuid(),
            "interfaces": [path.name for path in Path("/sys/class/net").iterdir()],
            "mounts": Path("/proc/self/mountinfo").read_text(),
            "fixture_hashes": {str(path.relative_to(self.fixtures)): file_hash(path)
                              for path in self.fixtures.rglob("*") if path.is_file()}}
        print(json.dumps(evidence), flush=True)
        self.scratch = tempfile.TemporaryDirectory(prefix="candidate-", dir=work)
        self.root = Path(self.scratch.name)
        self.environment = {key: value for key, value in os.environ.items()
                            if not key.startswith(("PYTHON", "LD_", "PIP_"))}
        self.environment.update(PATH="/usr/bin:/bin", TMPDIR=str(self.root),
                                CODEOPS_TMPDIR=str(self.root), PYTHONDONTWRITEBYTECODE="1",
                                LC_ALL="C")
        self.command_count = 0
        self.payloads, self.audits = {}, {}

    @staticmethod
    def _path(value: str) -> Path:
        """Reject missing or relative explicit input paths."""
        if not isinstance(value, str) or not Path(value).is_absolute() or not Path(value).exists():
            raise AssertionError(f"required absolute input is unavailable: {value!r}")
        return Path(value)

    def _record(self, name: str) -> Path:
        """Verify the exact bytes of a named required candidate artifact."""
        record = self.inputs[name]
        keys = {"path", "bytes", "sha256"} | ({"audit"} if name != "lock" else set())
        if set(record) != keys or type(record["bytes"]) is not int or record["bytes"] <= 0:
            raise AssertionError(f"{name}: invalid artifact record")
        path = self._path(record["path"])
        if (not path.is_file() or path.stat().st_size != record["bytes"]
                or file_hash(path) != record["sha256"]):
            raise AssertionError(f"{name}: candidate length/hash mismatch")
        return path

    def run(self, arguments: list, cwd: Path | None = None, check: bool = True,
            environment: dict | None = None) -> subprocess.CompletedProcess:
        """Record bounded real subprocess evidence; never replace an unavailable tool."""
        arguments = [str(value) for value in arguments]
        self.command_count += 1
        stem = self.root / f"command-{self.command_count}"
        with stem.with_suffix(".out").open("wb") as out, stem.with_suffix(".err").open("wb") as err:
            result = subprocess.run(
                arguments, cwd=cwd or self.root, env=environment or self.environment,
                stdin=subprocess.DEVNULL, stdout=out, stderr=err, timeout=300,
                preexec_fn=_limit_output)
        result.stdout = stem.with_suffix(".out").read_bytes()
        result.stderr = stem.with_suffix(".err").read_bytes()
        print(json.dumps({"argv": arguments, "exit": result.returncode,
                          "stdout": result.stdout.decode(errors="replace"),
                          "stderr": result.stderr.decode(errors="replace")}), flush=True)
        if check and result.returncode:
            raise AssertionError(f"{arguments}: exit {result.returncode}\n{result.stderr.decode(errors='replace')}")
        return result

    def stage(self) -> None:
        """Verify all candidate archives, safely extract them, then run their public auditor."""
        distribution = Path(__file__).resolve().parents[1]
        packager = distribution / "package_support.py"
        if not packager.is_file():
            raise AssertionError(f"missing copied public auditor: {packager}")
        self.run(["dpkg-query", "-W", "-f=${binary:Package}\t${Version}\n"])
        for name in ("amiga", "preparation", "wheels"):
            archive = self._record(name)
            audit = self._path(self.inputs[name]["audit"])
            target = self.root / name
            extract_verified(archive, target)
            self.run([sys.executable, packager, "--lock", self.inputs["lock"]["path"],
                      "--payload", target, "--audit", audit,
                      "--output", self.root / (name + "-audited.tar")])
            self.payloads[name], self.audits[name] = target, read_json(audit)
        ndk = self._path(self.inputs["ndk_archive"])
        record = self.lock["sources"][self.lock["preparation"]["archive"]]
        if ndk.stat().st_size != record["bytes"] or file_hash(ndk) != record["sha256"]:
            raise AssertionError("NDK archive does not match the verified lock")
        print(json.dumps({"lock_sha256": self.inputs["lock"]["sha256"],
                          "ndk_sha256": record["sha256"],
                          "archives": {name: self.inputs[name]["sha256"]
                                       for name in ("amiga", "preparation", "wheels")}}))

    def assembled(self, name: str) -> Path:
        """Copy verified payloads into a fresh root whose complete lifetime this test owns."""
        root = self.root / name
        root.mkdir()
        for source, destination in (("amiga", "sdk"), ("preparation", "preparation"), ("wheels", "wheelset")):
            shutil.copytree(self.payloads[source], root / destination, symlinks=True)
        return root

    def prepare(self, root: Path, destination: Path | None = None) -> None:
        """Run the actual shipped transformation and verify the full resulting SDK tree."""
        destination = destination or root / "sdk"
        metadata = read_json(root / "preparation/preparation.json")
        if (metadata["lock_sha256"] != self.inputs["lock"]["sha256"]
                or metadata["inventory"] != self.lock["preparation"]["inventory"]):
            raise AssertionError("preparation metadata is not bound to this lock and inventory")
        if not metadata["notices"]:
            raise AssertionError("preparation must retain its required notices")
        if tree(destination) != metadata["base"]:
            raise AssertionError("unprepared SDK does not match preparation baseline")
        self.run([sys.executable, root / "preparation/prepare_ndk.py", "--archive",
                  self.inputs["ndk_archive"], "--destination", destination])
        expected = dict(metadata["base"])
        expected.update({path: {"type": "file", "sha256": value}
                         for path, value in metadata["inventory"].items()})
        actual = tree(destination)
        marker = destination / ".ndk-prepared.json"
        actual.pop(".ndk-prepared.json", None)
        if actual != expected:
            raise AssertionError("actual prepared SDK differs from the complete reference tree")
        marker_expected = {
            "schema": 1, "lock_sha256": self.inputs["lock"]["sha256"],
            "ndk_sha256": self.lock["sources"][self.lock["preparation"]["archive"]]["sha256"],
            "inventory_sha256": digest(json_bytes(metadata["inventory"]))}
        if read_json(marker) != marker_expected:
            raise AssertionError("preparation completion metadata differs from actual verified inputs")
        for notice in metadata["notices"]:
            if notice not in metadata["inventory"] or not contained(destination, notice).is_file():
                raise AssertionError(f"required prepared notice is absent: {notice}")
