"""Shared isolation, inspection and provenance for the two support-build commands."""

import argparse
from email.parser import BytesParser
import hashlib
import itertools
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import zipfile

from support import SupportError, load_lock, verify_sources


def fail(identity, message):
    """Name the failed contract without publishing a successful output."""
    raise SupportError(f"{identity}: {message}")


def inspect_image(reference):
    """Read one existing image without pulling or trusting a mutable alias."""
    result = subprocess.run(
        ["docker", "image", "inspect", reference], capture_output=True,
        timeout=60, check=False)
    if result.returncode:
        fail(reference, "Docker could not inspect the local image")
    try:
        images = json.loads(result.stdout)
        if not isinstance(images, list) or len(images) != 1 or not isinstance(images[0], dict):
            fail(reference, "expected one image inspection")
        image = images[0]
        if re.fullmatch(r"sha256:[0-9a-f]{64}", str(image.get("Id", ""))) is None:
            fail(reference, "invalid local image identity")
        if image.get("Os") != "linux" or image.get("Architecture") != "amd64":
            fail(reference, "expected Linux/amd64")
        layers = image["RootFS"]["Layers"]
        if (image["RootFS"].get("Type") != "layers" or not isinstance(layers, list)
                or not layers or any(not isinstance(layer, str)
                                     or re.fullmatch(r"sha256:[0-9a-f]{64}", layer) is None
                                     for layer in layers)):
            fail(reference, "invalid root filesystem layers")
        if not isinstance(image.get("Config"), dict):
            fail(reference, "invalid image configuration")
        digests = image.get("RepoDigests")
        if not isinstance(digests, list) or any(not isinstance(item, str) for item in digests):
            fail(reference, "invalid registry digest observations")
    except (ValueError, KeyError, TypeError) as error:
        raise SupportError(f"{reference}: invalid Docker image inspection") from error
    return image


def verify_image(lock, image_id):
    """Match the immutable execution identity, pinned base layers and recipe labels."""
    prepared = inspect_image(image_id)
    if prepared.get("Id") != image_id:
        fail(image_id, "inspected execution identity differs")
    labels = prepared.get("Config", {}).get("Labels") or {}
    if not isinstance(labels, dict):
        fail(image_id, "invalid preparation labels")
    base_reference = lock["build"]["image"]
    recipe = lock["build"]["environment_recipe"]
    if labels.get("org.clorraine.base-image") != base_reference:
        fail(base_reference, "prepared image has a different base label")
    if labels.get("org.clorraine.environment-recipe-sha256") != lock["inputs"][recipe]:
        fail(recipe, "prepared image has a different environment recipe label")
    base = inspect_image(base_reference)
    # Docker resolved the digest-pinned reference above. RepoDigests may render
    # Docker Hub names without docker.io/library/, so compare actual layers,
    # not the registry's presentation spelling.
    layers = base["RootFS"]["Layers"]
    if prepared["RootFS"]["Layers"][:len(layers)] != layers:
        fail(base_reference, "base layers are not a prefix of the prepared image")


