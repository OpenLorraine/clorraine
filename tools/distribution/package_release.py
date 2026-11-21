#!/usr/bin/env python3
"""Assemble deterministic local release-entry bytes from already built immutable components."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import sys
import tarfile
import tempfile
import zipfile

from package_core import core_metadata
from support import SupportError, load_lock
from support_audit import audit_payload, load_audit
from support_build import verify_wheels

# Maintainer execution starts at tools/distribution, not at the checkout root.
# This is producer-only resolution; the emitted ZIP has its own closed packages.
_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(_ROOT))

from tools.distribution.manifest import (  # noqa: E402
    BASE_PACKAGES, parse_manifest, validate_release_tag,
)
from tools.setup.native_archive import (  # noqa: E402
    extract_tar, inspect_vsix, measure_tar, ordinary,
)
from tools.setup.native_state import encoded, publish_directory  # noqa: E402


_INPUTS = (
    "support-lock.json", "amiga.tar", "amiga-audit.json",
    "preparation.tar", "preparation-audit.json",
    "cp310.tar", "cp310-audit.json", "cp312.tar", "cp312-audit.json",
)
"""Only these nine support inputs are read; directory extras are not release assets."""
_MODULES = (
    "tools/setup/install_native.py", "tools/setup/native_archive.py",
    "tools/setup/native_state.py", "tools/distribution/manifest.py",
)
"""Runtime closure; no SDK producer or checkout-only Python dependency belongs in the helper."""
_PACKAGES = ("tools/__init__.py", "tools/setup/__init__.py", "tools/distribution/__init__.py")
"""Explicit empty initializers prevent fallback to a similarly named installed package."""
_DOWNLOAD = "https://github.com/OpenLorraine/clorraine/releases/download/"
"""Final immutable release namespace, emitted before local exact-byte proof."""
_ID = re.compile(r"[a-z0-9][a-z0-9-]*")
"""Component/support identities are literal tokens, never pathnames or shell text."""


def _snapshot(source: Path, destination: Path) -> None:
    """Copy one ordinary input privately before validation/query/archive measurements."""
    with ordinary(source) as input_file, destination.open("xb") as output:
        shutil.copyfileobj(input_file, output, length=65536)


def _digest(path: Path) -> str:
    """Stream a private ordinary snapshot's SHA-256."""
    digest = hashlib.sha256()
    with ordinary(path) as source:
        for block in iter(lambda: source.read(65536), b""):
            digest.update(block)
    return digest.hexdigest()


def _asset(path: Path, identity: str, kind: str, namespace: str,
           root: str | None = None, audit: str | None = None) -> dict:
    """Measure exact immutable bytes and construct one final-URL component record."""
    count, expanded = measure_tar(path) if root == "." else (0, 0)
    if kind == "vsix":
        _package, count, expanded = inspect_vsix(path)
    return {"identity": identity, "kind": kind, "url": _DOWNLOAD + namespace + "/" + path.name,
            "bytes": path.stat().st_size, "sha256": _digest(path), "root": root,
            "entries": count, "expanded_bytes": expanded, "audit": audit}


def _helper(destination: Path) -> None:
    """Write a fixed-metadata closed stdlib ZIP from exact source snapshots."""
    files = {name: b"" for name in _PACKAGES}
    files["__main__.py"] = (b"from tools.setup.install_native import main\n"
                            b"raise SystemExit(main())\n")
    for name in _MODULES:
        with ordinary(_ROOT / name) as source:
            files[name] = source.read(1024 * 1024 + 1)
        if len(files[name]) > 1024 * 1024:
            raise ValueError("helper: excessive runtime module length")
        compile(files[name], name, "exec")
    with zipfile.ZipFile(destination, "x", compression=zipfile.ZIP_DEFLATED,
                         compresslevel=9) as archive:
        for name, data in sorted(files.items()):
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = (stat.S_IFREG | 0o644) << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, data, compresslevel=9)
    destination.chmod(0o755)


