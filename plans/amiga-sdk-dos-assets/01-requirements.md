# Requirements: Amiga SDK Bundle 5 — DOS and Assets

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Covers**: R1–R13

## Scope

Bundle 5 adds the DOS file/asset surface and an ILBM image viewer for the
`hosted-amiga` and `hosted-amiga-13` profiles. It does not change the language,
grammar, diagnostics, or the ABI.

### In scope

- `std.amiga.dos`: seek/position/length on an open `File`, path locks,
  path information, directory create/delete, file delete/rename, current and
  parent directory names, and volume information.
- `std.iff`: a pure IFF/ILBM header parser, palette reader, and pixel decoder.
- `std.amiga.ilbm`: load an ILBM file into a `Bitmap` and a caller palette.
- `std.amiga.video`: a primitive that copies chunky pixel indices into a bitmap's
  planes.
- `examples/amiga/imageviewer.clo`, a committed ILBM fixture, an A500 smoke,
  docs, registry, and tracking updates.

### Out of scope

- `ReadArgs` / CLI argument parsing (`SDK-DOS-004`, 2.0+).
- `datatypes.library` (`SDK-DOS-006`, 3.0+).
- HAM, EHB, deep ILBM, masking, multi-`BODY` files, and animation.
- Windows, requesters, and Workbench integration (Bundles 6 and 8).

## Requirements

- **R1 — Seek and length.** `seekFile(file, offset, whence)` moves an open file
  and reports success; `filePosition(file)` and `fileLength(file)` report the
  current position and total size. The three calls must not disturb the file's
  content and must leave a failed seek's position unchanged as far as the libc
  primitive allows.
- **R2 — Locks.** `lockPath(path)` returns an owned shared lock or `null`;
  `unlockPath` releases it exactly once.
- **R3 — Path information.** `pathInfo(path)` returns an owned opaque handle or
  `null`; `pathInfoIsDir`, `pathInfoSize`, and `pathInfoName` read the captured
  `FileInfoBlock` after the lock has been released.
- **R4 — Directory and file mutation.** `makeDir`, `deleteFile`, and
  `renameFile` return `true` on success. `deleteFile` works on an empty
  directory as well as a file. `setCurrentDir` changes the process directory and
  returns `true`; `currentDirName` and `parentDirName` copy the name into a
  caller buffer and return the byte count.
- **R5 — Volume information.** `volumeInfo(path)` returns an owned opaque handle
  or `null`; `volumeName`, `volumeFreeBytes`, and `volumeUsedBytes` read the
  captured `InfoData` and volume name.
- **R6 — IFF structure.** `ilbmInfo(data)` validates `FORM`/`ILBM`, walks the
  chunks with even padding, and reports `ok` only when a usable `BMHD` and
  `BODY` are present. It records width, height, depth, compression, color count,
  and body/CMAP locations.
- **R7 — ILBM decoding.** `ilbmDecode(data, info, pixels)` decodes uncompressed
  and ByteRun1-compressed, row-interleaved planes into one byte per pixel. It
  rejects short output buffers, truncated bodies, bad RLE counts, and invalid
  geometry with `false`; it never traps or writes out of bounds.
- **R8 — Palette.** `ilbmPalette(data, info, palette)` copies RGB triplets from
  `CMAP` into the caller buffer and returns the number of colors copied.
- **R9 — Loader.** `loadIlbm(path, palette)` reads the whole file, parses and
  decodes it, creates a `Bitmap` of the image size and depth, copies the pixels
  into its planes, fills the palette, and returns an owned `IlbmImage` with an
  `ok` gate, the bitmap, actual color count, pixel hash, and geometry. Any
  failure returns an image with `ok = false` and a null bitmap, and releases
  every temporary resource. The loader rejects a file over 128 KB and an image
  wider than 320, taller than 256, or deeper than 5 planes before allocating.
- **R10 — Viewer.** `examples/amiga/imageviewer.clo` proves the DOS surface on
  the target: it creates a directory and file, writes and seeks, reads back,
  renames, inspects, deletes, removes the directory, and reads `DF0:` volume
  information. It then loads `DF0:image.ilbm`, applies the palette, opens a
  320x200x4 custom display, draws a caption layer with the ROM Topaz font, holds
  a deterministic frame, and prints
  `img: ok 320x200x4 hash <N> dos 255 vol 1`.
- **R11 — Fixture.** The committed fixture is a valid 320x200x4 ILBM with 16
  `CMAP` colors and a ByteRun1 `BODY`, at most 128 KB compressed. A generator
  reproduces it byte-exactly and an implementation test runs that generator.
- **R12 — Deferrals.** `SDK-DOS-004` and `SDK-DOS-006` are `deferred` with the
  minimum-OS reason; no 2.0+/3.0+ symbol is called by the 1.3 build.
- **R13 — Profile availability.** `std.iff` is a pure module on the host and
  both hosted-Amiga profiles (the freestanding profile keeps only `std.io`);
  `std.amiga.dos` and `std.amiga.ilbm` are hosted-Amiga only, and the 1.3
  whitelist includes both new modules.

## Constraints

- Target: 68000, OCS, 512 KB chip, Kickstart 1.3, `-mcrt=nix13`,
  `-m68000 -fwrapv`, strict C99 with `-Wall -Wextra -Werror`.
- Worst-case chip use is bounded at under 301 KB: 32 KB front, 32 KB back,
  82 KB pixel indices, 128 KB file buffer, and under 24 KB for display, layer,
  font, palette, and CLI buffers.
- The display API accepts widths up to 320, heights up to 256, and depths 1..5.
- The demo must fit in chip RAM: a 320x200x4 front bitmap, a matching back
  bitmap, the file buffer, one byte per decoded pixel, and the display
  structures.
- No decrypted ROM and no generated build output is committed.
- ADF boot and file reads are slow; smokes poll from 30 s in 5 s steps.

## Acceptance

1. `std.iff`, `std.amiga.dos`, and `std.amiga.ilbm` compile on both hosted
   profiles; `std.iff` runs on the host; the host rejects the two Amiga-only
   modules with `E11200` (R13).
2. The host-run parser cases cover uncompressed, ByteRun1, odd width, palette,
   and malformed inputs.
3. The full surface cross-builds with `nix13` and `nix20` under `-Werror`.
4. The A500 run sets every DOS check bit and every volume bit, loads the
   fixture, and prints a stable line and held frame.
5. `make check`, `make verify`, and `make sdk-registry` pass; earlier A500 gates
   stay green.
6. Registry Bundle 5 is 5/7 done (two deferred), issue #6 is closed, and the
   roadmap/programme show Bundle 5 done.