def inventory(root):
    """Inspect every actual payload file/link before publishing provenance.

    This checks safe filesystem structure, not redistribution permission or
    the later native-loader audit. Directories are implicit inventory parents.
    """
    if root.is_symlink() or not root.is_dir():
        fail("payload", "recipe did not create an ordinary payload directory")
    records = {}

    def unreadable(error):
        """Do not mistake inaccessible directories for empty, verified inputs."""
        raise error

    for directory, folders, files in os.walk(root, followlinks=False, onerror=unreadable):
        parent = Path(directory)
        if not folders and not files:
            fail(parent.relative_to(root).as_posix(), "unexpected empty payload directory")
        for name in sorted(folders + files):
            path = parent / name
            relative = path.relative_to(root).as_posix()
            relative.encode("utf-8")
            if ("\\" in relative or "!" in relative
                    or any(ord(char) < 32 or 127 <= ord(char) <= 159 for char in relative)):
                fail(repr(relative), "unsafe payload path")
            mode = path.lstat().st_mode
            if mode & 0o7000:
                fail(relative, "special permission bits are not permitted")
            if stat.S_ISLNK(mode):
                target = os.readlink(path)
                if os.path.isabs(target):
                    fail(relative, "absolute payload link")
                try:
                    resolved = path.resolve(strict=True)
                except (OSError, RuntimeError) as error:
                    raise SupportError(f"{relative}: dangling or cyclic payload link") from error
                if resolved != root and root not in resolved.parents:
                    fail(relative, "escaping payload link")
                records[relative] = {"type": "symlink", "target": target}
            elif stat.S_ISREG(mode):
                digest = hashlib.sha256()
                with path.open("rb") as stream:
                    for block in iter(lambda: stream.read(1024 * 1024), b""):
                        digest.update(block)
                records[relative] = {"type": "file", "sha256": digest.hexdigest()}
            elif not stat.S_ISDIR(mode):
                fail(relative, "special payload file")
    return records


def _wheel_record(value, fields, identity):
    """Require the exact wheel-manifest shape rather than silently ignoring fields."""
    if not isinstance(value, dict) or any(not isinstance(key, str) for key in value):
        fail(identity, "expected an object")
    if value.keys() != fields:
        fail(identity, f"missing/unknown fields: {sorted(value.keys() ^ fields)!r}")
    return value


def _wheel_relative(value, identity):
    """Validate canonical wheel/member names without using them as host paths."""
    if not isinstance(value, str) or not value:
        fail(identity, "expected a relative filename")
    value.encode("utf-8")
    if (value.startswith("/") or "\\" in value or "!" in value
            or any(part in ("", ".", "..") for part in value.split("/"))
            or any(ord(char) < 32 or 127 <= ord(char) <= 159 for char in value)):
        fail(identity, f"unsafe wheel/member path {value!r}")
    return value


def _compatible_tag(tag, abi):
    """Match the supported CPython/Linux tags, including legacy glibc-based manylinux."""
    parts = tag.split("-")
    if len(parts) != 3:
        return False
    python, interface, platform = parts
    minor = int(abi[3:])
    if platform == "any":
        platform_ok = interface == "none"
    elif platform == "linux_x86_64" or platform in {
            "manylinux1_x86_64", "manylinux2010_x86_64", "manylinux2014_x86_64"}:
        platform_ok = True
    else:
        match = re.fullmatch(r"manylinux_(\d+)_(\d+)_x86_64", platform)
        platform_ok = match is not None and (int(match[1]), int(match[2])) <= (2, 35)
    if interface == "abi3":
        match = re.fullmatch(r"cp3(\d+)", python)
        python_ok = match is not None and int(match[1]) <= minor
    elif interface == abi:
        python_ok = python == abi
    elif interface == "none":
        python_ok = python in {abi, "py3", f"py3{minor}"}
    else:
        python_ok = False
    return platform_ok and python_ok


