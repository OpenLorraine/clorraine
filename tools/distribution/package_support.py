#!/usr/bin/env python3
"""Audit a complete support payload, then publish a deterministic tar without replacement."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile

from support import SupportError, load_lock, verify_sources
from support_audit import audit_payload, load_audit
from support_build import inventory, verify_wheels
from support_native import inspect_payload, safe_relative


def _hash(path: Path) -> str:
    """Hash one actual ordinary data file for generated evidence."""
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def _source_archive(source: Path, output: Path, identity: str) -> None:
    """Preserve matching source bytes with deterministic metadata, not private Git storage.

    Only selected pthread code is conveyed from AROS. Prebuilt Amiga libraries
    are not source, and the inherited libnix pipe implementation is supplied as
    original-plus-patch rather than a complete modified source file.
    """
    with output.open("xb") as stream, gzip.GzipFile(
            fileobj=stream, mode="wb", filename="", mtime=0) as compressed, tarfile.open(
                fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as archive:
        for directory, folders, files in os.walk(source, followlinks=False):
            if Path(directory) == source:
                folders[:] = [name for name in folders if name != ".git"
                              and (identity != "aros-stuff" or name == "pthreads")]
            folders.sort()
            for name in sorted(folders + files):
                path = Path(directory) / name
                relative = path.relative_to(source).as_posix()
                if identity == "aros-stuff" and not relative.startswith("pthreads/"):
                    continue
                if identity == "amiga-gcc" and relative.startswith("lib/") and path.suffix == ".a":
                    continue
                if identity == "libnix" and relative == "sources/nix/extra/pipeoc.c":
                    continue
                if path.is_dir() and not path.is_symlink():
                    continue
                info = archive.gettarinfo(path, arcname=identity + "/" + relative)
                info.uid = info.gid = info.mtime = 0
                info.uname = info.gname = ""
                info.mode = 0o755 if info.mode & stat.S_IXUSR else 0o644
                if info.islnk():
                    info.type = tarfile.REGTYPE
                    info.linkname = ""
                    info.size = path.stat().st_size
                if info.isfile():
                    with path.open("rb") as content:
                        archive.addfile(info, content)
                elif info.issym():
                    archive.addfile(info)
                else:
                    raise SupportError(f"{identity}/{relative}: unsupported source entry")


def _sdk_origin(name: str) -> str:
    """Map each reviewed install producer, failing rather than guessing at unknown paths."""
    if name.startswith("bin/"):
        program = name[4:]
        if program in {"lha", "sfdc", "fd2sfd"}:
            return program
        if program.startswith("m68k-amigaos-gcc") or program == "m68k-amigaos-cpp":
            return "gcc"
        if program.startswith("m68k-amigaos-"):
            return "binutils"
    for prefix, origin in (
            ("lib/gcc/", "gcc"), ("libexec/gcc/", "gcc"),
            ("m68k-amigaos/bin/", "binutils"),
            ("m68k-amigaos/libnix/", "libnix"),
            ("m68k-amigaos/sys-include/", "newlib-cygwin"),
            ("m68k-amigaos/include/", "aros-stuff"),
            ("share/fd2sfd/", "fd2sfd")):
        if name.startswith(prefix):
            return origin
    if name.startswith("m68k-amigaos/lib/"):
        if "/ldscripts/" in name:
            return "binutils"
        return {"libamiga.a": "amiga-gcc", "libpthread.a": "aros-stuff",
                "libstubs.a": "libnix"}.get(Path(name).name, "newlib-cygwin")
    raise SupportError(f"{name}: no reviewed SDK install producer")


def _notice_data(source: Path, identity: str, canonical: str) -> list[tuple[str, bytes]]:
    """Collect original licence texts and the specifically reviewed inline-notice files."""
    inline = {
        "lha": {"README.md", "src/fnmatch.c", "src/fnmatch.h",
                "src/getopt_long.c", "src/getopt_long.h", "src/vsnprintf.c"},
        "libnix": {"sources/headers/poll.h", "sources/socket/extra/poll.c",
                   "sources/nix/extra/asprintf.c"},
    }
    result = []
    if source.is_dir():
        for directory, folders, files in os.walk(source, followlinks=False):
            if Path(directory) == source:
                folders[:] = [name for name in folders if name != ".git"
                              and (identity != "aros-stuff" or name == "pthreads")]
            for name in sorted(files):
                path = Path(directory) / name
                relative = path.relative_to(source).as_posix()
                if identity == "aros-stuff" and not relative.startswith("pthreads/"):
                    continue
                selected = (
                    relative == canonical or relative in inline.get(identity, set())
                    or re.match(r"(?:LICEN[CS]E|COPYING|NOTICE|COPYRIGHT)(?:$|[._-])",
                                name.upper())
                    or identity == "aros-stuff" and relative.startswith("pthreads/")
                    or identity == "libnix" and relative.startswith((
                        "sources/math/common/", "sources/math/mathfp/", "sources/socket/arpa/")))
                if selected and path.is_file() and not path.is_symlink():
                    result.append((relative, path.read_bytes()))
    else:
        with tarfile.open(source) as archive:
            for member in archive:
                relative = member.name.partition("/")[2]
                if member.isfile() and (
                        relative == canonical or re.match(
                            r"(?:LICEN[CS]E|COPYING|NOTICE|COPYRIGHT)(?:$|[._-])",
                            Path(member.name).name.upper())):
                    safe_relative(relative, identity)
                    if member.size > 16 * 1024 * 1024:
                        raise SupportError(f"{identity}/{relative}: oversized notice")
                    result.append((relative, archive.extractfile(member).read()))
    if not any(name == canonical for name, _ in result):
        raise SupportError(f"{identity}: missing original canonical notice {canonical}")
    return result


def assemble_payload(lock_path: Path, sources: Path, producer: Path, destination: Path,
                     *, sdk_base: Path | None = None) -> Path:
    """Assemble one real SDK, wheel or preparation payload and its complete audit.

    Sources must match the full lock. SDK/wheel bytes must match successful
    public producer provenance. Preparation additionally requires the assembled
    SDK baseline so its marker describes the complete installed tree, including
    notices and source. This maintainer API creates a fresh destination with
    payload/ and audit.json; it does not publish an archive or approve a licence.
    """
    if os.getuid() == 0:
        raise SupportError("assembly: require a non-root maintainer")
    lock, lock_digest = load_lock(lock_path)
    sources, producer = sources.resolve(strict=True), producer.resolve(strict=True)
    destination = destination.absolute()
    if destination.exists() or destination.is_symlink():
        raise SupportError(f"{destination}: refusing existing assembly output")
    if any(parent == destination.resolve() or parent in destination.resolve().parents
           for parent in (sources, producer)):
        raise SupportError("assembly: output must be outside input trees")
    verify_sources(lock, sources)
    original = inventory(producer)
    preparation = (producer / "preparation.json").is_file()
    wheels = (producer / "wheels.json").is_file()
    if not preparation:
        provenance = load_audit(producer.parent / "provenance.json")
        identities = {name: record.get("commit", record.get("sha256"))
                      for name, record in lock["sources"].items()}
        if (provenance.get("lock_sha256") != lock_digest or provenance.get("exit_code") != 0
                or provenance.get("files") != original or provenance.get("sources") != identities
                or provenance.get("inputs") != lock["inputs"]):
            raise SupportError("assembly: producer provenance does not match actual locked bytes")
    elif sdk_base is None:
        raise SupportError("assembly: preparation requires the complete assembled SDK baseline")
    if wheels:
        manifest = load_audit(producer / "wheels.json")
        if not isinstance(manifest, dict) or manifest.get("abi") not in ("cp310", "cp312"):
            raise SupportError("wheels.json: unsupported producer ABI")
        verify_wheels(producer, lock, lock_digest, manifest["abi"], original)
    if preparation:
        metadata = load_audit(producer / "preparation.json")
        expected_inputs = {name: digest for name, digest in lock["inputs"].items()
                           if name == lock["preparation"]["script"] or name.startswith("ndk-inputs/")}
        archive_id = lock["preparation"]["archive"]
        archive_record = lock["sources"][archive_id]
        if (not isinstance(metadata, dict) or set(metadata) != {
                "schema", "lock_sha256", "archive", "base", "helpers", "inputs", "inventory", "notices"}
                or type(metadata["schema"]) is not int or metadata["schema"] != 1
                or metadata["lock_sha256"] != lock_digest
                or metadata["archive"] != {"source": archive_id, "bytes": archive_record["bytes"],
                                           "sha256": archive_record["sha256"]}
                or metadata["inputs"] != expected_inputs
                or metadata["inventory"] != lock["preparation"]["inventory"]
                or not isinstance(metadata["helpers"], dict)
                or metadata["helpers"].keys() != lock["preparation"]["helpers"].keys()):
            raise SupportError("preparation.json: producer metadata differs from the real lock")
        expected_files = {name: {"type": "file", "sha256": digest}
                          for name, digest in expected_inputs.items()}
        for identity, helper in lock["preparation"]["helpers"].items():
            path = sdk_base / "bin" / identity
            digest = _hash(path)
            expected = {"path": helper["path"], "sha256": digest}
            if metadata["helpers"][identity] != expected:
                raise SupportError(f"{identity}: preparation helper differs from SDK baseline")
            expected_files[helper["path"]] = {"type": "file", "sha256": digest}
        expected_files["preparation.json"] = original.get("preparation.json")
        if expected_files != original:
            raise SupportError("preparation: unaccounted or modified producer resources")
    destination.mkdir(parents=True)
    payload = destination / "payload"
    try:
        shutil.copytree(producer, payload, symlinks=True)
        records = {}
        components = set()
        packages = load_audit(producer / "wheels.json")["packages"] if wheels else {}
        wheel_sources = {name: data["source"] for name, data in packages.items()}
        for name, record in original.items():
            if preparation:
                if name.startswith("helpers/"):
                    origins = [Path(name).name]
                    role = "program"
                elif name.startswith("ndk-inputs/netinclude/"):
                    origins, role = ["amiga-netinclude"], "preparation"
                elif name.startswith("ndk-inputs/helper-headers/"):
                    origins = ["fd2sfd" if name.endswith("/alib.h") else "fd2pragma"]
                    role = "preparation"
                else:
                    origins, role = ["amiga-gcc"], "preparation"
            elif wheels:
                if name.startswith("wheels/"):
                    origins = [next(data["source"] for data in packages.values()
                                    if data["filename"] == name)]
                    role = "wheel"
                elif name.startswith("notices/"):
                    origins, role = [name.split("/")[1]], "notice"
                elif name.startswith("sources/"):
                    identity = Path(name).name
                    for suffix in (".tar.gz", ".tar.xz", ".tar"):
                        identity = identity.removesuffix(suffix)
                    origins = [identity] if identity in lock["sources"] else sorted(set(wheel_sources.values()))
                    role = "source"
                elif name == "wheels.json":
                    origins, role = sorted(set(wheel_sources.values())), "preparation"
                else:
                    raise SupportError(f"{name}: unknown wheel producer output")
            else:
                origins = [_sdk_origin(name)]
                role = ("header" if "/include/" in name or "/sys-include/" in name
                        or "/include-fixed/" in name else "library")
                if (name.startswith(("bin/", "m68k-amigaos/bin/", "libexec/"))
                        and not name.endswith(".so")):
                    role = "program"
            components.update(origins)
            records[name] = dict(record, origins=origins,
                                 role="link" if record["type"] == "symlink" else role)
        header_sources = {}
        if not preparation and not wheels:
            for identity, directories in (
                    ("newlib-cygwin", ("newlib/libc/include", "newlib/libc/sys/amigaos/include")),
                    ("libnix", ("sources/headers",)), ("aros-stuff", ("pthreads",))):
                root = sources / "checkouts" / identity
                for directory in directories:
                    for path in sorted((root / directory).rglob("*")):
                        if path.is_file() and not path.is_symlink():
                            header_sources.setdefault(identity, {}).setdefault(
                                _hash(path), []).append(path.relative_to(root).as_posix())
            for name, record in records.items():
                identity = record["origins"][0]
                if record["role"] == "header" and identity in header_sources:
                    if record["sha256"] not in header_sources[identity]:
                        if (identity != "newlib-cygwin"
                                or name != "m68k-amigaos/sys-include/newlib.h"):
                            raise SupportError(f"{name}: copied header has no matching source bytes")
                        if not (payload / name).read_bytes().startswith(
                                b"/* newlib.h.  Generated from newlib.hin by configure."):
                            raise SupportError(f"{name}: unexpected generated-header source")

        def put(name: str, content: bytes, role: str, origins: list[str]) -> None:
            """Add original supporting bytes without overwriting a producer's different file."""
            safe_relative(name, name)
            path = payload / name
            if path.is_symlink() or path.exists() and path.read_bytes() != content:
                raise SupportError(f"{name}: conflicting assembled file")
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
            path.chmod(0o644)
            components.update(origins)
            records[name] = {"type": "file", "sha256": hashlib.sha256(content).hexdigest(),
                             "origins": origins, "role": role}

        if not wheels:
            if not preparation:
                components.update(("gmp", "mpfr", "mpc"))
            for identity in sorted(components):
                source = lock["sources"][identity]
                canonical = source["licence"]["notice"]
                if source["kind"] == "git":
                    root = sources / "checkouts" / identity
                    archive_name = "sources/" + identity + ".tar.gz"
                    (payload / "sources").mkdir(exist_ok=True)
                    _source_archive(root, payload / archive_name, identity)
                    records[archive_name] = {
                        "type": "file", "sha256": _hash(payload / archive_name),
                        "origins": [identity], "role": "source"}
                else:
                    root = sources / "archives" / identity
                    archive_name = "sources/" + identity + ".tar"
                    with root.open("rb") as stream:
                        signature = stream.read(6)
                    archive_name += ".bz2" if signature.startswith(b"BZh") else ".xz" if signature.startswith(b"\xfd7zXZ") else ".gz"
                    put(archive_name, root.read_bytes(), "source", [identity])
                for relative, content in _notice_data(root, identity, canonical):
                    put("notices/" + identity + "/" + relative, content, "notice", [identity])
        else:
            components.update(record["source"] for record in packages.values())
            for name in ("amitools", "machine68k"):
                put("notices/" + name + "/GPL-2.0.txt",
                    (sources / "checkouts/amiga-gcc/COPYING").read_bytes(), "notice", [name])
        supported = sorted(components)
        for name in ("LICENSE", "LICENSE-EXCEPTION.md"):
            put("notices/clorraine-build-code/" + name,
                (Path(__file__).resolve().parents[2] / name).read_bytes(), "notice", supported)
        for name in ("reference-support.sh", "support-payload.sh", "wheel-payload.py",
                     "environment.Dockerfile", "build-environment.sh", "ubuntu-packages.sha256",
                     "support.py", "support_build.py", "build-support.sh", "build-wheels.sh"):
            put("sources/build-inputs/" + name, (sources / "inputs" / name).read_bytes(),
                "source", supported)
        for name in sorted(lock["inputs"]):
            if name.startswith("compiler-patches/"):
                put("sources/build-inputs/" + name, (sources / "inputs" / name).read_bytes(),
                    "source", ["gcc"] if "gcc" in components else supported)
        if "libnix" in components:
            for path in sorted((sources / "inputs/source-forms").iterdir()):
                put("sources/source-forms/" + path.name, path.read_bytes(), "source", ["libnix"])
        readme = (
            "These files accompany the exact component sources and locked build inputs.\n"
            "Git snapshots omit private Git metadata and prebuilt Amiga libraries.\n"
            "When rebuilding GCC, apply build-inputs/compiler-patches/gcc-find-file-specs.diff "
            "to the original GCC snapshot before configuring/building it. The adjacent "
            "README records the modification notice and complete rebuild context.\n"
            "AROS contains only the selected pthread source subtree.\n"
            "Before rebuilding libnix, restore its inherited pipeoc source with:\n"
            "  patch -F0 -o libnix/sources/nix/extra/pipeoc.c "
            "source-forms/gnuplot-3.7.1-amiga.c source-forms/libnix-pipeoc.diff\n"
            "The integrity-checking recipes expect the locked Git checkouts; acquire "
            "their metadata separately when rerunning those checks.\n"
            "The original build commands, versions and inputs are supplied alongside them.\n"
            "File origin IDs associate supporting recipes/reviews with their components; "
            "they do not change copyright ownership.\n"
            "Clorraine's build-code notices apply only to project-authored files and "
            "do not relicense upstream code or the prebuilt libamiga archive.\n"
        ).encode()
        put("sources/BUILDING.txt", readme, "source", supported)
        component_records = {}
        for identity in supported:
            review = lock["sources"][identity]["licence"]["review"]
            name = "reviews/" + identity + ".md"
            put(name, (sources / "inputs" / review).read_bytes(), "review", [identity])
            component_records[identity] = {
                "review": name,
                "notices": sorted(name for name, record in records.items()
                                  if record["role"] == "notice" and identity in record["origins"]),
                "source": sorted(name for name, record in records.items()
                                 if record["role"] == "source" and identity in record["origins"]),
            }
        if preparation:
            metadata = load_audit(producer / "preparation.json")
            if metadata.get("lock_sha256") != lock_digest:
                raise SupportError("preparation.json: different producer lock")
            metadata["base"] = inventory(sdk_base.resolve(strict=True))
            for identity, helper in metadata["helpers"].items():
                path = sdk_base / "bin" / identity
                if _hash(path) != helper["sha256"]:
                    raise SupportError(f"{identity}: helper differs from the assembled SDK")
            content = (json.dumps(metadata, sort_keys=True, indent=2) + "\n").encode()
            (payload / "preparation.json").write_bytes(content)
            records["preparation.json"]["sha256"] = hashlib.sha256(content).hexdigest()
        trace = {
            "lock_sha256": lock_digest,
            "method": "actual locked producer bytes plus reviewed install rules; copied headers match source hashes",
            "producer_files": {
                name: {
                    "producer_record": record, "published_sha256": records[name].get("sha256"),
                    "origins": records[name]["origins"],
                    "matching_header_sources": header_sources.get(
                        records[name]["origins"][0], {}).get(record.get("sha256"), []),
                } for name, record in original.items()
            },
            "build_recipe_sha256": lock["inputs"]["reference-support.sh"],
            "notes": [
                "Generated newlib.h comes from the locked newlib.hin/configure inputs.",
                "Target libraries and generated GCC files follow their recorded install producers.",
                "Supporting recipe/review origin IDs identify associated components, not copyright ownership.",
                "The ordinary libamiga source/terms gap remains explicitly recorded; no grant is inferred.",
            ],
        }
        put("reviews/file-origins.json",
            (json.dumps(trace, sort_keys=True, indent=2) + "\n").encode(), "review", supported)
        native = inspect_payload(payload, records, lock["output"]["builder_prefixes"])
        for name, record in native.items():
            providers = {}
            for soname in record["needed"]:
                choices = [{package for package, value in cohort.items()
                            if soname in value["sonames"]}
                           for cohort in lock["output"]["runtime_packages"].values()]
                shared = set.intersection(*choices)
                if len(shared) != 1:
                    raise SupportError(f"{name}: ambiguous/missing system provider for {soname}")
                providers[soname] = "package:" + shared.pop()
            record["needed"] = providers
            record.pop("rpaths")
        audit = {"schema": 1, "lock_sha256": lock_digest, "components": component_records,
                 "files": records, "native": native}
        audit_payload(lock, lock_digest, payload, audit)
        (destination / "audit.json").write_text(
            json.dumps(audit, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    except BaseException:
        shutil.rmtree(destination)
        raise
    return destination


def write_archive(payload: Path, files: dict, output: Path) -> None:
    """Publish only verified archive bytes using a private sibling and no-clobber link.

    Re-reading the completed tar catches content changes between audit and
    archiving. The sibling hard link publishes the whole file atomically and
    refuses a destination created during the audit, unlike an overwriting
    rename. The private temporary name is always removed.
    """
    if output.exists() or output.is_symlink():
        raise SupportError(f"{output.name}: refusing an existing output")
    output.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix="." + output.name + ".", dir=output.parent)
    temporary = Path(temporary)
    try:
        with os.fdopen(descriptor, "wb") as stream, tarfile.open(
                fileobj=stream, mode="w", format=tarfile.PAX_FORMAT) as archive:
            for name in sorted(files):
                record = files[name]
                path = payload / name
                info = tarfile.TarInfo(name)
                info.uid = info.gid = info.mtime = 0
                info.uname = info.gname = ""
                if record["type"] == "symlink":
                    if not path.is_symlink() or os.readlink(path) != record["target"]:
                        raise SupportError(f"{name}: link changed after audit")
                    info.type = tarfile.SYMTYPE
                    info.linkname = record["target"]
                    info.mode = 0o777
                    archive.addfile(info)
                else:
                    descriptor = os.open(path, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW)
                    with os.fdopen(descriptor, "rb") as content:
                        status = os.fstat(content.fileno())
                        if not stat.S_ISREG(status.st_mode):
                            raise SupportError(f"{name}: no longer an ordinary file")
                        info.size = status.st_size
                        info.mode = 0o755 if status.st_mode & 0o100 else 0o644
                        archive.addfile(info, content)
        with tarfile.open(temporary, mode="r:") as archive:
            for member in archive:
                record = files[member.name]
                if member.isfile():
                    digest = hashlib.sha256()
                    with archive.extractfile(member) as content:
                        while block := content.read(1024 * 1024):
                            digest.update(block)
                    if digest.hexdigest() != record["sha256"]:
                        raise SupportError(f"{member.name}: file changed after audit")
        os.link(temporary, output)
    finally:
        temporary.unlink()


def main(arguments: list[str] | None = None) -> int:
    """Validate the public arguments and audit before creating any successful output."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lock", type=Path, required=True)
    parser.add_argument("--payload", type=Path, required=True)
    parser.add_argument("--audit", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(arguments)
    try:
        if os.getuid() == 0:
            raise SupportError("package-support: require a non-root maintainer")
        if args.output.exists() or args.output.is_symlink():
            raise SupportError(f"{args.output.name}: refusing an existing output")
        if args.payload.is_symlink() or not args.payload.is_dir():
            raise SupportError("payload: expected an ordinary directory")
        payload = args.payload.resolve(strict=True)
        output = args.output.absolute()
        if output == payload or payload in output.resolve().parents:
            raise SupportError("output: archive must be outside the payload")
        lock, digest = load_lock(args.lock)
        files = audit_payload(lock, digest, payload, load_audit(args.audit))
        write_archive(payload, files, output)
    except (SupportError, OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"package-support: {error}", file=sys.stderr)
        return 1
    print("package-support: audited archive created; release acceptance still required")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
