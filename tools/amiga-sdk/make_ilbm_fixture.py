#!/usr/bin/env python3
"""Generate the committed 320x200x4 ILBM acceptance fixture.

The image is a deterministic geometric pattern with a 16-entry palette and a
ByteRun1 body. It exercises the pure parser's chunk walk, palette copy, RLE
decoder, and plane interleave. Run this script to reproduce
``examples/amiga/assets/clorraine.ilbm`` byte for byte.

Usage:
    python3 tools/amiga-sdk/make_ilbm_fixture.py [--output PATH]

The default output is the committed fixture. The script prints the file size
and the FNV-1a hash of the chunky pixel indices; the parser test pins the same
hash, so a decoder defect cannot hide behind a self-captured golden.
"""

from __future__ import annotations

import argparse
import struct
from pathlib import Path

WIDTH = 320
HEIGHT = 200
DEPTH = 4

# Pen 0 is the dark field and pen 1 is white, so the viewer caption can use
# those indices regardless of the image content.
PALETTE = [
    (0, 0, 80),
    (255, 255, 255),
    (255, 136, 0),
    (0, 255, 255),
    (0, 200, 0),
    (220, 0, 0),
    (255, 255, 0),
    (255, 0, 255),
    (200, 200, 200),
    (128, 128, 128),
    (64, 64, 64),
    (80, 160, 255),
    (0, 128, 96),
    (128, 0, 192),
    (160, 96, 32),
    (0, 0, 0),
]


def pixel_at(x: int, y: int) -> int:
    """Returns the palette index for one pixel."""
    # A three-pixel frame in cyan.
    if x < 3 or y < 3 or x >= WIDTH - 3 or y >= HEIGHT - 3:
        return 3
    # Horizontal colour bars across the top third.
    if y < 64:
        bars = [2, 5, 4, 6, 11, 13, 14, 7]
        return bars[(x // 20) % len(bars)]
    # A centred diamond with a white body and an orange outline.
    cx = WIDTH // 2
    cy = 128
    distance = abs(x - cx) + abs(y - cy)
    if distance < 34:
        return 1
    if distance < 40:
        return 2
    # A checkerboard block in the lower-left.
    if 24 <= x < 120 and 112 <= y < 184:
        return 8 if ((x // 8) + (y // 8)) % 2 == 0 else 10
    # A vertical gradient band on the right.
    if WIDTH - 96 <= x < WIDTH - 24 and 80 <= y < 184:
        return 4 + ((y - 80) // 13) % 12
    return 0


def pixels() -> list[int]:
    """Returns the row-major pixel indices."""
    return [pixel_at(x, y) for y in range(HEIGHT) for x in range(WIDTH)]


def plane_row(row: list[int], plane: int) -> bytes:
    """Packs one plane row of a chunky pixel row."""
    length = ((WIDTH + 15) // 16) * 2
    out = bytearray(length)
    for x, pixel in enumerate(row):
        if pixel & (1 << plane):
            out[x // 8] |= 0x80 >> (x % 8)
    return bytes(out)


def rle_row(row: bytes) -> bytes:
    """ByteRun1-encodes one row."""
    out = bytearray()
    at = 0
    while at < len(row):
        run = 1
        while at + run < len(row) and row[at + run] == row[at]:
            run += 1
        if run >= 3:
            out.append(257 - run)
            out.append(row[at])
            at += run
        else:
            start = at
            at += run
            while at < len(row) and at - start < 128:
                next_run = 1
                while at + next_run < len(row) and row[at + next_run] == row[at]:
                    next_run += 1
                if next_run >= 3 or at - start + next_run > 128:
                    break
                at += next_run
            count = at - start
            out.append(count - 1)
            out.extend(row[start:at])
    return bytes(out)


def chunk(chunk_id: bytes, payload: bytes) -> bytes:
    """Returns one IFF chunk with even padding."""
    out = bytearray(chunk_id)
    out.extend(struct.pack(">I", len(payload)))
    out.extend(payload)
    if len(payload) % 2 == 1:
        out.append(0)
    return bytes(out)


def build() -> tuple[bytes, list[int]]:
    """Builds the ILBM bytes and returns them with the pixel indices."""
    values = pixels()
    bmhd = struct.pack(
        ">HHHHBBBBHBBHH",
        WIDTH,
        HEIGHT,
        0,
        0,
        DEPTH,
        0,  # masking
        1,  # ByteRun1
        0,
        0,
        10,
        11,
        WIDTH,
        HEIGHT,
    )
    cmap = bytes(component for color in PALETTE for component in color)
    body = bytearray()
    for y in range(HEIGHT):
        row = values[y * WIDTH : (y + 1) * WIDTH]
        for plane in range(DEPTH):
            body.extend(rle_row(plane_row(row, plane)))
    chunks = chunk(b"BMHD", bmhd) + chunk(b"CMAP", cmap) + chunk(b"BODY", bytes(body))
    form = b"FORM" + struct.pack(">I", 4 + len(chunks)) + b"ILBM" + chunks
    return form, values


def fnv1a(data: list[int]) -> int:
    """FNV-1a 32-bit over the chunky pixels."""
    value = 2166136261
    for byte in data:
        value ^= byte
        value = (value * 16777619) & 0xFFFFFFFF
    return value


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("examples/amiga/assets/clorraine.ilbm"),
    )
    args = parser.parse_args()
    data, values = build()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(data)
    print(
        f"wrote {args.output} {len(data)} bytes "
        f"{len(values)} pixels hash {fnv1a(values)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
