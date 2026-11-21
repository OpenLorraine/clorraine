"""Native installation orchestration using verified assets and system prerequisites."""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import resource
import shlex
import shutil
import subprocess
import sys
import tempfile
import tarfile
import zipfile

from tools.distribution.manifest import BASE_PACKAGES, ManifestError, validate_release_tag
from .native_archive import (
    extract_tar, fetch_asset, fetch_manifest, inspect_vsix, ordinary, read_json, verify_file,
)
from .native_state import OwnedRoot, durable_file, encoded, interpreter_identity


_COHORTS = {"22.04": (3, 10), "24.04": (3, 12)}
"""Actual Ubuntu/interpreter pairs; the installer never provisions a different Python."""
_PACKAGE = re.compile(r"[a-z0-9][a-z0-9+.-]+")
"""Only inert unqualified Debian package identifiers may enter printed apt guidance."""
_SONAME = re.compile(r"[A-Za-z0-9_+.-]+\.so(?:\.[0-9]+)*")
"""System loader names, never arbitrary paths or command expressions."""


class _UsageError(ValueError):
    """Malformed public invocation, distinguished from an installation failure."""


class _Parser(argparse.ArgumentParser):
    """Keep argparse diagnostics inside the installer's sanitized error boundary."""

    def error(self, message: str) -> None:
        """Raise instead of printing potentially unsafe option text or exiting early."""
        raise _UsageError(message)


def _safe_path(value: str, label: str) -> Path:
    """Canonicalize a selected filesystem location without accepting traversal or controls."""
    if (not value or any(not character.isprintable() for character in value)
            or ".." in Path(value).parts):
        raise _UsageError(f"{label}: choose a path without traversal or control characters")
    try:
        return Path(value).expanduser().resolve()
    except (OSError, RuntimeError) as error:
        raise _UsageError(f"{label}: cannot resolve the selected location") from error


def _arguments(argv: list[str] | None) -> argparse.Namespace:
    """Accept only the documented root/release options and preserve tags as literal data."""
    parser = _Parser(add_help=False, allow_abbrev=False)
    parser.add_argument("--root", default=str(Path.home() / ".clorraine"))
    parser.add_argument("--release")
    arguments = parser.parse_args(argv)
    arguments.root = _safe_path(arguments.root, "--root")
    if arguments.release is not None:
        try:
            validate_release_tag(arguments.release)
        except ManifestError as error:
            raise _UsageError(f"--release {arguments.release!r}: {error}") from error
    return arguments


def _cohort() -> tuple[str, str]:
    """Require the actual supported OS, architecture and CPython ABI before asset use."""
    operating_system = platform.freedesktop_os_release()
    ubuntu = operating_system.get("VERSION_ID", "")
    if operating_system.get("ID") != "ubuntu" or ubuntu not in _COHORTS:
        raise ValueError("platform: use Ubuntu 22.04/CPython 3.10 or Ubuntu 24.04/CPython 3.12")
    if platform.machine() != "x86_64":
        raise ValueError("architecture: this installer supports Linux x86_64 only")
    expected = _COHORTS[ubuntu]
    if (sys.implementation.name != "cpython" or sys.platform != "linux"
            or (sys.version_info.major, sys.version_info.minor) != expected):
        raise ValueError("Python: use CPython 3.10 on Ubuntu 22.04 or CPython 3.12 on Ubuntu 24.04")
    return ubuntu, "cp" + str(expected[0]) + str(expected[1])


def _limit_output() -> None:
    """Cap child-created files without raising a stricter inherited file-size limit."""
    finite = [value for value in resource.getrlimit(resource.RLIMIT_FSIZE) if value >= 0]
    limit = min([1024 ** 3, *finite])
    resource.setrlimit(resource.RLIMIT_FSIZE, (limit, limit))