def _inspect_wheel(path, name, record, abi):
    """Compare actual ZIP metadata/tags with its filename and recorded identity."""
    filename = path.name[:-4].split("-")
    if len(filename) not in (5, 6):
        fail(name, "invalid wheel filename")
    normalise = lambda text: re.sub(r"[-_.]+", "-", text).lower()
    if normalise(filename[0]) != name or filename[1] != record["version"]:
        fail(name, "wheel filename name/version mismatch")
    expanded = {"-".join(parts) for parts in itertools.product(
        *(part.split(".") for part in filename[-3:]))}
    tags = record["tags"]
    if (not isinstance(tags, list) or not tags
            or any(not isinstance(tag, str) or re.fullmatch(
                r"[a-z0-9_]+-[a-z0-9_]+-[a-z0-9_]+", tag) is None for tag in tags)
            or len(set(tags)) != len(tags) or set(tags) != expanded):
        fail(name, "wheel filename/manifest tags disagree")
    if not any(_compatible_tag(tag, abi) for tag in tags):
        fail(name, f"wheel is incompatible with {abi}/Ubuntu x86_64")
    try:
        with zipfile.ZipFile(path) as archive:
            entries = archive.infolist()
            seen = set()
            for entry in entries:
                member = _wheel_relative(
                    entry.filename[:-1] if entry.is_dir() else entry.filename, name)
                if member in seen:
                    fail(name, f"duplicate wheel member {member!r}")
                seen.add(member)
                mode = entry.external_attr >> 16
                if mode & 0o7000 or stat.S_IFMT(mode) not in (0, stat.S_IFREG, stat.S_IFDIR):
                    fail(name, f"special wheel member {member!r}")
            metadata = {}
            prefixes = set()
            for kind in ("METADATA", "WHEEL"):
                choices = [entry for entry in entries
                           if entry.filename.endswith(".dist-info/" + kind)]
                if len(choices) != 1 or choices[0].file_size > 16 * 1024 * 1024:
                    fail(name, f"missing, ambiguous or oversized {kind}")
                member = choices[0].filename
                if len(member.split("/")) != 2:
                    fail(name, "wheel metadata must be at its distribution root")
                prefixes.add(member.split("/")[0])
                metadata[kind] = BytesParser().parsebytes(archive.read(choices[0]))
            if len(prefixes) != 1:
                fail(name, "wheel metadata belongs to different distributions")
            distribution = metadata["METADATA"]
            for key, expected in (("Name", name), ("Version", record["version"])):
                values = distribution.get_all(key, [])
                if len(values) != 1 or (normalise(values[0]) if key == "Name" else values[0]) != expected:
                    fail(name, f"actual wheel {key} mismatch")
            actual_tags = metadata["WHEEL"].get_all("Tag", [])
            if len(actual_tags) != len(tags) or set(actual_tags) != set(tags):
                fail(name, "actual wheel tags mismatch")
            if metadata["WHEEL"].get("Wheel-Version") != "1.0":
                fail(name, "unsupported wheel format")
    except (zipfile.BadZipFile, KeyError, RuntimeError) as error:
        raise SupportError(f"{name}: invalid wheel ZIP") from error


