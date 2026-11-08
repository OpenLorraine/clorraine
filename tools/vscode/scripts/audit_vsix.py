#!/usr/bin/env python3
"""Audit a packaged Clorraine VS Code extension.

The archive must contain the compiled adapter and client, the manifest, the
canonical grammar, the language configuration, and the README. It must not
contain TypeScript sources, tests, scripts, dependencies, or a development
node_modules tree. When `--extract` is given, the archive is unpacked there for
the packaged-client smoke test.
"""

from __future__ import annotations

import argparse
import sys
import zipfile
from pathlib import Path

REQUIRED = (
    "extension/package.json",
    "extension/out/extension.js",
    "extension/out/client.js",
    "extension/syntaxes/clorraine.tmLanguage.json",
    "extension/language-configuration.json",
    "extension/README.md",
    "extension/LICENSE.txt",
)
FORBIDDEN_PREFIXES = ("extension/src/", "extension/test/", "extension/scripts/", "extension/node_modules/")
FORBIDDEN_SUFFIXES = (".ts", ".map", ".vsix")


def audit(vsix: Path, extract: Path | None) -> int:
    if not vsix.is_file():
        print(f"audit_vsix: missing archive: {vsix}", file=sys.stderr)
        return 2
    with zipfile.ZipFile(vsix) as archive:
        names = archive.namelist()
        lowered = {name.lower() for name in names}
        missing = [name for name in REQUIRED if name.lower() not in lowered]
        if missing:
            print(f"audit_vsix: missing entries: {', '.join(missing)}", file=sys.stderr)
            return 1
        for name in names:
            if name.startswith(FORBIDDEN_PREFIXES) or name.endswith(FORBIDDEN_SUFFIXES):
                print(f"audit_vsix: forbidden entry: {name}", file=sys.stderr)
                return 1
            if name.startswith("/") or ".." in Path(name).parts:
                print(f"audit_vsix: unsafe entry: {name}", file=sys.stderr)
                return 1
        if extract is not None:
            extract = extract.resolve()
            if extract.exists():
                import shutil

                shutil.rmtree(extract)
            extract.mkdir(parents=True)
            archive.extractall(extract)
    print(f"audit_vsix: ok ({len(names)} entries)")
    return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--vsix", type=Path, required=True)
    parser.add_argument("--extract", type=Path)
    arguments = parser.parse_args(argv)
    return audit(arguments.vsix, arguments.extract)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
