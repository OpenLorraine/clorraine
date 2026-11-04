# Design: Display, Copper, Checksum, and Boing

> **Document**: 03-02-display-copper-boing.md
> **Parent**: [Index](00-index.md)
> **Covers**: R5–R7, R9–R11, R13; AR-4, AR-5, AR-7, AR-9, AR-10, AR-15

## Display lifecycle

`Display` is a pair of complete `View`s, one per caller-owned bitmap. The two
views share a `ColorMap` but have separate `ViewPort`, `RasInfo`, and
`UCopList` instructions. The first flip builds both copper lists; later flips
only switch the active view.

```text
openDisplay(front, back)
    -> record bitmaps, GetColorMap, default palette
    -> no hardware access yet

setColor(...)
    -> SetRGB4CM on the shared ColorMap (before the first flip)

setCopper(copper)
    -> attach one UCopList to each ViewPort (before the first flip)

first flip
    -> InitView/InitVPort for both views
    -> MakeVPort + MrgCop for both views
    -> oldview = GfxBase->ActiView
    -> LoadView(view 0); WaitTOF
    -> front = 0

later flip
    -> LoadView(view 1 - front); WaitTOF
    -> front = 1 - front
```

The two-view technique is the 1.3 double-buffer method documented by the
graphics library: each view has its own compiled copper list, so a flip is a
single `LoadView` and no list rebuild. The display never modifies the caller's
bitmaps' geometry.

## Restore contract

`freeDisplay` restores the system view that was active at the first flip:

```text
LoadView(oldview)
WaitTOF
WaitTOF
FreeCprList(view[0].LOFCprList)
FreeCprList(view[1].LOFCprList)
FreeVPortCopLists(&view_port[0])
FreeVPortCopLists(&view_port[1])
FreeColorMap(color_map)
```

The two `WaitTOF` calls let the long and short frame copper lists finish before
the lists and bitmap planes are freed. The display does not free the bitmaps;
the caller drops them after the display.

## Copper list

The copper handle owns two `UCopList` structures. A builder call appends the
same instruction to both:

```text
copperWait(line)          -> CWait(list, line, 0)
copperMove(reg, value)    -> CMove(list, reg, value)
copperColor(i, r, g, b)   -> CMove(list, COLOR00 + i*2, 12-bit color)
copperEnd()               -> CWait(list, 10000, 255)
```

The 1.3 `UCopperListInit(list, 256)` allocates the intermediate instruction
buffer. The display stores the two lists until the first flip; the flip calls
`InitVPort` (which clears `UCopIns`), attaches the queued list, and only then
calls `MakeVPort` so the list is merged. The display takes ownership of the two
lists; `FreeVPortCopLists` releases them.

The Boing gradient waits every 16 lines and writes the three background color
registers, producing horizontal bands without touching the bitmap or the CPU.

## Blitter budget

Boing uses a 64x64 ball with a 64x64 single-plane mask and a static 320x256
background. Each frame:

1. For each of the two frame buffers:
   - restore the previous 64x64 ball rectangle from the master background
     (`BltBitMap`, 64x64x4);
   - blit the ball through the mask at the new position
     (`BltMaskBitMapRastPort`, 64x64x4 + 64x64x1 mask).
2. Flip the display and wait for VBlank.

That is four small blits per frame (about 2 x (16K + 16K) bit transfers), not a
40 KB full-frame copy. The frame count is printed so the run can be checked.

## Boing scene

| Element | Value |
|---|---|
| Display | 320x256x4, lowres PAL (Bundle 1 caps displays at 320x256x5) |
| Background | 16-pixel checkerboard in pens 1–3, drawn once into the master and both buffers |
| Ball | 64x64x4 shaded sphere in pens 4–15, with a 64x64x1 circular mask |
| Motion | 8.8 fixed point, velocity ±2.25 px/frame in x and ±1.75 in y, wall bounce |
| Copper | Color 1–3 gradient every 16 lines |
| Frames | 400 (eight seconds at 50 Hz) |
| Acceptance frame | the ball is parked at (128, 96) in both buffers and the front buffer is held for 10 s |
| Final line | `boing: checksum <8 hex digits>` |

The ball position is deterministic: integer fixed-point, no randomness, no
timing-dependent input. The checksum of the final visible buffer is therefore
stable across runs and can be compared as a text line.

## Frame checksum

`bitmapChecksum` folds the geometry and then every plane byte:

```text
h = 0x811C9DC5
h = (h XOR depth) * 0x01000193
h = (h XOR width)  * 0x01000193
h = (h XOR height) * 0x01000193
h = (h XOR bytes_per_row) * 0x01000193
h = (h XOR rows) * 0x01000193
for plane in 0 .. depth:
    for byte in plane[0 .. bytes_per_row * height]:
        h = (h XOR byte) * 0x01000193
```

The result is printed as eight hexadecimal digits. The demo prints the
checksum of the buffer that is visible at the end of the run, after the last
flip.

## A500 smoke

`docker/scripts/a500-boing-smoke.sh` mirrors the hello smoke:

1. Require the built `boing` binary.
2. Decrypt `amiga-os-130.rom` with `amitools` and `rom.key`; verify SHA-1
   `891e9a547772fe0c6c19b610baf8bc4ea7fcb785`.
3. Build a clean `boot1x` ADF with `boing` and a root startup-sequence
   `DF0:boing`.
4. Boot FS-UAE as an A500 and wait 30 seconds for the ~20-second boot plus the
   demo.
5. Poll the screen until the display is static (the held acceptance frame) and
   compare the display crop to `docs/amiga-sdk/a500-boing-frame.png`; save the
   live frame as evidence.
6. Continue polling until the shell returns and compare the `boing: checksum`
   crop to `docs/amiga-sdk/a500-boing-line.png`.
7. Stop the emulator and remove the decrypted ROM.

The held display crop is the visual gate: it covers the copper gradient,
palette, background, and ball. The checksum crop is the frame-content gate: it
covers the bitmap bytes independently of the display hardware.

## Error behavior

- A bitmap allocation failure frees the partial planes and returns `null`.
- A display open failure frees the color map and returns `null`.
- A copper attached after the first flip is freed and the call returns `false`.
- The demo maps each failure to a distinct `boing: ... failed` line and exits 1.
- The smoke treats a missing ROM, missing tool, or missing window as a hard
  failure with the offending path in the message.