def verify_wheels(root, lock, lock_digest, abi, files):
    """Validate the complete wheel set before any successful wheel output is published.

    This is metadata/transport validation. The separate native-loader and
    real-runtime acceptance gates still inspect ELF content and execute wheels.
    """
    path = root / "wheels.json"
    if files.get("wheels.json", {}).get("type") != "file":
        fail("wheels.json", "missing ordinary wheel manifest")

    def unique(pairs):
        """Keep duplicate keys from replacing selected hashes or package versions."""
        result = {}
        for key, value in pairs:
            if key in result:
                fail("wheels.json", f"duplicate JSON key {key!r}")
            result[key] = value
        return result

    with path.open("rb") as stream:
        content = stream.read(16 * 1024 * 1024 + 1)
    if len(content) > 16 * 1024 * 1024:
        fail("wheels.json", "manifest exceeds 16 MiB")
    try:
        metadata = json.loads(content.decode("utf-8"), object_pairs_hook=unique)
    except (json.JSONDecodeError, UnicodeError, RecursionError) as error:
        raise SupportError("wheels.json: invalid UTF-8 JSON") from error
    _wheel_record(metadata, {"schema", "lock_sha256", "abi", "packages"}, "wheels.json")
    if type(metadata["schema"]) is not int or metadata["schema"] != 1:
        fail("schema", "unsupported wheel schema")
    if metadata["lock_sha256"] != lock_digest:
        fail("lock_sha256", "wheel metadata does not match the exact lock bytes")
    if metadata["abi"] != abi:
        fail("abi", "wheel metadata does not match the requested interpreter")
    packages = metadata["packages"]
    if not isinstance(packages, dict):
        fail("packages", "expected a package map")
    for name in ("amitools", "machine68k", "greenlet", "pysdl2", "pip"):
        if name not in packages:
            fail(name, "required wheel is absent")
    filenames = set()
    for name, record in packages.items():
        if re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", name) is None:
            fail(repr(name), "package names must be normalised")
        _wheel_record(record, {"version", "source", "purpose", "filename",
                               "bytes", "sha256", "tags"}, name)
        if not isinstance(record["version"], str) or re.fullmatch(
                r"[0-9][a-zA-Z0-9.!+_-]*", record["version"]) is None:
            fail(name, "invalid exact wheel version")
        source = record["source"]
        if not isinstance(source, str) or source not in lock["sources"]:
            fail(name, f"unknown wheel source {source!r}")
        if (lock["sources"][source]["kind"] == "archive"
                and record["version"] != lock["sources"][source]["version"]):
            fail(name, "wheel version differs from its archive source")
        if record["purpose"] not in ("runtime", "bootstrap"):
            fail(name, "invalid wheel purpose")
        if name in ("amitools", "machine68k", "greenlet", "pysdl2", "pip"):
            if record["purpose"] != ("bootstrap" if name == "pip" else "runtime"):
                fail(name, "incorrect runtime/bootstrap purpose")
        filename = _wheel_relative(record["filename"], name)
        if (len(filename.split("/")) != 2 or not filename.startswith("wheels/")
                or not filename.endswith(".whl") or filename in filenames):
            fail(name, f"invalid or repeated wheel filename {filename!r}")
        wheel = root / filename
        if files.get(filename, {}).get("type") != "file":
            fail(filename, "missing ordinary wheel archive")
        if (type(record["bytes"]) is not int or record["bytes"] <= 0
                or wheel.stat().st_size != record["bytes"]
                or files[filename]["sha256"] != record["sha256"]):
            fail(name, "wheel length/hash mismatch")
        _inspect_wheel(wheel, name, record, abi)
        filenames.add(filename)
    directory = root / "wheels"
    if directory.is_symlink() or not directory.is_dir():
        fail("wheels", "expected an ordinary wheel directory")
    actual = {path.relative_to(root).as_posix() for path in directory.iterdir()}
    if actual != filenames:
        fail("wheels", f"unexpected/missing wheel files: {sorted(actual ^ filenames)!r}")


def stop_interrupted_container(cidfile):
    """Stop only the container named by this invocation's private Docker control file."""
    if not cidfile.is_file():
        return
    with cidfile.open("rb") as stream:
        identity = stream.read(128).strip()
    if re.fullmatch(rb"[0-9a-f]{64}", identity) is None:
        fail("container", "invalid owned container identity; manual cleanup required")
    result = subprocess.run(
        ["docker", "stop", "--time", "20", identity.decode("ascii")],
        capture_output=True, timeout=40, check=False)
    if result.returncode:
        fail(identity.decode("ascii"), "could not confirm container stop; manual cleanup required")


