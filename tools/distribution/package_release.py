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


def _strict_json(path: Path, label: str) -> object:
    """Read bounded JSON while rejecting duplicate keys before any field is used."""
    with ordinary(path) as source:
        data = source.read(16 * 1024 * 1024 + 1)
    if len(data) > 16 * 1024 * 1024:
        raise ValueError(label + ": metadata exceeds 16 MiB")
    def unique(pairs):
        result = {}
        for name, value in pairs:
            if name in result:
                raise ValueError(label + ": duplicate JSON key")
            result[name] = value
        return result
    try:
        return json.loads(data.decode("utf-8"), object_pairs_hook=unique,
                          parse_constant=_reject_number)
    except (UnicodeError, json.JSONDecodeError) as error:
        raise ValueError(label + ": invalid JSON or UTF-8") from error


def _reject_number(value: str) -> None:
    """Keep floating-point or nonfinite numbers out of release decisions."""
    raise ValueError("release metadata: numeric fields must be bounded integers")


def _ordinary_directory(path: Path, label: str) -> Path:
    """Require a caller-owned directory without following a final alias."""
    if path.is_symlink() or not path.is_dir():
        raise ValueError(label + ": expected an ordinary directory")
    return path


def _current_core_version() -> str:
    """Read the one authoritative compiler version record."""
    manifest = (_ROOT / "compiler/Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'(?m)^version = "([0-9]+\.[0-9]+\.[0-9]+)"$', manifest)
    if match is None:
        raise ValueError("core: current version record is missing")
    return match.group(1)


def _current_extension() -> dict:
    """Read the current editor package record used to bind a VSIX payload."""
    value = json.loads((_ROOT / "tools/vscode/package.json").read_bytes())
    if not isinstance(value, dict):
        raise ValueError("extension: current package record is not an object")
    return value


def _verified_core(path: Path) -> dict:
    """Return embedded core metadata and require the current version."""
    with ordinary(path) as source, tarfile.open(fileobj=source, mode="r:") as archive:
        metadata = core_metadata(archive)
    if metadata["core_version"] != _current_core_version():
        raise ValueError("core: archive version differs from compiler/Cargo.toml")
    return metadata


def _verified_extension(path: Path) -> dict:
    """Return VSIX package metadata and require the current editor record."""
    package, _count, _bytes = inspect_vsix(path)
    current = _current_extension()
    for field in ("name", "publisher", "version"):
        if package.get(field) != current.get(field):
            raise ValueError("VSIX: name/publisher/version differs from current extension metadata")
    return package


def _component_decisions(component: str, core: Path, vsix: Path,
                         existing: Path | None) -> dict:
    """Classify independently reusable payloads without changing existing bytes."""
    with tempfile.TemporaryDirectory(prefix=".release-verify-") as temporary:
        work = Path(temporary)
        snapshots = (work / "core.tar", work / "clorraine.vsix")
        _snapshot(core, snapshots[0])
        _snapshot(vsix, snapshots[1])
        current = _current_core_version()
        _verified_core(snapshots[0])
        package = _verified_extension(snapshots[1])
        result = {"schema": 1, "component_release": "components-" + component,
                  "core_version": current, "extension_version": package["version"],
                  "core": {"identity": component + "-core",
                           "bytes": snapshots[0].stat().st_size,
                           "sha256": _digest(snapshots[0]), "decision": "publish"},
                  "vsix": {"identity": component + "-vsix",
                           "bytes": snapshots[1].stat().st_size,
                           "sha256": _digest(snapshots[1]), "decision": "publish"}}
        if existing is None:
            return result
        _ordinary_directory(existing, "existing")
        for name, record in (("core.tar", result["core"]),
                             ("clorraine.vsix", result["vsix"])):
            path = existing / name
            if not os.path.lexists(path):
                continue
            with ordinary(path) as source:
                prior = source.read(1024 * 1024 * 1024 + 1)
            if len(prior) > 1024 * 1024 * 1024:
                raise ValueError("identity: existing asset exceeds the size ceiling")
            if (len(prior) != record["bytes"]
                    or hashlib.sha256(prior).hexdigest() != record["sha256"]):
                raise ValueError("identity: existing " + name
                                 + " bytes differ; use a new identity")
            record["decision"] = "reuse"
        return result


def _approval(value: object) -> dict:
    """Require the complete approval record and its exact field set."""
    fields = {"schema", "release", "manifest_sha256", "approved",
              "unresolved_major", "st56"}
    if not isinstance(value, dict) or set(value) != fields:
        raise ValueError("approval: unknown or missing fields")
    if type(value["schema"]) is not int or value["schema"] != 1:
        raise ValueError("approval: unsupported schema")
    if not isinstance(value["manifest_sha256"], str) or re.fullmatch(
            r"[0-9a-f]{64}", value["manifest_sha256"]) is None:
        raise ValueError("approval: invalid manifest hash")
    if type(value["approved"]) is not bool:
        raise ValueError("approval: approved must be boolean")
    if type(value["unresolved_major"]) is not int or value["unresolved_major"] < 0:
        raise ValueError("approval: unresolved_major must be a nonnegative integer")
    if value["st56"] not in ("pending", "passed", "failed"):
        raise ValueError("approval: invalid ST-56 state")
    return value


def _asset_file(assets: Path, asset: dict) -> Path:
    """Resolve one referenced local asset by its literal published filename."""
    name = asset["url"].rsplit("/", 1)[-1]
    if not name or name in (".", "..") or "/" in name or "\\" in name:
        raise ValueError("asset: invalid published filename")
    return assets / name


def _promotion(args: argparse.Namespace) -> dict:
    """Evaluate pre-promotion eligibility without publishing or mutating assets."""
    with ordinary(args.manifest) as source:
        data = source.read(16 * 1024 * 1024 + 1)
    manifest = parse_manifest(data)
    approval = _approval(_strict_json(args.approval, "approval"))
    assets = _ordinary_directory(args.assets, "assets")
    blockers: list[str] = []
    if approval["release"] != manifest["release"]:
        blockers.append("approval release differs from the candidate")
    if approval["manifest_sha256"] != hashlib.sha256(data).hexdigest():
        blockers.append("approval manifest hash differs from the candidate")
    if approval["approved"] is not True:
        blockers.append("explicit publication approval is absent")
    if approval["unresolved_major"] != 0:
        blockers.append("unresolved major findings remain")
    for asset in manifest["assets"]:
        if asset["kind"] == "ndk":
            continue  # Upstream source is referenced by the lock, not republished here.
        path = _asset_file(assets, asset)
        if not os.path.lexists(path):
            blockers.append("missing payload: " + asset["identity"])
            continue
        try:
            with ordinary(path) as source:
                size = os.fstat(source.fileno()).st_size
                observed = hashlib.sha256()
                for block in iter(lambda: source.read(1024 * 1024), b""):
                    observed.update(block)
            if size != asset["bytes"]:
                blockers.append("size mismatch: " + asset["identity"])
            if observed.hexdigest() != asset["sha256"]:
                blockers.append("hash mismatch: " + asset["identity"])
            count, expanded = ((0, 0) if asset["root"] is None else
                               (inspect_vsix(path)[1:] if asset["kind"] == "vsix"
                                else measure_tar(path)))
            if (count, expanded) != (asset["entries"], asset["expanded_bytes"]):
                blockers.append("inventory mismatch: " + asset["identity"])
        except (OSError, ValueError, tarfile.TarError, zipfile.BadZipFile):
            blockers.append("unreadable payload: " + asset["identity"])
    ready = not blockers
    passed = ready and approval["st56"] == "passed"
    return {"schema": 1, "release": manifest["release"], "promotion_ready": ready,
            "final_completion_ready": passed, "blockers": blockers,
            "st56": approval["st56"],
            "post_publication": [] if passed else ["ST-56"]}


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


def validate_inputs(args: argparse.Namespace) -> dict:
    """Validate literal dispatch values before a workflow uses them in paths or commands."""
    if args.support is None:
        raise ValueError("support: required identity is missing")
    if (args.release is None) != (args.components is None):
        raise ValueError("release/components: supply both distribution values or neither")
    if args.release is not None:
        validate_release_tag(args.release)
    if (args.components is not None and _ID.fullmatch(args.components) is None
            or _ID.fullmatch(args.support) is None):
        raise ValueError("identity: expected a component/support token")
    if re.fullmatch(r"[0-9a-f]{40}", args.commit) is None:
        raise ValueError("commit: expected a full lowercase commit identity")
    record = {"schema": 1, "support": args.support, "commit": args.commit}
    if args.release is not None:
        record.update(release=args.release, components=args.components)
    for name in ("previous_core", "previous_vsix"):
        value = getattr(args, name)
        if value is not None:
            if value != "none" and _ID.fullmatch(value) is None:
                raise ValueError(name.replace("_", "-") + ": expected a component token or none")
            record[name] = value
    if args.support_run_id is not None:
        if re.fullmatch(r"[1-9][0-9]{0,19}", args.support_run_id) is None:
            raise ValueError("support-run-id: expected a positive decimal run identity")
        record["support_run_id"] = args.support_run_id
    if args.approved_manifest_sha256 is not None:
        value = args.approved_manifest_sha256
        if value != "none" and re.fullmatch(r"[0-9a-f]{64}", value) is None:
            raise ValueError("approved-manifest-sha256: expected a lowercase digest or none")
        record["approved_manifest_sha256"] = value
    return record


def main(argv: list[str] | None = None) -> int:
    """Run a maintainer-only release operation; return usage/content/success status."""
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
    validate = operations.add_parser("validate", allow_abbrev=False)
    for name in ("release", "components", "support", "commit",
                 "previous-core", "previous-vsix", "support-run-id",
                 "approved-manifest-sha256"):
        validate.add_argument("--" + name, required=False, dest=name.replace("-", "_"))
    verify = operations.add_parser("verify-component", allow_abbrev=False)
    verify.add_argument("--components", required=True)
    for name in ("core", "vsix"):
        verify.add_argument("--" + name, required=True, type=Path)
    verify.add_argument("--existing", required=False, type=Path)
    promotion = operations.add_parser("promotion", allow_abbrev=False)
    for name in ("manifest", "assets", "approval"):
        promotion.add_argument("--" + name, required=True, type=Path)
    args = parser.parse_args(arguments)
    try:
        if args.operation == "assemble":
            assemble(args)
        elif args.operation == "validate":
            record = validate_inputs(args)
            sys.stdout.buffer.write(encoded(record))
        elif args.operation == "verify-component":
            if _ID.fullmatch(args.components) is None:
                raise ValueError("identity: expected a component token")
            record = _component_decisions(args.components, args.core, args.vsix, args.existing)
            sys.stdout.buffer.write(encoded(record))
        else:
            record = _promotion(args)
            sys.stdout.buffer.write(encoded(record))
            if not record["promotion_ready"] or record["st56"] == "failed":
                return 1
    except (OSError, ValueError, KeyError, TypeError, RuntimeError, RecursionError,
            SupportError, tarfile.TarError, zipfile.BadZipFile) as error:
        message = "".join(character if character.isprintable()
                          else f"\\u{ord(character):04x}" for character in str(error))
        print("package-release: " + message[:8192], file=sys.stderr)
        return 1
    if args.operation == "assemble":
        print(args.output.absolute())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
