"""Check that a Windows release executable is the expected build.

`package.sh` reads each staged or zipped `.exe` with this module: it must be a
PE32+ image for x86-64 and carry `clorraine <version>`, the text its
`--version` prints, not followed by further version digits.
"""

from __future__ import annotations

import re
import struct
import sys


def problem(data: bytes, version: str) -> str | None:
    """Returns why `data` is not the release executable, or None."""
    if len(data) < 0x40 or data[:2] != b"MZ":
        return "is not a PE32+ x86-64 executable"
    offset = struct.unpack_from("<I", data, 0x3C)[0]
    if offset + 26 > len(data) or data[offset:offset + 4] != b"PE\0\0":
        return "is not a PE32+ x86-64 executable"
    machine = struct.unpack_from("<H", data, offset + 4)[0]
    magic = struct.unpack_from("<H", data, offset + 24)[0]
    if (machine, magic) != (0x8664, 0x020B):
        return "is not a PE32+ x86-64 executable"
    # String literals sit back to back in the image, so any byte may follow
    # the version; only a further version digit (`1.2.0.1`, `1.2.01`) is
    # refused. A pre-release suffix cannot be told apart from the next string.
    pattern = rb"clorraine " + re.escape(version.encode()) + rb"(?![0-9]|\.[0-9])"
    if re.search(pattern, data) is None:
        return f"does not carry the version {version}"
    return None


def main() -> int:
    """Checks standard input; argv is the shown name and the version."""
    name, version = sys.argv[1], sys.argv[2]
    found = problem(sys.stdin.buffer.read(), version)
    if found:
        print(f"package: {name} {found}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
