# Design: ILBM Loader, Fixture, Viewer, and A500 Smoke

> **Document**: 03-03-ilbm-loader-viewer.md
> **Parent**: [Index](00-index.md)
> **Covers**: R9–R12; AR-11–AR-14

## Loader

`std.amiga.ilbm` is a hosted-Amiga module that composes `std.iff`,
`std.amiga.dos`, `std.io`, `std.mem`, and `std.amiga.video`.

```text
export struct IlbmImage {
    ok: bool;
    bitmap: Bitmap?;
    colors: u16;
    hash: u32;
    width: i32;
    height: i32;
    depth: u8;
}

export func loadIlbm(path: cstring, palette: u8[]): IlbmImage
```

`ok` is the gate; when it is false the bitmap is null. The struct owns the
bitmap when `ok` is true; its scalar fields describe the decoded image. `hash`
is `ilbmPixelHash` over the chunky pixels before conversion, and `colors` is the
number of whole palette entries copied. A caller checks `ok` and `bitmap`, then
borrows `image.bitmap` for display and lets the struct drop the bitmap after the
display is freed.

The loader:

1. opens `path` with `MODE_OLDFILE`;
2. reads `fileLength`; rejects `<= 0` or `> 131072` bytes, so the file buffer
   cannot exceed the 512 KB chip budget;
3. allocates one `Buffer` of that size and loops `readFile` until full;
4. calls `ilbmInfo`; a failed parse returns an image with `ok = false`;
5. rejects images wider than 320 or taller than 256 (the display budget);
6. allocates one byte per pixel and calls `ilbmDecode`;
7. calls `newBitmap(width, height, depth)`;
8. copies the indices with `bitmapFromIndices`;
9. copies the `CMAP` into the caller palette and computes the pixel hash;
10. returns the image; every temporary `File`/`Buffer` drops on the return path.

The caller owns the returned image and its palette buffer.

## Pixel copy primitive

`std.amiga.video` gains:

```text
export func bitmapFromIndices(borrow bitmap: Bitmap, const indices: u8[]): void
```

The shim clears the destination planes and sets bit `plane` for every pixel
whose index has that bit set. A short index view leaves the bitmap cleared
rather than reading past the view.

`std.iff` is registered for both hosted profiles and the host; `std.amiga.ilbm`
is hosted-Amiga only. The 1.3 whitelist adds `std.iff` and `std.amiga.ilbm`
(`std.amiga.dos` is already present). The export tables in
`compiler/src/stdlib.rs` add the new `std.iff` and `std.amiga.ilbm` names and
the new `std.amiga.dos`/`std.amiga.video` wrappers.

## Fixture

`examples/amiga/assets/clorraine.ilbm` is a committed 320x200x4 ILBM:

| Chunk | Content |
|---|---|
| `BMHD` | 320x200, 4 planes, masking 0, compression 1 (ByteRun1) |
| `CMAP` | 16 RGB triplets: pen 0 dark blue, pen 1 white, then orange, cyan, green, and grays |
| `BODY` | row-interleaved ByteRun1 planes with long runs |

The image is a deterministic geometric pattern: a dark field, horizontal color
bars, a centered diamond, and a checkerboard corner.
`tools/amiga-sdk/make_ilbm_fixture.py` regenerates the exact bytes.
`compiler/tests/amiga_assets_impl_test.rs` runs the generator into a scratch
directory and compares the bytes with the committed file; the smoke uses the
committed file, so it does not depend on Pillow or ImageMagick.

## Viewer

`examples/amiga/imageviewer.clo` runs this sequence:

1. **DOS self-test.** Create `DF0:tmp`, lock it, and set it current; check
   `currentDirName` is `tmp` and `parentDirName` is non-empty; restore `DF0:`.
   Create/write `DF0:tmp/probe` with four bytes, reopen it, check
   `fileLength == 4`, seek to offset 1, check `filePosition == 1`, and read back
   `L`. Check `pathInfo` (not a directory, size 4, name `probe`), rename to
   `probe2`, check the new name, delete the file, and delete the directory.
   Check `DF0:` volume name, free bytes, and used bytes.
2. **Load.** `loadIlbm("DF0:image.ilbm", palette)` with a 48-byte palette
   buffer.
3. **Display.** Create a matching 320x200x4 back bitmap, open the display over
   `image.bitmap` and the back bitmap, apply all 16 palette entries with
   `displaySetColor`, and flip once.
4. **Caption.** Open the ROM `topaz.font` (fallback `defaultFont`), create a
   200x16 layer at (8,8), fill it with pen 15 (black), draw
   `Clorraine ILBM viewer` in pen 1 (white) with `layerText`, and draw directly
   without the damage update protocol.
5. **Hold.** Hold 1200 ticks (24 s) for the A500 capture.
6. **Report.** Free the layer, display, font, and back bitmap, then print:

```text
img: ok 320x200x4 hash <N> dos 255 vol 1
```

`dos` is an eight-bit mask: file round-trip (1), path info (2), rename (4),
delete file (8), directory create/delete (16), volume info (32), lock/unlock
(64), and current/parent directory (128). `vol` is 1 when the volume name is
non-empty and both free and used bytes are positive. `hash` is the
`ilbmPixelHash` value of the decoded pixels.

Failure exits are distinct: 1 for the bitmap/back allocation, 2 for the display,
3 for the image load, 4 for the caption layer, 5 for the font. The demo prints a
specific `image:` failure line before returning.

## Chip-RAM budget

| Item | Bytes |
|---|---|
| Front bitmap (image) | 32,000 |
| Back bitmap | 32,000 |
| Pixel indices (320x256 max) | 81,920 |
| ILBM file buffer (128 KB cap) | 131,072 |
| Display, layer, font, palette, CLI | < 24,000 |
| **Total** | **< 301,000 of 524,288** |

The fixture is much smaller than the cap; the budget proves the worst case the
loader accepts still fits.

## A500 smoke

`docker/scripts/a500-assets-smoke.sh` follows the text smoke:

1. Build the demo with `--profile hosted-amiga-13`.
2. Create a clean `boot1x` ADF with `imageviewer` and `image.ilbm`; the startup
   sequence runs `DF0:imageviewer`.
3. Decrypt the licensed KS1.3 ROM and verify the A500 SHA-1.
4. Boot FS-UAE as an A500 and poll from 30 s in 5 s steps.
5. When the held display matches `docs/amiga-sdk/a500-assets.png`, take a
   second capture and require zero differing pixels.
6. Compare the 620x22 line crop with `docs/amiga-sdk/a500-assets-line.png`.
7. Stop the emulator and remove the decrypted ROM.

The goldens are captured from the same committed fixture and demo, so the line
and frame are deterministic. The target build uses `--release`: the debug image
is about 280 KB, which left the ADF at 98% and did not reach the display within
the poll budget; the release image is about 74 KB and reaches the display in
about a minute. The smoke treats a missing ROM, tool, window,
fixture, or golden as a hard failure.

## Errors

- A missing or malformed fixture prints `image: load failed` and exits 3.
- A missing display exits 2; a missing font exits 5; a missing layer exits 4.
- The demo never frees the display before the layer or the image bitmap before
  the display.
- The smoke removes the decrypted ROM on every exit path.
