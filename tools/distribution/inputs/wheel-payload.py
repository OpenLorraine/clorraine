# Historical reference only: the native support pipeline and its
# prepare_ndk.py guard were removed. This file is retained as provenance and is
# not runnable.
"""Produce fixed-source vamos wheels using the selected preinstalled build environment."""

from __future__ import annotations

import argparse
import base64
import csv
from email.parser import BytesParser
import hashlib
import importlib.metadata
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile

from packaging.requirements import Requirement


_ARCHIVES = {
    "machine68k": "a58ae3f0f5f32639296e7eaf564f2aa96852462078e35b0c6d8e714d2510a9cb",
    "greenlet": "8e67c43bdfc88d5fee6db0d3e40175b362fc95fb85f0412d233b9b203c53a575",
    "pysdl2": "48c6ef01a4eb123db5f7e46e1a1b565675755b07e615f3fe20a623c94735b52b",
    "pip": "f6ad667e89a1fe78046c8f13232b247200f5258d7828f3f7883d660878e0813f",
    "setuptools": "583b361c8da8de57403743e756609670de6fb2345920e36dc5c2d914c319c945",
    "setuptools-scm": "3d555e92b75dacd037d32bafdf94f97af51ea29ae8c7b234cf94b7a5bd242a63",
    "cython": "b988bb297ce76c671e28c97d017b95411010f7c77fa6623dd0bb47eed1aee1bc",
    "wheel": "661e1abd9198507b1409a20c02106d9670b2576e916d58f520316666abca6729",
    "packaging": "c228a6dc5e932d346bc5739379109d49e8853dd8223571c7c5b55260edc0b97f",
    "tomli": "d46d457a85337051c36524bc5349dd91b1877838e2979ac5ced3e710ed8a60ed",
    "flit-core": "18f63100d6f94385c6ed57a72073443e1a71a4acb4339491615d0f16d6ff01b2",
    "cpython312": "b5796915dc45ff745cfb4e990a7e7c93d6f546328a102ce31448d69ebc37575c",
}
"""Exact original Python-source closure; none of these archives is fetched during a build."""

_BACKENDS = {
    "pip": "26.2.1", "setuptools": "77.0.3", "setuptools-scm": "8.3.1",
    "cython": "3.0.12", "wheel": "0.45.1", "packaging": "24.2",
    "tomli": "2.0.2", "flit-core": "3.12.0",
}
"""Build-only tools already measured in both prepared interpreter environments."""

_AMITOOLS_COMMIT = "d0b9727da3069328231784d2d5fdcf44816c95b1"
"""Pinned source identity associated with the observed SDK distribution version."""
_AMITOOLS_VERSION = "0.8.1.dev274+gd0b9727da"
"""The measured SDK version, fixed independently of mutable or absent local Git tags."""
_SOURCE_DATE_EPOCH = 1790576253
"""Timestamp from the authenticated amitools commit, used for deterministic archive metadata."""

_INLINE_NOTICES = {
    "amitools": ("README.md",),
    "machine68k": ("README.md", "src/cpu.c", "src/musashi/m68kcpu.c",
                   "src/musashi/m68kcpu.h"),
    "pysdl2": ("doc/copying.rst",),
}
"""Original files carrying terms that a licence-filename search cannot discover."""


def _fail(identity: str, message: str) -> None:
    """Identify a source/backend/build failure without attempting network repair."""
    raise ValueError(f"{identity}: {message}")


def _hash(path: Path) -> str:
    """Hash ordinary source/wheel bytes without following a file symlink."""
    if path.is_symlink() or not path.is_file():
        _fail(str(path), "expected an ordinary file")
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _normalise(name: str) -> str:
    """Use Python distribution-name normalisation for manifests and dependency comparisons."""
    return re.sub(r"[-_.]+", "-", name).lower()


def _run(arguments: list[str], cwd: Path, environment: dict[str, str]) -> None:
    """Execute one literal local build command, propagating any failure."""
    result = subprocess.run(arguments, cwd=cwd, env=environment, timeout=1800, check=False)
    if result.returncode:
        _fail(cwd.name, f"local build command failed (exit {result.returncode})")


def _members_validator(sources: Path):
    """Reuse the public pure member guard from the authenticated preparation input."""
    path = sources / "inputs/prepare_ndk.py"
    spec = importlib.util.spec_from_file_location("_wheel_archive_guard", path)
    if spec is None or spec.loader is None:
        _fail("prepare_ndk.py", "missing staged member validator")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.validate_archive_members


