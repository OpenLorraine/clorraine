"""Construct independently authenticated miniature producer inputs.

These archives are useful for metadata and extraction failures only. Their
programs intentionally cannot establish compiler, wheel or Amiga compatibility.
"""

from __future__ import annotations

import base64
import csv
import io
import json
import os
import shutil
import subprocess
from pathlib import Path

from native_manifest_data import (
    NOTICES, PACKAGE_NAMES, VERSION, digest, encode, tar_bytes, zip_bytes,
)
from native_transport_fixture import limit_files


LICENCE = b"Synthetic fixture licence.\n"
"""Notice bytes shared by the synthetic reviewed source."""
REVIEW = b"Synthetic review; not publication approval.\n"
"""Hashed review input retained in each miniature audited archive."""
PROGRAM = b"#!/bin/sh\nexit 0\n"
"""Tiny ordinary program used by the documented miniature support source."""
NDK_HEADER = b"Synthetic excluded NDK-derived header.\n"
"""Excluded content introduced only during private NDK preparation."""
_CORE_SOURCE = r'''
#include <stdio.h>
#include <string.h>
int main(int argc, char **argv) {
    if (argc == 2 && strcmp(argv[1], "--version") == 0) {
        puts(TOOL_NAME " " TOOL_VERSION);
        return 0;
    }
    fputs("synthetic fixture is not a runtime acceptance input\n", stderr);
    return 97;
}
'''
"""Observation executable source, never a substitute for compiler smoke proof."""


def _command(argv: list[str], root: Path, environment: dict[str, str]) -> bytes:
    """Run a bounded fixture-building process and name its failed stderr."""
    result = subprocess.run(argv, cwd=root, env=environment, capture_output=True,
                            timeout=30, check=False, preexec_fn=limit_files)
    if result.returncode:
        raise AssertionError(f"fixture process {argv[0]} failed: {result.stderr!r}")
    return result.stdout


