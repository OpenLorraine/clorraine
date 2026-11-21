#!/usr/bin/env python3
"""Thin maintainer entry for the proved support payload assembly API."""

from __future__ import annotations

import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import sys

from package_support import assemble_payload, SupportError
from support import load_lock
from support_audit import load_audit
from support_build import inventory


KINDS = ("sdk", "cp310", "cp312", "preparation")
"""Payload kinds matching the immutable support archive names."""


@contextmanager
def _ordinary_file(path: Path, label: str):
    """Open one ordinary file without following a final alias or blocking on a FIFO."""
    try:
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    except OSError as error:
        raise SupportError(label + ": expected an ordinary file") from error
    try:
        if not stat.S_ISREG(os.fstat(descriptor).st_mode):
            raise SupportError(label + ": expected an ordinary file")
        source = os.fdopen(descriptor, "rb")
    except BaseException:
        os.close(descriptor)
        raise
    with source:
        yield source


def _digest(path: Path, label: str) -> str:
    """Hash one ordinary producer file without loading an archive."""
    digest = hashlib.sha256()
    with _ordinary_file(path, label) as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _copy(source: Path, destination: Path, mode: int, label: str) -> None:
    """Copy one ordinary input into the private bundle with a fixed mode."""
    with _ordinary_file(source, label) as reader, destination.open("xb") as writer:
        shutil.copyfileobj(reader, writer, length=1024 * 1024)
    destination.chmod(mode)


def _preparation_bundle(lock_path: Path, sdk_payload: Path, output: Path) -> Path:
    """Assemble the exact preparation producer from locked inputs and SDK helpers.

    The bundle is not a release asset. It is the input the proved
    ``assemble_payload`` preparation branch expects: the locked script,
    NDK-derived inputs, the three helper executables, the SDK baseline inventory
    and the descriptor fields assembly later verifies.
    """
    if os.getuid() == 0:
        raise SupportError("preparation bundle: require a non-root maintainer")
    lock, lock_digest = load_lock(lock_path)
    sdk_payload = sdk_payload.absolute()
    if sdk_payload.is_symlink() or not sdk_payload.is_dir():
        raise SupportError("sdk payload: expected an ordinary directory")
    output = output.absolute()
    preparation = lock["preparation"]
    expected_inputs = {name: digest for name, digest in lock["inputs"].items()
                       if name == preparation["script"] or name.startswith("ndk-inputs/")}
    helpers = {}
    for identity in preparation["helpers"]:
        source = sdk_payload / "bin" / identity
        helpers[identity] = {"path": "helpers/" + identity,
                             "sha256": _digest(source, identity)}
    output.parent.mkdir(parents=True, exist_ok=True)
    try:
        output.mkdir(mode=0o755)
    except FileExistsError as error:
        raise SupportError("preparation bundle: refusing an existing output") from error
    try:
        (output / "helpers").mkdir()
        _copy(Path(__file__).with_name("prepare_ndk.py"),
              output / "prepare_ndk.py", 0o644, "prepare_ndk.py")
        if _digest(output / "prepare_ndk.py", "prepare_ndk.py") != expected_inputs[preparation["script"]]:
            raise SupportError("prepare_ndk.py: differs from the lock")
        for name in sorted(expected_inputs):
            if name == preparation["script"]:
                continue
            relative = Path(name)
            if relative.is_absolute() or ".." in relative.parts:
                raise SupportError("preparation input: unsafe path")
            destination = output / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            _copy(Path(__file__).with_name("inputs") / relative,
                  destination, 0o644, name)
            if _digest(destination, name) != expected_inputs[name]:
                raise SupportError(name + ": differs from the lock")
        for identity, record in helpers.items():
            destination = output / record["path"]
            _copy(sdk_payload / "bin" / identity, destination, 0o755, identity)
            if _digest(destination, identity) != record["sha256"]:
                raise SupportError(identity + ": helper changed during copying")
        archive_id = preparation["archive"]
        archive_record = lock["sources"][archive_id]
        metadata = {
            "schema": 1,
            "lock_sha256": lock_digest,
            "archive": {"source": archive_id, "bytes": archive_record["bytes"],
                        "sha256": archive_record["sha256"]},
            "base": inventory(sdk_payload),
            "helpers": helpers,
            "inputs": expected_inputs,
            "inventory": preparation["inventory"],
            "notices": sorted(name for name in preparation["inventory"]
                              if name.endswith("/README")),
        }
        (output / "preparation.json").write_text(
            json.dumps(metadata, sort_keys=True, indent=2) + "\n", encoding="utf-8")
        (output / "preparation.json").chmod(0o644)
    except BaseException:
        shutil.rmtree(output, ignore_errors=True)
        raise
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
