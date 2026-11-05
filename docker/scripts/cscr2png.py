#!/usr/bin/env python3
"""Convert a Clorraine native screen capture (`CSCR`) to a PNG.

The capture format is written by `saveScreen` in `std.amiga.intuition`. It
stores the Amiga screen bitmap instead of expanded pixels, so the file is small
enough to write to a floppy while the UI keeps running. The layout is:

    0   "CSCR"
    4   width           big-endian u16
    6   height          big-endian u16
    8   depth           big-endian u16
    10  bytes per row   big-endian u16
    12  palette         2**depth RGB triples, 8 bits per component
    ..  bitplanes       depth * bytes per row * height bytes

The PNG is written with the Python standard library only, so the smoke tests do
not need Pillow. `--scale` enlarges the image with nearest-neighbour sampling,
which keeps the pixel-art look readable when the 640x256 screen is shown in a
report.
"""

from __future__ import annotations

import argparse
import struct
import sys
import zlib
from pathlib import Path

MAGIC = b"CSCR"
HEADER_SIZE = 12


def read_capture(path: Path) -> tuple[int, int, int, int, list[tuple[int, int, int]], list[bytes]]:
    """Read and validate a CSCR file, returning its geometry and image data."""
    data = path.read_bytes()
    if len(data) < HEADER_SIZE or data[:4] != MAGIC:
        raise ValueError(f"{path}: not a CSCR capture")
    width, height, depth, bytes_per_row = struct.unpack(">HHHH", data[4:HEADER_SIZE])
    if width == 0 or height == 0:
        raise ValueError(f"{path}: empty image")
    if depth < 1 or depth > 8:
        raise ValueError(f"{path}: unsupported depth {depth}")
    colors = 1 << depth
    palette_end = HEADER_SIZE + colors * 3
    plane_size = bytes_per_row * height
    expected = palette_end + depth * plane_size
    if len(data) != expected:
        raise ValueError(
            f"{path}: expected {expected} bytes for {width}x{height}x{depth}, "
            f"found {len(data)}"
        )
    palette = [
        tuple(data[HEADER_SIZE + index * 3 : HEADER_SIZE + index * 3 + 3])
        for index in range(colors)
    ]
    planes = []
    for plane in range(depth):
        start = palette_end + plane * plane_size
        planes.append(data[start : start + plane_size])
    return width, height, depth, bytes_per_row, palette, planes


def to_rgb_rows(
    width: int,
    height: int,
    depth: int,
    bytes_per_row: int,
    palette: list[tuple[int, int, int]],
    planes: list[bytes],
    scale: int,
) -> list[bytes]:
    """Expand the bitplanes into PNG-ready RGB rows, optionally enlarged."""
    rows = []
    for y in range(height):
        row = bytearray()
        base = y * bytes_per_row
        for x in range(width):
            byte_index = base + (x >> 3)
            bit = 7 - (x & 7)
            index = 0
            for plane_index in range(depth):
                index |= ((planes[plane_index][byte_index] >> bit) & 1) << plane_index
            pixel = bytes(palette[index])
            row += pixel * scale
        rows.extend([bytes(row)] * scale)
    return rows


def write_png(path: Path, width: int, height: int, rows: list[bytes]) -> None:
    """Write an 8-bit RGB PNG using only the standard library."""

    def chunk(tag: bytes, payload: bytes) -> bytes:
        crc = zlib.crc32(tag + payload) & 0xFFFFFFFF
        return struct.pack(">I", len(payload)) + tag + payload + struct.pack(">I", crc)

    raw = b"".join(b"\x00" + row for row in rows)
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 9))
    png += chunk(b"IEND", b"")
    path.write_bytes(png)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path, help="input .cscr file")
    parser.add_argument("png", type=Path, help="output .png file")
    parser.add_argument(
        "--scale",
        type=int,
        default=1,
        help="nearest-neighbour enlargement factor (default: 1)",
    )
    args = parser.parse_args(argv)
    if args.scale < 1:
        parser.error("--scale must be at least 1")

    width, height, depth, bytes_per_row, palette, planes = read_capture(args.capture)
    rows = to_rgb_rows(
        width, height, depth, bytes_per_row, palette, planes, args.scale
    )
    write_png(args.png, width * args.scale, height * args.scale, rows)
    print(f"{args.png}: {width * args.scale}x{height * args.scale} from {args.capture}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