def miniature_lock(root: Path) -> tuple[dict, bytes]:
    """Create the documented real Git identity, NDK archive and exact support lock."""
    source = root / "source"
    source.mkdir()
    (source / "LICENSE").write_bytes(LICENCE)
    for name, data in (("bin/lha", PROGRAM), ("notices/lha.txt", LICENCE),
                       ("reviews/lha.txt", REVIEW), ("sources/lha.sh", PROGRAM)):
        path = source / "payload" / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        if name == "bin/lha":
            path.chmod(0o755)
    environment = {key: value for key, value in os.environ.items()
                   if not key.startswith(("GIT_", "PYTHON", "LD_"))}
    environment.update(HOME=str(root), GIT_CONFIG_NOSYSTEM="1",
                       GIT_AUTHOR_DATE="2000-01-01T00:00:00Z",
                       GIT_COMMITTER_DATE="2000-01-01T00:00:00Z")
    git = shutil.which("git")
    if git is None:
        raise AssertionError("git is required only to construct fixture identities")
    for argv in ([git, "init", "-q"],
                 [git, "add", "LICENSE", "payload"],
                 [git, "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                  "-c", "core.hooksPath=/dev/null", "commit", "-qm", "Fixture"]):
        _command(argv, source, environment)
    commit = _command([git, "rev-parse", "HEAD"], source, environment).decode().strip()
    ndk = tar_bytes({"LICENSE": LICENCE})
    recipe = b"#!/bin/sh\nset -eu\ncp -R /sources/checkouts/lha/payload /work/payload\n"
    dockerfile = b"ARG BASE_IMAGE\nFROM ${BASE_IMAGE}\n"
    licence = {"id": "LicenseRef-Fixture", "notice": "LICENSE",
               "source_required": True, "review": "review.txt"}
    package = {"version": "1", "sonames": [], "paths": ["/bin/sh"]}
    lock = {
        "schema": 1,
        "sources": {
            "lha": {"kind": "git", "url": "https://example.invalid/lha.git",
                    "commit": commit, "licence": licence},
            "seed": {"kind": "archive", "url": "https://example.invalid/seed.tar",
                     "version": "1", "bytes": len(ndk), "sha256": digest(ndk),
                     "licence": licence}},
        "inputs": {"recipe.sh": digest(recipe), "prepare.sh": digest(PROGRAM),
                   "review.txt": digest(REVIEW), "environment.Dockerfile": digest(dockerfile)},
        "build": {"image": "docker.io/library/ubuntu@sha256:" + "0" * 64,
                  "environment_recipe": "environment.Dockerfile", "ubuntu": "22.04",
                  "packages": {"dash": "1"}, "tools": {"python3": "3.10.0"},
                  "argv": ["/bin/sh", "/sources/inputs/recipe.sh"]},
        "preparation": {
            "archive": "seed", "script": "prepare.sh", "patches": [],
            "helpers": {"lha": {"source": "lha", "path": "bin/lha"}},
            "inventory": {"include/fixture-ndk.h": digest(NDK_HEADER),
                          "notices/ndk.txt": digest(LICENCE)}},
        "output": {"architecture": "x86_64", "python_abis": ["cp310", "cp312"],
                   "runtimes": ["nix13", "nix20"],
                   "runtime_packages": {"22.04": {"dash": package}, "24.04": {"dash": package}},
                   "builder_prefixes": ["/sources", "/work"],
                   "forbidden_sha256": [digest(NDK_HEADER)]},
    }
    return lock, ndk


def _reviewed(files: dict[str, tuple[bytes, str]], lock: dict) -> tuple[bytes, bytes]:
    """Bind complete ordinary-file coverage, notice/source roles and review bytes."""
    files = dict(files)
    files.update({"notices/lha.txt": (LICENCE, "notice"),
                  "reviews/lha.txt": (REVIEW, "review"),
                  "sources/lha.sh": (PROGRAM, "source")})
    audit = {
        "schema": 1, "lock_sha256": digest(encode(lock)),
        "components": {"lha": {"review": "reviews/lha.txt", "notices": ["notices/lha.txt"],
                              "source": ["sources/lha.sh"]}},
        "files": {name: {"type": "file", "sha256": digest(data),
                         "origins": ["lha"], "role": role}
                  for name, (data, role) in files.items()},
        "native": {},
    }
    executable = tuple(name for name, (_, role) in files.items()
                       if role == "program" or name.endswith(".py"))
    return tar_bytes({name: data for name, (data, _) in files.items()}, executable), encode(audit)


def _wheel(name: str) -> bytes:
    """Create a real minimal wheel ZIP with complete metadata, tags and RECORD."""
    dist = name + "-1.0.dist-info/"
    files = {
        name + "/__init__.py": b'"""Synthetic metadata fixture, not a runtime."""\n',
        dist + "METADATA": f"Metadata-Version: 2.1\nName: {name}\nVersion: 1.0\n\n".encode(),
        dist + "WHEEL": b"Wheel-Version: 1.0\nRoot-Is-Purelib: true\nTag: py3-none-any\n",
    }
    if name == "amitools":
        files[dist + "entry_points.txt"] = b"[console_scripts]\nvamos = amitools:main\n"
        files[name + "/__init__.py"] += (
            b'def main():\n    raise RuntimeError("not runtime acceptance")\n')
    records = io.StringIO(newline="")
    writer = csv.writer(records, lineterminator="\n")
    for path, data in sorted(files.items()):
        hashed = base64.urlsafe_b64encode(bytes.fromhex(digest(data))).decode().rstrip("=")
        writer.writerow((path, "sha256=" + hashed, str(len(data))))
    writer.writerow((dist + "RECORD", "", ""))
    files[dist + "RECORD"] = records.getvalue().encode()
    return zip_bytes(files)


def _core(root: Path) -> bytes:
    """Compile three distinct ELF version observers, then build their exact inventory."""
    cc = shutil.which("cc")
    if cc is None:
        raise AssertionError("cc is required to construct miniature ELF observation data")
    source = root / "observer.c"
    source.write_text(_CORE_SOURCE, encoding="utf-8")
    files = {name: b"Synthetic core notice: " + name.encode() + b"\n" for name in NOTICES}
    executables = []
    for name in ("clc", "clorraine", "clorraine-lsp"):
        output = root / name
        _command([cc, "-std=c99", "-Wall", "-Wextra", "-Werror",
                  '-DTOOL_NAME="' + name + '"', '-DTOOL_VERSION="' + VERSION + '"',
                  str(source), "-o", str(output)], root, os.environ.copy())
        files["bin/" + name] = output.read_bytes()
        executables.append("bin/" + name)
    inventory = {path: {"bytes": len(data), "sha256": digest(data),
                        "executable": path in executables} for path, data in files.items()}
    files["core.json"] = encode({"schema": 1, "core_version": VERSION, "files": inventory})
    return tar_bytes(files, tuple(executables))


def build_inputs(root: Path) -> tuple[dict[str, bytes], dict]:
    """Return the twelve exact miniature inputs and their independently built lock."""
    lock, ndk = miniature_lock(root)
    sdk_files = {"bin/lha": (PROGRAM, "program"),
                 "bin/m68k-amigaos-gcc": (PROGRAM, "program"),
                 "include/fixture-base.h": (b"/* baseline fixture */\n", "header")}
    amiga, amiga_audit = _reviewed(sdk_files, lock)
    base = {path: {"type": "file", "sha256": record["sha256"]}
            for path, record in json.loads(amiga_audit)["files"].items()}
    prep = {
        "schema": 1, "lock_sha256": digest(encode(lock)),
        "archive": {"source": "seed", "bytes": len(ndk), "sha256": digest(ndk)},
        "base": base, "helpers": {"lha": {"path": "bin/lha", "sha256": digest(PROGRAM)}},
        "inputs": {"prepare.sh": digest(PROGRAM)}, "inventory": lock["preparation"]["inventory"],
        "notices": ["notices/ndk.txt"],
    }
    preparation, preparation_audit = _reviewed({
        "prepare_ndk.py": (b'raise SystemExit("synthetic failure fixture")\n', "preparation"),
        "preparation.json": (encode(prep), "preparation"),
        "prepare.sh": (PROGRAM, "preparation"), "bin/lha": (PROGRAM, "program"),
    }, lock)
    raw = {"core.tar": _core(root), "support-lock.json": encode(lock), "ndk": ndk,
           "amiga.tar": amiga, "amiga-audit.json": amiga_audit,
           "preparation.tar": preparation, "preparation-audit.json": preparation_audit,
           "clorraine.vsix": zip_bytes({
               "[Content_Types].xml": b'<?xml version="1.0"?><Types/>',
               "extension.vsixmanifest": b'<?xml version="1.0"?><PackageManifest/>',
               "extension/package.json": encode({"name": "clorraine", "publisher": "openlorraine",
                                                  "version": VERSION}),
               "extension/out/extension.js": b"exports.activate = () => {};\n"})}
    for abi in ("cp310", "cp312"):
        wheels = {}
        packages = {}
        for name in PACKAGE_NAMES:
            filename = "wheels/" + name + "-1.0-py3-none-any.whl"
            data = _wheel(name)
            wheels[filename] = (data, "wheel")
            packages[name] = {"version": "1.0", "source": "lha",
                              "purpose": "bootstrap" if name == "pip" else "runtime",
                              "filename": filename, "bytes": len(data), "sha256": digest(data),
                              "tags": ["py3-none-any"]}
        wheels["wheels.json"] = (encode({"schema": 1, "lock_sha256": digest(encode(lock)),
                                         "abi": abi, "packages": packages}), "preparation")
        raw[abi + ".tar"], raw[abi + "-audit.json"] = _reviewed(wheels, lock)
    return raw, lock