def _unpack(path: Path, output: Path, validate_members) -> Path:
    """Validate all tar members before copying only safe source files and aliases."""
    with tarfile.open(path, "r:*") as archive:
        entries = archive.getmembers()
        members = []
        for entry in entries:
            kind = ("file" if entry.isfile() else "directory" if entry.isdir()
                    else "symlink" if entry.issym() else "unsupported")
            if entry.mode & 0o7000:
                _fail(entry.name, "special source permissions")
            members.append({"path": entry.name.rstrip("/") if entry.isdir() else entry.name,
                            "type": kind, "target": entry.linkname if entry.issym() else None})
        validate_members(members)
        if not entries or sum(entry.size for entry in entries) > 1024**3:
            _fail(path.name, "empty or oversized source archive")
        output.mkdir()
        for entry, member in zip(entries, members):
            destination = output / member["path"]
            if entry.isdir():
                destination.mkdir(parents=True, exist_ok=True)
            elif entry.isfile():
                destination.parent.mkdir(parents=True, exist_ok=True)
                with archive.extractfile(entry) as source, destination.open("xb") as target:
                    shutil.copyfileobj(source, target)
                destination.chmod(0o755 if entry.mode & 0o111 else 0o644)
        # Aliases are created only after every regular source file exists.
        for entry, member in zip(entries, members):
            if entry.issym():
                destination = output / member["path"]
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.symlink_to(entry.linkname)
    roots = list(output.iterdir())
    if len(roots) != 1 or not roots[0].is_dir() or roots[0].is_symlink():
        _fail(path.name, "expected one ordinary source-project root")
    return roots[0]


def _source_snapshot(project: Path, output: Path) -> None:
    """Archive the raw tracked checkout without private Git storage or build products."""
    with tarfile.open(output, "w", format=tarfile.PAX_FORMAT) as archive:
        for directory, folders, files in os.walk(project, followlinks=False):
            if Path(directory) == project and ".git" in folders:
                folders.remove(".git")
            folders.sort()
            for name in sorted(folders + files):
                path = Path(directory) / name
                relative = path.relative_to(project).as_posix()
                info = archive.gettarinfo(path, arcname="amitools/" + relative)
                info.uid = info.gid = 0
                info.uname = info.gname = ""
                info.mtime = _SOURCE_DATE_EPOCH
                info.mode &= 0o777
                if info.isfile():
                    with path.open("rb") as stream:
                        archive.addfile(info, stream)
                elif info.isdir() or info.issym():
                    archive.addfile(info)
                else:
                    _fail(relative, "unsupported corresponding-source entry")


def _notices(source: Path, destination: Path, validate_members) -> None:
    """Retain original notice-named source files; final compliance audit remains separate."""
    if source.is_dir():
        paths = [(path.relative_to(source).as_posix(), path.read_bytes())
                 for path in source.rglob("*") if path.is_file() and not path.is_symlink()
                 and ".git" not in path.relative_to(source).parts
                 and (path.relative_to(source).as_posix() in _INLINE_NOTICES.get(source.name, ())
                      or re.match(r"(?:LICEN[CS]E|COPYING|NOTICE|COPYRIGHT)(?:$|[._-])",
                                  path.name.upper()))]
    else:
        with tarfile.open(source, "r:*") as archive:
            extra = _INLINE_NOTICES.get(source.name, ())
            entries = []
            found = set()
            for entry in archive.getmembers():
                relative = entry.name.partition("/")[2]
                if entry.isfile() and (
                        relative in extra or re.match(
                            r"(?:LICEN[CS]E|COPYING|NOTICE|COPYRIGHT)(?:$|[._-])",
                            Path(entry.name).name.upper())):
                    entries.append(entry)
                    found.add(relative)
            if set(extra) - found:
                _fail(source.name, f"missing inline notice files: {sorted(set(extra) - found)!r}")
            validate_members([{"path": entry.name, "type": "file", "target": None}
                              for entry in entries])
            if any(entry.size > 16 * 1024 * 1024 for entry in entries):
                _fail(source.name, "oversized source notice")
            paths = [(entry.name, archive.extractfile(entry).read()) for entry in entries]
    if not paths:
        _fail(source.name, "source notices are absent")
    for name, content in paths:
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content)


