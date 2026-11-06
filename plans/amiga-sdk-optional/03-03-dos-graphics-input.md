# DOS, Graphics, and Input: Bundle 9

> **Document**: 03-03-dos-graphics-input.md
> **Parent**: [Index](00-index.md)

## Overview

This component covers `SDK-DOS-004` ReadArgs, `SDK-DOS-006` datatypes (3.1 V40 object API),
`SDK-TXT-005` regions, `SDK-AUD-008` EClock, and `SDK-AUD-006` lowlevel `ReadJoyPort`. The modules
are `hosted-amiga` only. `std.amiga.text.openDiskFont` already exists; its runtime gate is in
[03-06](03-06-demos-gates-media.md). [AR-8, AR-17–AR-21, AR-30, AR-39]

## Implementation Details

### `std.amiga.args` — ReadArgs

| Export | Signature | Ownership / behavior |
|---|---|---|
| `Args` | `handle freedBy cl_free_args` | Owns `RDArgs`, slot array, and parsed slot kinds |
| `readArgs` | `(template: cstring, slots: i32): Args?` | Command-line `ReadArgs`; slots must equal template count |
| `readArgsFrom` | `(template: cstring, slots: i32, const text: u8[]): Args?` | Deterministic buffer source; appends `\n` for V37 |
| `freeArgs` | `(take args): void` | `FreeArgs`, free `DOS_RDARGS`, free slots/kinds |
| `argCount` | `(borrow args): i32` | Parsed template count |
| `argPresent` | `(borrow args, index: i32): bool` | Raw slot non-zero |
| `argNumber` | `(borrow args, index: i32): i32` | `/N` only; dereferences the stored `LONG *` |
| `argText` | `(borrow args, index: i32, borrow buf: u8[]): i32` | `/A`/`/K` only; bounded NUL-terminated copy |
| `argSwitch` | `(borrow args, index: i32): bool` | `/S` only |

`readArgs` parses the template, counts top-level options, rejects `/M`, `/T`, `/F`, and requires
`slots == count`. Each slot stores a kind (`A/K`, `N`, `S`) so `argText`/`argNumber`/`argSwitch`
never reinterpret the wrong type. `readArgsFrom` uses `AllocDosObject(DOS_RDARGS)`,
`RDA_Source.CS_Buffer`, `CS_Length`, and `RDAF_NOPROMPT`. [AR-17]

### `std.amiga.datatypes` — picture objects (3.1)

| Export | Signature | Ownership / behavior |
|---|---|---|
| `DTObject` | `handle freedBy cl_dispose_dt_object` | Owns `Object *` |
| `openPicture` | `(path: cstring): DTObject?` | `NewDTObjectA(path, DTA_GroupID=GID_PICTURE, TAG_DONE)` |
| `disposeDTObject` | `(take object): void` | `DisposeDTObject` |
| `dtWidth` | `(borrow object): i32` | `PDTA_BitMapHeader.bmh_Width` |
| `dtHeight` | `(borrow object): i32` | `bmh_Height` |
| `dtDepth` | `(borrow object): i32` | `bmh_Depth` |
| `dtColors` | `(borrow object): i32` | `PDTA_NumColors` |
| `drawDTObject` | `(borrow object, borrow window: Window, x: i32, y: i32): bool` | `struct dtDraw` + `DoDTMethodA` |

The shim uses `PDTA_BitMapHeader`/`PDTA_NumColors`, not the deprecated `DTA_*` width/depth tags.
`drawDTObject` builds `struct dtDraw { MethodID=DTM_DRAW, dtd_RPort=window->RPort, dtd_Left,
dtd_Top, dtd_Width, dtd_Height }` and calls `DoDTMethodA`. The fixture is the committed
320x200x4 ILBM. [AR-18, AR-39]

### `std.amiga.regions` — regions and layer clipping (2.0+)

