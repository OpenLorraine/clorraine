# Design: Native Video API

> **Document**: 03-01-native-video-api.md
> **Parent**: [Index](00-index.md)
> **Covers**: R1–R9, R12; AR-2, AR-3, AR-6, AR-9, AR-12

## Module surface

Bundle 1 adds two embedded modules:

| Module | Profiles | Purpose |
|---|---|---|
| `std.amiga.video` | `hosted-amiga`, `hosted-amiga-13` | Chip memory, bitmaps, drawing, blitter, copper, display |
| `std.math` | `host`, `hosted-amiga`, `hosted-amiga-13` | Fixed-point `sin`, `cos`, `sqrt` |

`std.amiga.video` is registered in `compiler/src/stdlib.rs` and added to the
1.3 whitelist in `Profile::supports_module`. `std.math` is registered and is
accepted by the host automatically because its name is not under `std.amiga.`.

## Opaque handles

```text
export handle ChipBuffer freedBy cl_video_chip_free;
export handle Bitmap     freedBy cl_video_bitmap_free;
export handle Copper     freedBy cl_video_copper_free;
export handle Display    freedBy cl_video_display_free;
```

The handles are C pointers to private shim structs. No `struct BitMap`,
`ViewPort`, `UCopList`, or other OS struct appears in a public signature. The
shim owns the OS layout; the application sees only the handle.

## Chip memory

```text
allocChip(size: u32): ChipBuffer?
freeChip(take buffer: ChipBuffer): void
chipSize(const buffer: ChipBuffer): u32
chipRead8(const buffer: ChipBuffer, offset: u32): u8
chipWrite8(borrow buffer: ChipBuffer, offset: u32, value: u8): void
chipRead16 / chipWrite16 / chipRead32 / chipWrite32
```

`allocChip` allocates the payload with `AllocMem(size, MEMF_CHIP | MEMF_CLEAR)`
and a small fast-memory record that stores the payload pointer and size. A
failed payload allocation frees the record and returns `null`. The read/write
helpers return 0 / ignore the write when `offset` is outside the payload, so a
bad offset cannot read or write outside the allocation. The 16/32-bit helpers
require the offset to be aligned and the full width to fit; otherwise they
return 0 / ignore the write.

## Bitmaps

```text
newBitmap(width: i32, height: i32, depth: u8): Bitmap?
freeBitmap(take bitmap: Bitmap): void
bitmapWidth / bitmapHeight / bitmapDepth / bitmapBytesPerRow
bitmapClear(borrow bitmap: Bitmap, pen: u8): void
bitmapPlot(borrow bitmap: Bitmap, x, y: i32, pen: u8): void
bitmapLine(borrow bitmap: Bitmap, x1, y1, x2, y2: i32, pen: u8): void
bitmapFill(borrow bitmap: Bitmap, x1, y1, x2, y2: i32, pen: u8): void
bitmapPlaneBit(const bitmap: Bitmap, plane: u8, x, y: i32): bool
bitmapSetPlaneBit(borrow bitmap: Bitmap, plane: u8, x, y: i32, on: bool): void
bitmapBlit(borrow dest: Bitmap, const src: Bitmap, sx, sy, dx, dy, w, h: i32): void
bitmapBlitMasked(borrow dest: Bitmap, const src: Bitmap, const mask: Bitmap,
                 sx, sy, dx, dy, w, h: i32): void
bitmapChecksum(const bitmap: Bitmap): u32
```

`newBitmap` calls `InitBitMap` and `AllocRaster` for each plane. Width must be
1..=1024 and height 1..=1024; depth must be 1..=8 (OCS displays use 1..=5, but
the allocator does not enforce a chipset limit). A failed plane allocation frees
the planes already allocated and the record.

`depth` is a byte and the plane bits above the depth are never written. A pen
value with bits above the depth is masked. Coordinates are clipped to the
bitmap before any plane access.

`bitmapChecksum` uses FNV-1a 32-bit:

```text
hash = 0x811C9DC5
for each byte in plane 0 .. depth-1, row 0 .. height-1:
    hash = (hash XOR byte) * 0x01000193
```

The depth, width, and height are folded in first, so two bitmaps with the same
pixel bytes but different geometry do not share a checksum. The multiplication
is `u32` wrapping (`-fwrapv` is already on the 1.3 toolchain).

## Blitter

`bitmapBlit` calls `BltBitMap(src, sx, sy, dest, dx, dy, w, h, 0xC0, 0xFF,
NULL)`. `bitmapBlitMasked` builds a temporary `RastPort` for `dest` with
`InitRastPort`/`RastPort.BitMap = dest->bm`, then calls
`BltMaskBitMapRastPort(src, sx, sy, &rp, dx, dy, w, h, 0xE0,
mask->planes[0])`. The mask bitmap must have the same width and height as the
source and depth exactly 1; the shim returns without a blit if it does not. The
OCS blitter limit is 992 pixels wide and 1024 rows; a larger request is
ignored. Both calls end with `WaitBlit()`, so the source and destination planes
are safe to free or inspect when the call returns.