def _wheel_record(path: Path, source: str) -> tuple[str, dict, list[str]]:
    """Read actual built wheel metadata, tags and runtime requirements."""
    with zipfile.ZipFile(path) as archive:
        metadata_names = [name for name in archive.namelist() if name.endswith(".dist-info/METADATA")]
        wheel_names = [name for name in archive.namelist() if name.endswith(".dist-info/WHEEL")]
        if len(metadata_names) != 1 or len(wheel_names) != 1:
            _fail(path.name, "ambiguous wheel metadata")
        metadata = BytesParser().parsebytes(archive.read(metadata_names[0]))
        wheel = BytesParser().parsebytes(archive.read(wheel_names[0]))
    name = _normalise(metadata["Name"])
    record = {
        "version": metadata["Version"], "source": source,
        "purpose": "bootstrap" if name == "pip" else "runtime",
        "filename": "wheels/" + path.name, "bytes": path.stat().st_size,
        "sha256": _hash(path), "tags": sorted(wheel.get_all("Tag", [])),
    }
    return name, record, metadata.get_all("Requires-Dist", [])

def select_greenlet_runtime(path: Path) -> None:
    """Remove only the pinned Greenlet wheel's optional test subtree in place.

    Runtime member bytes, ZIP metadata and original licence files are retained.
    RECORD is regenerated with SHA-256 hashes/sizes for the surviving files.
    A private sibling is atomically substituted only after successful writing;
    identity, missing-runtime and signed-RECORD failures leave the input intact.
    This runs during maintainer builds, never during user installation.
    """
    record_name = "greenlet-3.5.6.dist-info/RECORD"
    metadata_name = "greenlet-3.5.6.dist-info/METADATA"
    with zipfile.ZipFile(path) as original:
        members = original.infolist()
        names = [member.filename for member in members]
        if (len(names) != len(set(names)) or metadata_name not in names
                or record_name not in names
                or sum(member.file_size for member in members) > 512 * 1024 * 1024):
            _fail("greenlet", "unexpected or oversized wheel layout")
        metadata = BytesParser().parsebytes(original.read(metadata_name))
        if metadata["Name"] != "greenlet" or metadata["Version"] != "3.5.6":
            _fail("greenlet", "runtime selection requires the pinned distribution")
        if any(name in names for name in (record_name + ".jws", record_name + ".p7s")):
            _fail("greenlet", "cannot invalidate a wheel RECORD signature")
        if not any(name.startswith("greenlet/_greenlet.") and name.endswith(".so") for name in names):
            _fail("greenlet", "missing native runtime extension")
        if not any(name.startswith("greenlet/tests/") for name in names):
            _fail("greenlet", "missing expected optional test subtree")
        descriptor, temporary_name = tempfile.mkstemp(prefix=".greenlet-runtime-", dir=path.parent)
        temporary = Path(temporary_name)
        try:
            with os.fdopen(descriptor, "w+b") as stream, zipfile.ZipFile(stream, "w") as selected:
                selected.comment = original.comment
                rows = []
                for member in members:
                    if member.filename.startswith("greenlet/tests/") or member.filename == record_name:
                        continue
                    data = original.read(member)
                    selected.writestr(member, data)
                    if not member.is_dir():
                        digest = base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=")
                        rows.append((member.filename, "sha256=" + digest.decode("ascii"), str(len(data))))
                rows.append((record_name, "", ""))
                content = io.StringIO(newline="")
                csv.writer(content, lineterminator="\n").writerows(rows)
                selected.writestr(original.getinfo(record_name), content.getvalue().encode("utf-8"))
            temporary.chmod(path.stat().st_mode & 0o777)
            os.replace(temporary, path)
        finally:
            if temporary.exists():
                temporary.unlink()