| Export | Signature | Ownership / behavior |
|---|---|---|
| `Region` | `handle freedBy cl_free_region` | Owns `struct Region *` |
| `newRegion` | `(): Region?` | `NewRegion` |
| `freeRegion` | `(take region): void` | `DisposeRegion` |
| `regionOrRect` | `(borrow region, x1: i32, y1: i32, x2: i32, y2: i32): bool` | `OrRectRegion` |
| `regionAndRect` | `(borrow region, x1: i32, y1: i32, x2: i32, y2: i32): void` | `AndRectRegion` (void) |
| `regionClearRect` | `(borrow region, x1, y1, x2, y2): bool` | `ClearRectRegion` |
| `regionBounds` | `(borrow region): RegionBounds` | Fills from `region->bounds` |
| `regionIsEmpty` | `(borrow region): bool` | `MaxX < MinX || MaxY < MinY` |
| `layerSetClipRegion` | `(borrow layer: Layer, borrow region: Region): void` | `InstallClipRegion`; layer borrows |
| `layerClearClipRegion` | `(borrow layer: Layer): void` | `InstallClipRegion(layer, NULL)` |

`RegionBounds` is `{ x1: i32, y1: i32, x2: i32, y2: i32 }`. The layer borrows the region; the caller
must clear the clip before freeing the region or closing the layer. `layerClearClipRegion` does not
free the old region (the caller owns it). The existing `layerBeginUpdate`/`layerEndUpdate` remain
the damage protocol. [AR-19]

### `std.amiga.eclock` — EClock (2.0+)

| Export | Signature | Ownership / behavior |
|---|---|---|
| `EClock` | `handle freedBy cl_free_eclock` | Owns a `timer.device` `UNIT_ECLOCK` request |
| `openEClock` | `(): EClock?` | `OpenDevice("timer.device", UNIT_ECLOCK, ...)` |
| `readEClock` | `(borrow clock): EClockTicks` | Calls `ReadEClock`; returns `{ hi: u32, lo: u32 }` |
| `eclockFrequency` | `(borrow clock): u32` | The `ReadEClock` return (709379 PAL / 715909 NTSC) |
| `freeEClock` | `(take clock): void` | Close/free |

`EClockTicks` is a value struct with two `u32` fields; v1 has no `u64`. The shim stores the last
`EClockVal` and fills the struct from two accessors. [AR-20]

### `std.amiga.lowlevel` — ReadJoyPort (3.1)

| Export | Signature | Ownership / behavior |
|---|---|---|
| `readJoyPort` | `(port: u8): u32` | Lazy `OpenLibrary("lowlevel.library", 40)`; `ReadJoyPort(port)` |
| `decodeJoyPort` | `(raw: u32): u8` | Type-aware mapping to `JOY_*` |
| `readJoyPortDecoded` | `(port: u8): u8` | `decodeJoyPort(readJoyPort(port))` |

`decodeJoyPort` checks `raw & JP_TYPE_MASK`: directions only for `JP_TYPE_GAMECTLR`/`JP_TYPE_JOYSTK`;
`JPF_BUTTON_RED` maps to `JOY_FIRE` for available controllers; mouse/not-available return 0. The
gate prints the type nibble for both ports and does not assert a direction. [AR-21]

## Error Handling

- `readArgsFrom` returns null for a malformed template, count mismatch, unsupported form, or OOM.
- `argText` returns -1 for a non-string slot, absent value, or empty buffer.
- `openPicture` returns null when the file/class is unavailable; accessors return 0.
- `newRegion` returns null on OOM; `regionAndRect` is void; `regionBounds` returns empty bounds.
- `openEClock` returns null; `readEClock` returns zero ticks for a null/closed clock.
- `readJoyPort` returns 0 when the library is absent; `decodeJoyPort` is total.

## Tests

- Compile all five surfaces on `hosted-amiga`; reject host/1.3/freestanding with `E11200`.
- Cross-build `nix20`; no `nix13` for these modules.
- Pin `ReadArgs`/`FreeArgs`/`AllocDosObject`/`FreeDosObject`, `NewDTObjectA`/`GetDTAttrsA`/
  `DoDTMethodA`/`DisposeDTObject`, `NewRegion`/`OrRectRegion`/`AndRectRegion`/`ClearRectRegion`/
  `DisposeRegion`/`InstallClipRegion`, `ReadEClock`, `ReadJoyPort`.
- Runtime: A600/KS2.05 for ReadArgs/EClock/regions; A1200/KS3.1 for datatypes; A600/KS3.1 for
  lowlevel.
- Gate lines: `args: ok n 3 file 1 num 7 sw 1`; `regions: ok bounds 10 10 20 20 clip 1`;
  `eclock: ok 709379`; `dt: ok w 320 h 200 depth 4 colors 16`; `ll: ok v 40 p0 0 p1 0`.