def _run(argv: list, work: Path, step: str, *, input_text: str | None = None) -> str:
    """Execute literal arguments with finite elapsed time and bounded captured output.

    Logs stay in this invocation's owned work directory. Only a small error tail
    enters a diagnostic, and even successful commands may not return unbounded
    output to the installer.
    """
    with tempfile.TemporaryFile(dir=work) as stdout, tempfile.TemporaryFile(dir=work) as stderr:
        try:
            result = subprocess.run(
                [str(argument) for argument in argv], cwd=work,
                stdin=subprocess.DEVNULL if input_text is None else None,
                input=input_text.encode("utf-8") if input_text is not None else None,
                stdout=stdout, stderr=stderr, timeout=180, check=False,
                preexec_fn=_limit_output,
                env={**{key: value for key, value in os.environ.items()
                        if not key.startswith(("PYTHON", "PIP_", "LD_"))},
                     "HOME": str(work), "TMPDIR": str(work), "PATH": "/usr/bin:/bin",
                     "LC_ALL": "C", "PYTHONDONTWRITEBYTECODE": "1"},
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            raise ValueError(f"{step}: subprocess unavailable or timed out") from error
        if result.returncode:
            stderr.seek(max(0, stderr.tell() - 8192))
            detail = stderr.read(8192).decode("utf-8", errors="replace")
            raise ValueError(f"{step}: subprocess failed ({result.returncode}): {detail}")
        stdout.seek(0)
        data = stdout.read(1024 * 1024 + 1)
        if len(data) > 1024 * 1024:
            raise ValueError(f"{step}: subprocess output exceeds the safe bound")
        return data.decode("utf-8", errors="replace")


def _apt(packages) -> str:
    """Format a user-action command from validated package names, never execute it."""
    names = sorted(set(packages))
    if not names or any(not isinstance(name, str) or _PACKAGE.fullmatch(name) is None
                        for name in names):
        raise ValueError("prerequisites: invalid package identifier")
    return "sudo apt install " + shlex.join(names)


def _prerequisites(packages: list[str], runtime: dict, work: Path) -> None:
    """Check installed packages, real C headers/venv capability and declared runtime loading."""
    remedy = _apt(packages)
    query = shutil.which("dpkg-query")
    if query is None:
        raise ValueError("prerequisites: dpkg-query is required on the selected Ubuntu system")
    try:
        output = _run([query, "-W", "-f=${Package} ${db:Status-Status}\n", *packages],
                      work, "prerequisite packages")
        installed = {line.split()[0] for line in output.splitlines() if line.endswith(" installed")}
        if set(packages) - installed:
            raise ValueError("some declared packages are not installed")
    except ValueError as error:
        raise ValueError(f"prerequisite packages: {error}; run {remedy} and retry") from error

    compiler = shutil.which("cc") or shutil.which("gcc")
    if compiler is None or shutil.which("gcc") is None:
        raise ValueError("host compiler: install gcc and C headers; run " +
                         _apt(("gcc", "libc6-dev")) + " and retry")
    try:
        _run([compiler, "-std=c99", "-Werror", "-fsyntax-only", "-x", "c", "-"],
             work, "host compiler/header prerequisite",
             input_text="#include <stdio.h>\n#include <stdlib.h>\nint main(void) { return 0; }\n")
    except ValueError as error:
        raise ValueError(f"host compiler/C headers: {error}; run " +
                         _apt(("gcc", "libc6-dev")) + " and retry") from error
    try:
        _run([sys.executable, "-I", "-c", "import venv; import ensurepip"],
             work, "Python venv prerequisite")
    except ValueError as error:
        raise ValueError(f"Python venv: {error}; run {_apt(('python3-venv',))} and retry") from error

    for provider, record in runtime.items():
        for soname in record["sonames"]:
            if not isinstance(soname, str) or _SONAME.fullmatch(soname) is None:
                raise ValueError("runtime prerequisite: invalid system SONAME in support metadata")
            try:
                ctypes.CDLL(soname)
            except OSError as error:
                raise ValueError("runtime library prerequisite is unavailable; run " +
                                 _apt((provider,)) + " and retry") from error


def _display(message: object) -> str:
    """Keep diagnostics printable and bounded even when an OS or child reports unsafe text."""
    text = str(message)
    return "".join(character if character.isprintable() or character in "\n\t"
                   else f"\\u{ord(character):04x}" for character in text)[:8192]


def _hash(path: Path) -> str:
    """Hash one ordinary payload without following its final filename as an alias."""
    digest = hashlib.sha256()
    with ordinary(path) as source:
        for block in iter(lambda: source.read(65536), b""):
            digest.update(block)
    return digest.hexdigest()


def _relative(value: object) -> str:
    """Validate a metadata pathname before joining it onto private staging."""
    if (not isinstance(value, str) or not value or value.startswith("/")
            or "\\" in value or "!" in value
            or any(not character.isprintable() for character in value)
            or any(part in ("", ".", "..") for part in value.split("/"))):
        raise ValueError("payload metadata: unsafe relative pathname")
    return value


def _inventory(root: Path) -> dict:
    """Measure complete ordinary/link coverage without following directory links."""
    result = {}
    for directory, directories, files in os.walk(root, followlinks=False):
        parent = Path(directory)
        for name in [*directories, *files]:
            path = parent / name
            relative = _relative(path.relative_to(root).as_posix())
            if path.is_symlink():
                result[relative] = {"type": "symlink", "target": os.readlink(path)}
            elif path.is_file():
                result[relative] = {"type": "file", "sha256": _hash(path)}
            elif not path.is_dir():
                raise ValueError("payload inventory: unexpected special file")
    return result


def _lock(path: Path, manifest: dict) -> dict:
    """Bind consumed support metadata to the selected manifest and fixed runtime cohorts."""
    lock = read_json(path)
    if (set(lock) != {"schema", "sources", "inputs", "build", "preparation", "output"}
            or type(lock["schema"]) is not int or lock["schema"] != 1):
        raise ValueError("support lock: unsupported schema")
    output, preparation = lock["output"], lock["preparation"]
    if (not isinstance(output, dict) or output.get("architecture") != "x86_64"
            or set(output.get("python_abis", [])) != {"cp310", "cp312"}
            or set(output.get("runtimes", [])) != {"nix13", "nix20"}):
        raise ValueError("support lock: incompatible platform/ABI/profile set")
    for name, digest in preparation["inventory"].items():
        _relative(name)
        if not isinstance(digest, str) or re.fullmatch("[0-9a-f]{64}", digest) is None:
            raise ValueError("preparation inventory: invalid hash")
    if hashlib.sha256(encoded(preparation["inventory"])).hexdigest() != manifest["support"][
            "prepared_inventory_sha256"]:
        raise ValueError("support lock: prepared inventory identity mismatch")
    runtime = output["runtime_packages"]
    if set(runtime) != {"22.04", "24.04"}:
        raise ValueError("support lock: runtime cohort mismatch")
    for cohort, packages in runtime.items():
        if set(manifest["prerequisites"][cohort]) != set(packages) | BASE_PACKAGES:
            raise ValueError("support lock: prerequisite set differs from manifest")
        for name, record in packages.items():
            _apt((name,))
            if (set(record) != {"version", "sonames", "paths"}
                    or not isinstance(record["sonames"], list) or not isinstance(record["paths"], list)):
                raise ValueError("support lock: invalid runtime package record")
    source = lock["sources"][preparation["archive"]]
    ndk = next(asset for asset in manifest["assets"] if asset["kind"] == "ndk")
    if any(ndk[key] != source[key] for key in ("bytes", "sha256", "url")):
        raise ValueError("support lock: NDK identity mismatch")
    return lock


def _audit(path: Path, lock_digest: str, lock: dict) -> dict:
    """Bind delivered file obligations to the selected lock before archive use.

    Matching archive hashes alone cannot prove that required source, notices
    and review decisions were retained. Each component's obligations must name
    ordinary audited files with the right role and locked source origin.
    """
    audit = read_json(path)
    if (set(audit) != {"schema", "lock_sha256", "components", "files", "native"}
            or type(audit["schema"]) is not int or audit["schema"] != 1
            or audit["lock_sha256"] != lock_digest
            or any(not isinstance(audit[field], dict) for field in ("files", "components", "native"))):
        raise ValueError("support audit: schema or lock identity mismatch")
    for name, record in audit["files"].items():
        _relative(name)
        if (not isinstance(record, dict) or record.get("type") not in ("file", "symlink")
                or not isinstance(record.get("origins"), list) or not record["origins"]
                or not all(isinstance(origin, str) for origin in record["origins"])
                or len(set(record["origins"])) != len(record["origins"])):
            raise ValueError("support audit: invalid member record")
        field = "sha256" if record["type"] == "file" else "target"
        roles = ({"program", "header", "library", "wheel", "preparation", "notice", "review", "source"}
                 if field == "sha256" else {"link"})
        if (set(record) != {"type", field, "origins", "role"}
                or not isinstance(record["role"], str) or record["role"] not in roles
                or not isinstance(record[field], str)
                or (field == "sha256" and re.fullmatch("[0-9a-f]{64}", record[field]) is None)):
            raise ValueError("support audit: invalid file role or content identity")
        for origin in record["origins"]:
            if origin not in audit["components"] or origin not in lock["sources"]:
                raise ValueError("support audit: missing or unknown locked source origin")
    for origin, component in audit["components"].items():
        if (origin not in lock["sources"] or not isinstance(component, dict)
                or set(component) != {"review", "notices", "source"}):
            raise ValueError("support audit: invalid component obligations")
        terms = lock["sources"][origin]["licence"]
        if (not isinstance(terms, dict) or type(terms.get("source_required")) is not bool
                or not isinstance(terms.get("review"), str)):
            raise ValueError("support audit: invalid locked licence obligations")
        for field in ("notices", "source"):
            paths = component[field]
            if (not isinstance(paths, list) or not all(isinstance(name, str) for name in paths)
                    or len(set(paths)) != len(paths)
                    or (not paths and (field == "notices" or terms["source_required"]))):
                raise ValueError("support audit: missing or invalid " + field + " obligations")
        review = _relative(component["review"])
        obligations = [(review, "review")]
        obligations.extend((name, "notice") for name in component["notices"])
        obligations.extend((name, "source") for name in component["source"])
        for name, role in obligations:
            record = audit["files"].get(_relative(name))
            if (record is None or record["type"] != "file"
                    or record["role"] != role or origin not in record["origins"]):
                raise ValueError("support audit: missing matching " + role + " obligation")
        expected = lock["inputs"].get(_relative(terms["review"]))
        if (not isinstance(expected, str) or re.fullmatch("[0-9a-f]{64}", expected) is None
                or audit["files"][review]["sha256"] != expected):
            raise ValueError("support audit: review bytes differ from the locked decision")
    return audit["files"]


def _core(candidate: Path, observed: dict, version: str) -> None:
    """Verify exact core inventory, executable ELF identity and declared version."""
    metadata = read_json(candidate / "core.json")
    notices = {"LICENSE", "LICENSE-EXCEPTION.md", "NOTICE", "TRADEMARKS.md", "THIRD_PARTY_NOTICES.md"}
    binaries = {"bin/" + name for name in ("clc", "clorraine", "clorraine-lsp")}
    if (set(metadata) != {"schema", "core_version", "files"} or type(metadata["schema"]) is not int
            or metadata["schema"] != 1 or metadata["core_version"] != version
            or set(metadata["files"]) != notices | binaries
            or set(observed) != notices | binaries | {"core.json"}):
        raise ValueError("core: version or complete inventory mismatch")
    for name, record in metadata["files"].items():
        actual = observed[name]
        if (set(record) != {"bytes", "sha256", "executable"} or type(record["executable"]) is not bool
                or actual["type"] != "file" or record["bytes"] != actual["bytes"]
                or record["sha256"] != actual["sha256"]
                or record["executable"] != bool(actual["mode"] & 0o111)
                or record["executable"] != (name in binaries)):
            raise ValueError("core: member hash/length/executable mismatch")
        if name in binaries:
            with ordinary(candidate / name) as source:
                header = source.read(20)
            if (len(header) != 20 or header[:7] != b"\x7fELF\x02\x01\x01"
                    or int.from_bytes(header[16:18], "little") not in (2, 3)
                    or int.from_bytes(header[18:20], "little") != 62):
                raise ValueError("core: expected Linux x86_64 executable")


def _wheelset(directory: Path, abi: str, lock_digest: str) -> dict:
    """Bind offline wheel pins to the authenticated ABI-specific inventory."""
    metadata = read_json(directory / "wheels.json")
    names = {"amitools", "machine68k", "greenlet", "pysdl2", "pip"}
    if (set(metadata) != {"schema", "lock_sha256", "abi", "packages"}
            or type(metadata["schema"]) is not int or metadata["schema"] != 1
            or metadata["lock_sha256"] != lock_digest or metadata["abi"] != abi
            or set(metadata["packages"]) != names):
        raise ValueError("wheels: invalid ABI/package/lock manifest")
    for name, record in metadata["packages"].items():
        if (set(record) != {"version", "source", "purpose", "filename", "bytes", "sha256", "tags"}
                or not isinstance(record["version"], str)
                or re.fullmatch(r"[0-9][A-Za-z0-9.+!_-]*", record["version"]) is None
                or record["purpose"] != ("bootstrap" if name == "pip" else "runtime")
                or not _relative(record["filename"]).startswith("wheels/")
                or not record["filename"].endswith(".whl")
                or not isinstance(record["tags"], list) or not record["tags"]):
            raise ValueError("wheels: invalid offline package pin")
        verify_file(directory / record["filename"], {**record, "kind": "wheel"})
    declared = {record["filename"] for record in metadata["packages"].values()}
    if declared != {path.relative_to(directory).as_posix() for path in directory.rglob("*.whl")}:
        raise ValueError("wheels: unindexed or missing wheel")
    return metadata


def _assemble(root: OwnedRoot, manifest: dict, manifest_bytes: bytes, abi: str,
              ubuntu: str, work: Path) -> Path:
    """Assemble verified inputs in fresh private storage, without completing or activating it."""
    assets = {asset["identity"]: asset for asset in manifest["assets"]}
    support = manifest["support"]

    def obtain(identity: str) -> Path:
        """Use a revalidated cache snapshot, never a mutable shared extraction."""
        return root.input(assets[identity], work, fetch_asset)

    lock_path = obtain(support["lock"])
    lock_digest = _hash(lock_path)
    lock = _lock(lock_path, manifest)
    _prerequisites(manifest["prerequisites"][ubuntu], lock["output"]["runtime_packages"][ubuntu], work)
    candidate = work / "candidate"
    core_asset = next(asset for asset in assets.values() if asset["kind"] == "core")
    observed = extract_tar(obtain(core_asset["identity"]), candidate, core_asset)
    _core(candidate, observed, manifest["core_version"])
    vsix_asset = next(asset for asset in assets.values() if asset["kind"] == "vsix")
    vsix = obtain(vsix_asset["identity"])
    package, _count, _expanded = inspect_vsix(vsix, vsix_asset)
    if (package.get("name") != "clorraine" or package.get("publisher") != "openlorraine"
            or package.get("version") != manifest["extension_version"]):
        raise ValueError("VSIX: extension identity/version mismatch")
    shutil.copyfile(vsix, candidate / "clorraine.vsix")
    destinations = {"amiga": candidate / "sdk/amiga", "preparation": work / "preparation",
                    "wheels": work / "wheelset"}
    audits = {}
    for field, identity in (("amiga", support["amiga"]), ("preparation", support["preparation"]),
                            ("wheels", support["wheels"][abi])):
        asset = assets[identity]
        records = _audit(obtain(asset["audit"]), lock_digest, lock)
        extract_tar(obtain(identity), destinations[field], asset, records)
        audits[field] = records
    preparation, sdk, wheelset = (destinations[name] for name in ("preparation", "amiga", "wheels"))
    prep = read_json(preparation / "preparation.json")
    if (prep.get("schema") != 1 or prep.get("lock_sha256") != lock_digest
            or prep.get("inventory") != lock["preparation"]["inventory"]
            or prep.get("base") != _inventory(sdk)):
        raise ValueError("preparation: baseline/inventory/lock mismatch")
    for name, digest in prep["inputs"].items():
        if _hash(preparation / _relative(name)) != digest:
            raise ValueError("preparation: input identity mismatch")
    ndk = obtain(support["ndk"])
    _run([sys.executable, "-I", preparation / "prepare_ndk.py", "--archive", ndk,
          "--destination", sdk], work, "NDK preparation")
    expected = {name: {key: value for key, value in record.items()
                      if key in ("type", "sha256", "target")} for name, record in audits["amiga"].items()}
    expected.update({name: {"type": "file", "sha256": digest}
                     for name, digest in lock["preparation"]["inventory"].items()})
    receipt = read_json(sdk / ".ndk-prepared.json")
    actual = _inventory(sdk)
    actual.pop(".ndk-prepared.json", None)
    if actual != expected or receipt != {
            "schema": 1, "lock_sha256": lock_digest, "ndk_sha256": assets[support["ndk"]]["sha256"],
            "inventory_sha256": support["prepared_inventory_sha256"]}:
        raise ValueError("NDK preparation: final inventory/completion mismatch")
    metadata = _wheelset(wheelset, abi, lock_digest)
    pins = work / "wheel-pins.txt"
    durable_file(pins, "".join(
        f"{name}=={record['version']} --hash=sha256:{record['sha256']}\n"
        for name, record in sorted(metadata["packages"].items())).encode())
    venv = candidate / "sdk/vamos"
    _run([sys.executable, "-I", "-m", "venv", venv], work, "Python venv creation")
    python = venv / "bin/python"
    _run([python, "-I", "-m", "pip", "--isolated", "--no-cache-dir", "install", "--no-index",
          "--find-links", wheelset / "wheels", "--only-binary=:all:", "--no-deps",
          "--require-hashes", "--force-reinstall", "-r", pins], work, "offline wheel installation")
    _run([python, "-I", "-m", "pip", "--isolated", "check"], work, "offline wheel dependency check")
    probe = ("import importlib.metadata,json,site,sys;"
             "print(json.dumps([sys.prefix,sys.flags.isolated,site.ENABLE_USER_SITE,"
             "{n:importlib.metadata.version(n) for n in sys.argv[1:]}]))")
    observed = json.loads(_run([python, "-I", "-c", probe, *sorted(metadata["packages"])],
                              work, "installed wheel identity"))
    if observed != [str(venv), 1, False,
                    {name: record["version"] for name, record in metadata["packages"].items()}]:
        raise ValueError("offline wheels: actual environment identity mismatch")
    shutil.copyfile(wheelset / "wheels.json", venv / "wheels.json")
    durable_file(candidate / "manifest.json", manifest_bytes)
    durable_file(candidate / "installation.json", encoded({
        "schema": 1, "core_version": manifest["core_version"], "support_lock_sha256": lock_digest,
        "ndk_sha256": assets[support["ndk"]]["sha256"],
        "prepared_inventory_sha256": support["prepared_inventory_sha256"], "python_abi": abi}))
    return candidate


def _smoke(candidate: Path, manifest: dict, work: Path) -> None:
    """Require actual candidate versions and host/both-vamos Hello output before completion."""
    for name in ("clc", "clorraine", "clorraine-lsp"):
        output = _run([candidate / "bin" / name, "--version"], work, name + " version smoke")
        if output.strip() != name + " " + manifest["core_version"]:
            raise ValueError(name + ": candidate version smoke mismatch")
    program = work / "hello.clo"
    program.write_text('module installer.Hello;\nimport { println } from std.io;\n'
                       'func main(): i32 { println("Hello, Clorraine!"); return 0; }\n',
                       encoding="utf-8")
    for profile in ("host", "hosted-amiga", "hosted-amiga-13"):
        command = [candidate / "bin/clorraine", "run", "-p", profile, program]
        if profile != "host":
            # Explicit selection outranks a developer's inherited SDK override.
            # The host profile neither needs nor accepts an SDK selection.
            command.extend(["--sdk-root", candidate / "sdk/amiga"])
        output = _run(command, work, profile + " required smoke")
        if output != "Hello, Clorraine!\n":
            raise ValueError(profile + ": required smoke output mismatch")


def main(argv: list[str] | None = None) -> int:
    """Install or revalidate an exact distribution using only the two public options.

    For example, ``main(["--root", "/home/user/Clorraine SDK", "--release",
    "v1.2.0-r1"])`` runs one locked transaction. Return 2 for malformed
    arguments, 1 for a failed transaction, and 0 only after durable activation.
    No package manager, shell-startup change or extension installation is run.
    """
    try:
        arguments = _arguments(argv)
    except (_UsageError, OSError, ValueError) as error:
        print("clorraine: " + _display(error), file=sys.stderr)
        return 2
    try:
        ubuntu, abi = _cohort()
        identity = interpreter_identity(abi)
        with OwnedRoot(arguments.root) as root, root.work() as work:
            _prerequisites(sorted(BASE_PACKAGES), {}, work)
            manifest, manifest_bytes = fetch_manifest(arguments.release)
            candidate = root.completed(manifest_bytes, identity)
            if candidate is None:
                candidate = _assemble(root, manifest, manifest_bytes, abi, ubuntu, work)
                candidate = root.stage(candidate, manifest["release"], abi)
                _smoke(candidate, manifest, work)
                root.complete(candidate, manifest_bytes, identity)
            else:
                _smoke(candidate, manifest, work)
            root.activate(candidate)
            print("Clorraine " + manifest["release"] + " installed at " + str(root.root))
            print("export PATH=" + shlex.quote(str(root.root / "bin")) + ':\"$PATH\"')
            print("code --install-extension " + shlex.quote(str(candidate / "clorraine.vsix")))
        return 0
    except (OSError, ValueError, RuntimeError, KeyError, TypeError, AttributeError,
            RecursionError, tarfile.TarError, zipfile.BadZipFile, subprocess.SubprocessError) as error:
        print("clorraine: " + _display(error) + "\nCheck the named input or prerequisite and retry; "
              "any previous selected installation is retained.", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
