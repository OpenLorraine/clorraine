#!/usr/bin/env python3
"""Audit a packaged Clorraine VS Code extension.

The archive must contain the compiled adapter and client, the manifest, the
canonical grammar, the language configuration, and the Marketplace listing
files: the README, the changelog, and a PNG icon of at least 128 pixels. The
manifest must keep the listing properties the Marketplace page relies on. It
must not contain TypeScript sources, tests, scripts, dependencies, the
maintainer notes, or a development node_modules tree. When `--extract` is given, the archive is unpacked there for
the packaged-client smoke test.
"""

from __future__ import annotations

import argparse
import json
import struct
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
    "extension/CHANGELOG.md",
    "extension/images/icon.png",
    "extension/LICENSE.txt",
    "extension/LICENSE-EXCEPTION.md",
    "extension/NOTICE",
    "extension/TRADEMARKS.md",
)
FORBIDDEN_PREFIXES = ("extension/src/", "extension/test/", "extension/scripts/", "extension/node_modules/")
FORBIDDEN_SUFFIXES = (".ts", ".map", ".vsix")
FORBIDDEN_NAMES = ("extension/development.md",)
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
MINIMUM_ICON = 128
DRIVER_COMMAND = "clorraine.driverPath"


def read_entry(archive: zipfile.ZipFile, name: str) -> bytes:
    """Return the bytes of `name`, matched case-insensitively as the presence check is."""
    for entry in archive.namelist():
        if entry.lower() == name.lower():
            return archive.read(entry)
    raise KeyError(name)


def listing_problem(archive: zipfile.ZipFile) -> str | None:
    """Describe the first broken Marketplace listing property, or return None.

    The icon must be a PNG (the Marketplace rejects SVG icons) of at least
    128x128 pixels, and the manifest must name it and keep the preview flag,
    the workspace extension kind, limited Restricted Mode support, and the
    driver command hidden from the Command Palette.
    """
    icon = read_entry(archive, "extension/images/icon.png")
    if len(icon) < 24 or not icon.startswith(PNG_SIGNATURE) or icon[12:16] != b"IHDR":
        return "images/icon.png is not a PNG"
    width, height = struct.unpack(">II", icon[16:24])
    if width < MINIMUM_ICON or height < MINIMUM_ICON:
        return f"images/icon.png is {width}x{height}; the Marketplace needs at least {MINIMUM_ICON}x{MINIMUM_ICON}"
    try:
        manifest = json.loads(read_entry(archive, "extension/package.json"))
    except ValueError:
        return "package.json is not valid JSON"
    contributes = manifest.get("contributes", {})
    hidden = any(item.get("command") == DRIVER_COMMAND and item.get("when") == "false"
                 for item in contributes.get("menus", {}).get("commandPalette", []))
    checks = (
        (manifest.get("icon") == "images/icon.png", "package.json icon is not images/icon.png"),
        (manifest.get("preview") is True, "package.json preview is not true"),
        (manifest.get("extensionKind") == ["workspace"], 'package.json extensionKind is not ["workspace"]'),
        (manifest.get("capabilities", {}).get("untrustedWorkspaces", {}).get("supported") == "limited",
         "package.json untrustedWorkspaces.supported is not limited"),
        (any(item.get("command") == DRIVER_COMMAND for item in contributes.get("commands", [])),
         f"package.json does not contribute {DRIVER_COMMAND}"),
        (hidden, f"package.json does not hide {DRIVER_COMMAND} from the Command Palette"),
    )
    for passed, problem in checks:
        if not passed:
            return problem
    return None


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
            if (name.startswith(FORBIDDEN_PREFIXES) or name.endswith(FORBIDDEN_SUFFIXES)
                    or name.lower() in FORBIDDEN_NAMES):
                print(f"audit_vsix: forbidden entry: {name}", file=sys.stderr)
                return 1
            if name.startswith("/") or ".." in Path(name).parts:
                print(f"audit_vsix: unsafe entry: {name}", file=sys.stderr)
                return 1
        problem = listing_problem(archive)
        if problem:
            print(f"audit_vsix: {problem}", file=sys.stderr)
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
