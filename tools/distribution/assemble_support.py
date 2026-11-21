#!/usr/bin/env python3
"""Thin maintainer entry for the proved support payload assembly API."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile

from package_support import assemble_payload, SupportError
from support import load_lock
from support_audit import load_audit


KINDS = ("sdk", "cp310", "cp312", "preparation")
"""Payload kinds matching the immutable support archive names."""


def _digest(path: Path) -> str:
    """Hash one ordinary producer file without loading an archive."""
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _ordinary(path: Path, label: str) -> Path:
    """Require a readable ordinary file; do not follow a final alias."""
    if path.is_symlink() or not path.is_file():
        raise SupportError(label + ": expected an ordinary file")
    return path


def _preparation_bundle(lock_path: Path, sdk_payload: Path, output: Path) -> Path:
    """Assemble the exact preparation producer from locked inputs and SDK helpers.

    The bundle is not a release asset. It is the input the proved
    ``assemble_payload`` preparation branch expects: the locked script,
    NDK-derived inputs, the three helper executables and the descriptor without
    the SDK-relative ``base`` field, which assembly adds later.
    """
    if os.getuid() == 0:
        raise SupportError("preparation bundle: require a non-root maintainer")
    lock, lock_digest = load_lock(lock_path)
    sdk_payload = sdk_payload.resolve(strict=True)
    if sdk_payload.is_symlink() or not sdk_payload.is_dir():
        raise SupportError("sdk payload: expected an ordinary directory")
    output = output.absolute()
    if os.path.lexists(output):
        raise SupportError("preparation bundle: refusing an existing output")
    preparation = lock["preparation"]
    expected_inputs = {name: digest for name, digest in lock["inputs"].items()
                       if name == preparation["script"] or name.startswith("ndk-inputs/")}
    helpers = {}
    for identity, record in preparation["helpers"].items():
        source = _ordinary(sdk_payload / "bin" / identity, identity)
        helpers[identity] = {"path": "helpers/" + identity, "sha256": _digest(source)}
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".preparation-bundle-", dir=output.parent) as temp:
        bundle = Path(temp) / "bundle"
        bundle.mkdir()
        (bundle / "helpers").mkdir()
        script = _ordinary(Path(__file__).with_name("prepare_ndk.py"), "prepare_ndk.py")
        if _digest(script) != expected_inputs[preparation["script"]]:
            raise SupportError("prepare_ndk.py: differs from the lock")
        shutil.copyfile(script, bundle / "prepare_ndk.py")
        (bundle / "prepare_ndk.py").chmod(0o644)
        for name in sorted(expected_inputs):
            if name == preparation["script"]:
                continue
            relative = Path(name)
            if relative.is_absolute() or ".." in relative.parts:
                raise SupportError("preparation input: unsafe path")
            source = _ordinary(Path(__file__).with_name("inputs") / relative, name)
            if _digest(source) != expected_inputs[name]:
                raise SupportError(name + ": differs from the lock")
            destination = bundle / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)
            destination.chmod(0o644)
        for identity, record in helpers.items():
            source = _ordinary(sdk_payload / "bin" / identity, identity)
            destination = bundle / record["path"]
            shutil.copyfile(source, destination)
            destination.chmod(0o755)
        archive_id = preparation["archive"]
        archive_record = lock["sources"][archive_id]
        metadata = {
            "schema": 1,
            "lock_sha256": lock_digest,
            "archive": {"source": archive_id, "bytes": archive_record["bytes"],
                        "sha256": archive_record["sha256"]},
            "helpers": helpers,
            "inputs": expected_inputs,
            "inventory": preparation["inventory"],
            "notices": sorted(name for name in preparation["inventory"]
                              if name.endswith("/README")),
        }
        (bundle / "preparation.json").write_text(
            json.dumps(metadata, sort_keys=True, indent=2) + "\n", encoding="utf-8")
        (bundle / "preparation.json").chmod(0o644)
        os.rename(bundle, output)
    return output


def main(arguments: list[str] | None = None) -> int:
    """Run one assembly and report its destination; never build or publish."""
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    operation = parser.add_subparsers(dest="operation", required=True)
    assemble = operation.add_parser("assemble", allow_abbrev=False)
    assemble.add_argument("--kind", choices=KINDS, required=True)
    assemble.add_argument("--lock", type=Path, required=True)
    assemble.add_argument("--sources", type=Path, required=True)
    assemble.add_argument("--producer", type=Path, required=True)
    assemble.add_argument("--sdk-base", type=Path)
    assemble.add_argument("--output", type=Path, required=True)
    bundle = operation.add_parser("bundle-preparation", allow_abbrev=False)
    bundle.add_argument("--lock", type=Path, required=True)
    bundle.add_argument("--sdk-payload", type=Path, required=True)
    bundle.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(arguments)
    try:
        if args.operation == "bundle-preparation":
            print(_preparation_bundle(args.lock, args.sdk_payload, args.output))
            return 0
        if args.kind == "preparation" and args.sdk_base is None:
            raise SupportError("preparation: --sdk-base is required")
        if args.kind != "preparation" and args.sdk_base is not None:
            raise SupportError("--sdk-base is only valid for preparation")
        if args.kind == "preparation":
            if not (args.producer / "preparation.json").is_file():
                raise SupportError("preparation: producer is not a preparation bundle")
        elif (args.producer / "preparation.json").is_file():
            raise SupportError("producer is a preparation bundle, not --kind " + args.kind)
        if args.kind == "sdk" and (args.producer / "wheels.json").is_file():
            raise SupportError("producer is a wheel payload, not --kind sdk")
        if args.kind in ("cp310", "cp312"):
            manifest = load_audit(args.producer / "wheels.json")
            if not isinstance(manifest, dict) or manifest.get("abi") != args.kind:
                raise SupportError("producer ABI does not match --kind")
        if os.getuid() == 0:
            raise SupportError("assembly: require a non-root maintainer")
        output = assemble_payload(args.lock, args.sources, args.producer,
                                  args.output, sdk_base=args.sdk_base)
    except (SupportError, OSError, ValueError, RuntimeError) as error:
        print("assemble-support: " + str(error)[:8192], file=sys.stderr)
        return 1
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