## Copper

```text
newCopper(): Copper?
freeCopper(take copper: Copper): void
copperWait(borrow copper: Copper, line: i32): void
copperMove(borrow copper: Copper, reg: u16, value: u16): void
copperColor(borrow copper: Copper, index: u8, r: u8, g: u8, b: u8): void
copperEnd(borrow copper: Copper): void
```

The handle owns two `UCopList` structures, one for each display viewport. Every
builder call appends the same instruction to both lists, so a double-buffered
display keeps the same effect on both frames. `copperColor(index, r, g, b)`
appends a move to `COLOR00 + index*2` with the 12-bit value
`(r >> 4) << 8 | (g >> 4) << 4 | (b >> 4)`.

`copperWait` uses `CWait(list, line, 0)`. `copperEnd` uses `CWait(list, 10000,
255)`, the documented `CEND` value (wait forever). The lists are allocated with
`AllocMem(sizeof(struct UCopList), MEMF_PUBLIC | MEMF_CLEAR)` and initialized
with `UCopperListInit(list, 256)`; a list that exceeds 256 instructions is
truncated and the extra calls are ignored.

`freeCopper` is only for a copper that was never given to a display. It frees
each `UCopList` through a temporary `ViewPort` and `FreeVPortCopLists`, then the
record. A copper passed to `displaySetCopper` is owned by the display and must
not be freed by the caller; the language's `take` mode enforces that.

## Display

```text
openDisplay(borrow front: Bitmap, borrow back: Bitmap): Display?
freeDisplay(take display: Display): void
displaySetColor(borrow display: Display, index: u8, r: u8, g: u8, b: u8): void
displaySetCopper(borrow display: Display, take copper: Copper): bool
displayFlip(borrow display: Display): void
displayFailed(const display: Display): bool
displayIndex(const display: Display): u8
waitVBlank(): void
```

`openDisplay` checks that both bitmaps have the same width, height, and depth.
Bundle 1 is a low-resolution PAL view: width 1..=320, height 1..=256, depth
1..=5. It allocates the `ColorMap` with `GetColorMap(1 << depth)` and
initializes it to a default palette (black, white, red, green, blue, yellow,
cyan, magenta, then gray steps). It stores the borrowed bitmap pointers but
does not call `MakeVPort`/`MrgCop`/`LoadView` yet.

`displaySetColor` writes the 4-bit components into the `ColorMap` with
`SetRGB4CM`. It is valid before the first flip; after the first flip the baked
copper list keeps the old colors, so later calls only update the map for a
future display. The demo sets every color before its first flip.

`displaySetCopper` queues the copper's two lists for the first flip and returns
`true`. It is valid only before the first flip; if the display has already been
built it frees the copper and returns `false`. The first flip calls `InitVPort`
first (which clears `UCopIns`), then attaches the queued list, then calls
`MakeVPort`.

The first `displayFlip`:

1. `InitView` and `InitVPort` for both views.
2. Set `DWidth`, `DHeight`, `DxOffset = 0x81`, `DyOffset = 0x2c`,
   `RasInfo.BitMap`, `ColorMap`, and `UCopIns` (when a copper is attached).
3. Attach the queued user copper list, then run `MakeVPort` and `MrgCop` for
   both views.
4. Save `GfxBase->ActiView` and `LoadView(&views[0])`.
5. `WaitTOF` and set `front = 0`.

Later flips call `LoadView(&views[1 - front])`, `WaitTOF`, and update `front`.
The caller draws into `1 - displayIndex(display)`. If a view cannot be built,
the display marks itself failed and later flips are no-ops; `displayFailed`
reports that state so the caller can fail loudly.

`freeDisplay`:

1. If built: `LoadView(oldview)`, `WaitTOF` twice.
2. `FreeCprList` on both `LOFCprList`s.
3. `FreeVPortCopLists` on both viewports (this also frees an attached user
   copper list).
4. `FreeColorMap`, then free the record.

The display never closes `graphics.library`; the shim uses one process-wide
`GfxBase` and keeps it open for the life of the program, matching the existing
screen helpers.

## `std.math`

```text
sin(angle: i32): i32
cos(angle: i32): i32
sqrt(value: i32): i32
```

Angles are 256 steps per turn. The sine result is scaled by 256; `cos(a)` is
`sin(a + 64)`. A 65-entry quarter-wave table is stored in the function; the
other quadrants use symmetry. `sqrt` uses the integer Newton iteration and
returns 0 for negative input.

## Diagnostics and failure behavior

- A null constructor result is the only failure signal; the caller narrows it
  before use (the existing nullable rule, `E10010`).
- Invalid bitmap geometry, a depth of 0, a mismatched display pair, or a
  mask/source mismatch is a no-op or a `null` return; it never traps.
- The host profile rejects `std.amiga.video` with the existing `E11200` module
  gate; `std.math` is host-available.