def assemble(args: argparse.Namespace) -> None:
    """Validate snapshots and publish exactly shell/helper/manifest without clobbering output."""
    validate_release_tag(args.release)
    if any(_ID.fullmatch(value) is None for value in (args.components, args.support)):
        raise ValueError("identity: expected a component/support token")
    output = args.output.absolute()
    if os.path.lexists(output):
        raise ValueError("output: destination already exists")
    if args.support_inputs.is_symlink() or not args.support_inputs.is_dir():
        raise ValueError("support inputs: expected an ordinary directory")
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".release-assemble-", dir=output.parent) as temporary:
        work = Path(temporary)
        snapshots = work / "inputs"
        snapshots.mkdir()
        _snapshot(args.core, snapshots / "core.tar")
        _snapshot(args.vsix, snapshots / "clorraine.vsix")
        for name in _INPUTS:
            _snapshot(args.support_inputs / name, snapshots / name)
        lock, lock_digest = load_lock(snapshots / "support-lock.json")
        with ordinary(snapshots / "core.tar") as source, tarfile.open(fileobj=source, mode="r:") as archive:
            core = core_metadata(archive)
        package, _count, _bytes = inspect_vsix(snapshots / "clorraine.vsix")
        expected_package = json.loads((_ROOT / "tools/vscode/package.json").read_bytes())
        for field in ("name", "publisher", "version"):
            if package.get(field) != expected_package[field]:
                raise ValueError("VSIX: name/publisher/version differs from current extension metadata")
        assets = [
            _asset(snapshots / "core.tar", args.components + "-core", "core",
                   "components-" + args.components, "."),
            _asset(snapshots / "clorraine.vsix", args.components + "-vsix", "vsix",
                   "components-" + args.components, "extension"),
            _asset(snapshots / "support-lock.json", args.support + "-lock", "support-lock",
                   "support-" + args.support),
        ]
        # Validate complete core contents as well as its embedded inventory.
        extract_tar(snapshots / "core.tar", work / "core", assets[0])
        audits = []
        for name in ("amiga", "preparation", "cp310", "cp312"):
            kind = "wheels-" + name if name.startswith("cp") else name
            audit_id = args.support + "-" + name + "-audit"
            asset = _asset(snapshots / (name + ".tar"), args.support + "-" + name,
                           kind, "support-" + args.support, ".", audit_id)
            audit = load_audit(snapshots / (name + "-audit.json"))
            if audit.get("lock_sha256") != lock_digest:
                raise ValueError(name + ": audit lock identity mismatch")
            extracted = work / name
            observed = extract_tar(snapshots / (name + ".tar"), extracted, asset, audit["files"])
            audit_payload(lock, lock_digest, extracted, audit)
            if name.startswith("cp"):
                verify_wheels(extracted, lock, lock_digest, name, observed)
            assets.append(asset)
            audits.append(_asset(snapshots / (name + "-audit.json"), audit_id,
                                 "support-audit", "support-" + args.support))
        assets.extend(audits)
        ndk = lock["sources"][lock["preparation"]["archive"]]
        assets.append({"identity": args.support + "-ndk", "kind": "ndk", "url": ndk["url"],
                       "bytes": ndk["bytes"], "sha256": ndk["sha256"], "root": None,
                       "entries": 0, "expanded_bytes": 0, "audit": None})
        manifest = {
            "schema": 1, "release": args.release, "core_version": core["core_version"],
            "extension_version": package["version"],
            "platform": {"os": "linux", "architecture": "x86_64",
                         "cohorts": {"22.04": "cp310", "24.04": "cp312"}},
            "assets": assets,
            "support": {
                "identity": args.support, "lock": args.support + "-lock",
                "amiga": args.support + "-amiga", "preparation": args.support + "-preparation",
                "wheels": {abi: args.support + "-" + abi for abi in ("cp310", "cp312")},
                "ndk": args.support + "-ndk", "profiles": ["hosted-amiga", "hosted-amiga-13"],
                "prepared_inventory_sha256": hashlib.sha256(
                    encoded(lock["preparation"]["inventory"])).hexdigest()},
            "prerequisites": {
                cohort: sorted(set(lock["output"]["runtime_packages"][cohort]) | BASE_PACKAGES)
                for cohort in ("22.04", "24.04")},
        }
        data = encoded(manifest)
        parse_manifest(data)
        candidate = work / "entry"
        candidate.mkdir(mode=0o755)
        _helper(candidate / "install.pyz")
        with ordinary(_ROOT / "tools/setup/install.sh") as source:
            shell = source.read(1024 * 1024).decode("utf-8")
        for key, value in {
                "URL": _DOWNLOAD + args.release + "/install.pyz",
                "SHA256": _digest(candidate / "install.pyz"),
                "BYTES": str((candidate / "install.pyz").stat().st_size)}.items():
            slot = "@CLORRAINE_HELPER_" + key + "@"
            if shell.count(slot) != 1:
                raise ValueError("helper: missing or repeated shell template slot")
            shell = shell.replace(slot, value)
        (candidate / "install.sh").write_text(shell, encoding="utf-8")
        (candidate / "install.sh").chmod(0o755)
        (candidate / "manifest.json").write_bytes(data)
        (candidate / "manifest.json").chmod(0o644)
        publish_directory(candidate, output)


def main(argv: list[str] | None = None) -> int:
    """Run the maintainer-only assemble command; return usage/content/success status."""
    arguments = list(sys.argv[1:] if argv is None else argv)
    # Argparse may echo unknown options before content validation runs. Reject
    # controls at the literal-argv boundary so even usage errors remain safe.
    if any(not isinstance(value, str) or any(not character.isprintable() for character in value)
           for value in arguments):
        print("package-release: arguments: terminal controls are not allowed", file=sys.stderr)
        return 1
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    operations = parser.add_subparsers(dest="operation", required=True)
    operation = operations.add_parser("assemble", allow_abbrev=False)
    for name in ("release", "components", "support"):
        operation.add_argument("--" + name, required=True)
    for name in ("core", "vsix", "support-inputs", "output"):
        operation.add_argument("--" + name, required=True, type=Path)
    args = parser.parse_args(arguments)
    try:
        assemble(args)
    except (OSError, ValueError, KeyError, TypeError, RuntimeError, RecursionError,
            SupportError, tarfile.TarError, zipfile.BadZipFile) as error:
        message = "".join(character if character.isprintable()
                          else f"\\u{ord(character):04x}" for character in str(error))
        print("package-release: " + message[:8192], file=sys.stderr)
        return 1
    print(args.output.absolute())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
