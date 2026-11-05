# Design: Pure IFF/ILBM Parser

> **Document**: 03-01-iff-parser.md
> **Parent**: [Index](00-index.md)
> **Covers**: R6–R8; AR-2–AR-6

## Why a pure module

`std.iff` imports only `std.mem` and language intrinsics. It never imports
`std.amiga.*`, so the host profile runs the same parser the A500 runs. The
specification tests decode real ILBM bytes and assert pixel values on the
development host, not just cross-build the code.

## Surface

```text
export struct IlbmInfo {
    ok: bool;
    width: i32;
    height: i32;
    depth: u8;
    compression: u8;
    colors: u16;
    cmap_start: usize;
    body_start: usize;
    body_len: usize;
}

export func ilbmInfo(data: u8[]): IlbmInfo
export func ilbmDecode(data: u8[], const info: IlbmInfo, pixels: u8[]): bool
export func ilbmPalette(data: u8[], const info: IlbmInfo, palette: u8[]): u16
export func ilbmPixelHash(pixels: u8[]): u32
```

`IlbmInfo` is a native struct with only scalar fields; it is returned by value
and copies freely. A failed parse returns `ok = false` and zero geometry.
`ilbmPixelHash` is FNV-1a over the decoded bytes, so the host tests and the A500
line can pin the same value.

## Header walk

1. Require at least 12 bytes; bytes 0..4 are `FORM`, bytes 8..12 are `ILBM`.
2. Read the big-endian FORM length. Require `length >= 4` and
   `length <= data.len - 8` (subtraction, never `8 + length`); the chunk window
   ends at `8 + length`.
3. Walk chunks from offset 12 while `end - offset >= 8`:
   - read the 4-byte id and the big-endian 4-byte size;
   - require `size <= end - offset - 8` (subtraction);
   - `BMHD` (size >= 20) records width, height, depth, masking, and compression;
   - `CMAP` records the color count (`size / 3`, capped at 256) and the payload
     offset;
   - `BODY` records the first payload offset and length; reject a second `BODY`
     and a zero-length body;
   - advance by `8 + size` (safe because `size <= end - offset - 8`), then one
     pad byte when `size` is odd; require the result not to pass `end`.
4. `ok` requires a `BMHD`, a `BODY`, `1 <= width <= 1024`,
   `1 <= height <= 1024`, `1 <= depth <= 8`, `masking == 0`, and
   `compression <= 1`.

`CAMG`, `ABHD`, `CRNG`, and other chunks are skipped.

## Decoding

`ilbmDecode` writes one chunky index per pixel into `pixels`.

```text
require info.ok
require 1 <= width <= 1024, 1 <= height <= 1024, 1 <= depth <= 8
require compression <= 1
require body_start <= data.len and body_len <= data.len - body_start
body_end = body_start + body_len
require pixels.len >= width * height
row_bytes = ((width + 15) / 16) * 2
require row_bytes <= 128

clear pixels[0 .. width * height]
cursor = body_start
for y in 0..height:
    for plane in 0..depth:
        decode one row of row_bytes bytes into row[]
        for x in 0..width:
            bit = (row[x >> 3] >> (7 - (x & 7))) & 1
            if bit != 0:
                pixels[y * width + x] |= u8(1 << plane)
```

Uncompressed rows are copied. ByteRun1 rows decode with the standard control
byte:

| Control | Meaning |
|---|---|
| `0..127` | copy the next `control + 1` literal bytes |
| `-1..-127` | repeat the next byte `-control + 1` times |
| `-128` | no-op |

Every cursor read and every row write is bounds-checked before it happens. A
malformed stream returns `false`; the function never slices past `data.len` and
never writes past `pixels.len`. `row` is a fixed `u8[128]`, which covers the
maximum 1024-pixel width at 128 bytes per row; wider headers fail the parse.

The destination is cleared first, so a caller may pass recycled memory. Tests
start from `0xFF` and assert that every pixel was overwritten.

## Palette

`ilbmPalette` copies whole RGB triplets only:
`min(colors, palette.len / 3) * 3` bytes starting at `cmap_start`. It returns
the number of whole colors copied. A missing `CMAP` returns 0 and leaves the
caller's buffer untouched.

## Errors

| Condition | Result |
|---|---|
| Short FORM, wrong type, truncated chunk, wrapped length/size | `info.ok == false` |
| Missing `BMHD`/`BODY`, zero/multiple BODY, bad depth/geometry/masking/compression | `info.ok == false` |
| Short pixel buffer, truncated body, bad RLE count, `info.ok == false` | `ilbmDecode == false` |
| No `CMAP` | `ilbmPalette == 0` |

The parser allocates nothing; all output goes to caller buffers.