def produce(abi: str, sources: Path, work: Path) -> dict:
    """Build the fixed wheel set from already verified sources and return package records.

    The public builder verifies the complete source lock before this recipe.
    Manual authoring proof must verify its staged inputs separately. This
    function does not invent a lock hash or write a release-bound manifest.
    """
    expected_python = (3, 10) if abi == "cp310" else (3, 12) if abi == "cp312" else None
    if expected_python is None or sys.version_info[:2] != expected_python:
        _fail(abi, "wrong prepared interpreter")
    for name, version in _BACKENDS.items():
        if importlib.metadata.version(name) != version:
            _fail(name, "prepared build backend version differs")
    for name, digest in _ARCHIVES.items():
        if _hash(sources / "archives" / name) != digest:
            _fail(name, "fixed source archive hash differs")
    environment = {
        "PATH": str(Path(sys.executable).parent) + ":/usr/bin:/bin",
        "LC_ALL": "C", "PYTHONHASHSEED": "0", "PYTHONNOUSERSITE": "1",
        "SOURCE_DATE_EPOCH": str(_SOURCE_DATE_EPOCH), "CFLAGS": "-O2", "CXXFLAGS": "-O2",
        "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null",
        "GIT_TERMINAL_PROMPT": "0", "GIT_OPTIONAL_LOCKS": "0",
        "GIT_NO_REPLACE_OBJECTS": "1", "GIT_ATTR_NOSYSTEM": "1",
        "HOME": str(work / "home"), "TMPDIR": str(work / "tmp"),
    }
    for name in ("home", "tmp", "projects"):
        (work / name).mkdir()
    checkout = sources / "checkouts/amitools"
    result = subprocess.run(
        ["git", "-C", str(checkout), "-c", "core.fsmonitor=false",
         "-c", "core.hooksPath=/dev/null", "rev-parse", "HEAD"],
        env=environment, capture_output=True, text=True, timeout=30, check=True)
    if result.stdout.strip() != _AMITOOLS_COMMIT:
        _fail("amitools", "source commit differs from the fixed SDK cohort")
    validate_members = _members_validator(sources)
    payload = work / "payload"
    wheels = payload / "wheels"
    wheels.mkdir(parents=True)
    (payload / "sources").mkdir()
    for name in sorted(_ARCHIVES):
        archive = sources / "archives" / name
        suffix = ".tar.xz" if name == "cpython312" else ".tar.gz"
        shutil.copy2(archive, payload / "sources" / (name + suffix))
        _notices(archive, payload / "notices" / name, validate_members)
    project = work / "projects/amitools"
    shutil.copytree(checkout, project, symlinks=True)
    _source_snapshot(project, payload / "sources/amitools.tar")
    _notices(project, payload / "notices/amitools", validate_members)
    shutil.copy2(Path(__file__), payload / "sources/wheel-payload.py")
    packages, requirements = {}, {}
    for name in ("amitools", "machine68k", "greenlet", "pysdl2", "pip"):
        if name != "amitools":
            project = _unpack(sources / "archives" / name, work / "projects" / name,
                              validate_members)
        current = dict(environment)
        if name == "amitools":
            # The observed SDK version is preserved without depending on
            # mutable tags or the acquisition checkout's history depth.
            current["SETUPTOOLS_SCM_PRETEND_VERSION_FOR_AMITOOLS"] = _AMITOOLS_VERSION
        before = set(wheels.iterdir())
        _run([sys.executable, "-I", "-m", "pip", "--isolated", "--no-cache-dir", "wheel",
              "--no-index", "--no-deps", "--no-build-isolation",
              "--wheel-dir", str(wheels), str(project)], project, current)
        created = set(wheels.iterdir()) - before
        if len(created) != 1:
            _fail(name, "expected exactly one newly built wheel")
        path = created.pop()
        if name == "greenlet":
            select_greenlet_runtime(path)
        actual_name, record, dependencies = _wheel_record(path, name)
        if actual_name != name or actual_name in packages:
            _fail(name, "built wheel has a different or duplicate distribution identity")
        packages[name], requirements[name] = record, dependencies
    for name, dependencies in requirements.items():
        for text in dependencies:
            dependency = Requirement(text)
            extras = ("", "vamos") if name == "amitools" else ("",)
            if dependency.marker is not None and not any(
                    dependency.marker.evaluate({"extra": extra}) for extra in extras):
                continue
            target = _normalise(dependency.name)
            if target not in packages:
                _fail(name, f"undeclared runtime dependency {target!r}")
            if not dependency.specifier.contains(packages[target]["version"], prereleases=True):
                _fail(name, f"unsatisfied runtime dependency {text!r}")
    return packages


def main(arguments: list[str] | None = None) -> int:
    """Write a real lock-bound wheel manifest only when the verified caller supplies its digest."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--abi", choices=("cp310", "cp312"), required=True)
    args = parser.parse_args(arguments)
    try:
        if os.getuid() == 0 or Path.cwd() != Path("/work"):
            _fail("wheel-payload", "require a non-root builder in /work")
        lock_digest = os.environ.get("CLORRAINE_SUPPORT_LOCK_SHA256", "")
        if re.fullmatch(r"[0-9a-f]{64}", lock_digest) is None:
            _fail("lock_sha256", "missing verified caller lock digest")
        packages = produce(args.abi, Path("/sources"), Path("/work"))
        metadata = {"schema": 1, "lock_sha256": lock_digest,
                    "abi": args.abi, "packages": packages}
        (Path("/work/payload") / "wheels.json").write_text(
            json.dumps(metadata, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"wheel-payload: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
