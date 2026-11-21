#!/usr/bin/env python3
"""Thin maintainer entry for the proved support payload assembly API."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import sys

from package_support import assemble_payload, SupportError


KINDS = ("sdk", "cp310", "cp312", "preparation")
"""Payload kinds matching the immutable support archive names."""


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
    args = parser.parse_args(arguments)
    try:
        if args.kind == "preparation" and args.sdk_base is None:
            raise SupportError("preparation: --sdk-base is required")
        if args.kind != "preparation" and args.sdk_base is not None:
            raise SupportError("--sdk-base is only valid for preparation")
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