def build(lock, lock_digest, sources, image_id, output, abi=None):
    """Run the literal recipe, inspect its output and publish only this build's artifacts."""
    output.mkdir(parents=True, exist_ok=True)
    published = False
    with tempfile.TemporaryDirectory(prefix=".support-build-", dir=output) as temporary:
        stage = Path(temporary)
        work = stage / "work"
        work.mkdir()
        # Docker writes its control file outside the container's writable mount.
        # The recipe therefore cannot select another host container for cleanup.
        cidfile = stage / "container.id"
        argv = [*lock["build"]["argv"], *(["--wheels", abi] if abi is not None else [])]
        command = [
            "docker", "run", "--rm", "--pull=never", "--network=none",
            "--user", f"{os.getuid()}:{os.getgid()}",
            "--ulimit", "fsize=1073741824:1073741824",
            "--cidfile", str(cidfile), "--workdir", "/work",
            "--env", f"CLORRAINE_SUPPORT_LOCK_SHA256={lock_digest}",
            "--mount", f"type=bind,src={sources},dst=/sources,readonly",
            "--mount", f"type=bind,src={work},dst=/work",
            "--entrypoint", "", image_id, *argv,
        ]
        # Clearing ENTRYPOINT disables inherited image startup code. The
        # complete locked argv then supplies the executable and its arguments.
        try:
            try:
                result = subprocess.run(command, timeout=21600, check=False)
            except BaseException:
                stop_interrupted_container(cidfile)
                raise
            if result.returncode:
                fail(argv[1], f"recipe failed (Docker exit {result.returncode})")
            files = inventory(work / "payload")
            if abi is not None:
                verify_wheels(work / "payload", lock, lock_digest, abi, files)
            provenance = {
                "schema": 1, "lock_sha256": lock_digest,
                "image": lock["build"]["image"], "image_id": image_id,
                "environment_recipe_sha256": lock["inputs"][lock["build"]["environment_recipe"]],
                "argv": argv, "exit_code": 0,
                "sources": {name: source.get("commit", source.get("sha256"))
                            for name, source in lock["sources"].items()},
                "inputs": lock["inputs"], "files": files,
            }
            metadata = stage / "provenance.json"
            metadata.write_text(json.dumps(provenance, sort_keys=True, indent=2) + "\n",
                                encoding="utf-8")
            if output.is_symlink() or {path.name for path in output.iterdir()} != {stage.name}:
                fail("output", "destination changed during the build")
            (work / "payload").rename(output / "payload")
            published = True
            metadata.rename(output / "provenance.json")
        except BaseException:
            if published:
                shutil.rmtree(output / "payload")
            raise


def main(mode="support", arguments=None):
    """Run a fixed support or wheel entry without exposing arbitrary recipe suffixes.

    The public shell wrappers select the mode. Wheel callers must choose one
    supported ABI; ordinary support callers cannot pass wheel options.
    """
    if mode not in ("support", "wheels"):
        raise ValueError("unknown internal build mode")
    label = "build-wheels" if mode == "wheels" else "build-support"
    parser = argparse.ArgumentParser(prog=label, description=__doc__)
    parser.add_argument("--lock", type=Path, required=True)
    parser.add_argument("--sources", type=Path, required=True)
    parser.add_argument("--image-id", required=True)
    parser.add_argument("--output", type=Path, required=True)
    if mode == "wheels":
        parser.add_argument("--abi", choices=("cp310", "cp312"), required=True)
    args = parser.parse_args(arguments)
    try:
        if os.getuid() == 0:
            fail(label, "require a non-root caller")
        if re.fullmatch(r"sha256:[0-9a-f]{64}", args.image_id) is None:
            fail(args.image_id, "expected an immutable prepared image ID")
        if args.output.is_symlink():
            fail(f"output {str(args.output)!r}", "destination must not be a symlink")
        sources = args.sources.resolve(strict=True)
        output = args.output.resolve()
        if (sources == output or sources in output.parents or output in sources.parents):
            fail("output", "source staging and output must be disjoint")
        if "," in str(sources) or "," in str(output):
            fail("output/sources", "commas cannot be represented in Docker bind mounts")
        if output.exists() and (not output.is_dir() or any(output.iterdir())):
            fail(f"output {str(output)!r}", "refusing an existing nonempty destination")
        lock, digest = load_lock(args.lock)
        verify_sources(lock, sources)
        verify_image(lock, args.image_id)
        build(lock, digest, sources, args.image_id, output,
              args.abi if mode == "wheels" else None)
    except (SupportError, OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        print(f"{label}: {error}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        print(f"{label}: interrupted; private build staging removed", file=sys.stderr)
        return 130
    print(f"{label}: payload and provenance created; packaging audit still required")
    return 0
