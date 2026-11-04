# Design: Layers, Scrolling Demo, and A500 Smoke

> **Document**: 03-02-layers-demo.md
> **Parent**: [Index](00-index.md)
> **Covers**: R3–R6, R10; AR-5–AR-9

## Layer module

`std.amiga.layers` is available on both hosted profiles and imports `Bitmap`
and `Font`.

```text
export handle Layer freedBy cl_layer_free;

export func newLayer(borrow bitmap: Bitmap, x0: i32, y0: i32,
                     x1: i32, y1: i32): Layer?
export func freeLayer(take layer: Layer): void
export func layerMove(borrow layer: Layer, dx: i32, dy: i32): bool
export func layerSize(borrow layer: Layer, dw: i32, dh: i32): bool
export func layerFill(borrow layer: Layer, x1: i32, y1: i32,
                      x2: i32, y2: i32, pen: u8): void
export func layerClear(borrow layer: Layer, pen: u8): void
export func layerText(borrow layer: Layer, borrow font: Font,
                      x: i32, y: i32, pen: u8, const text: u8[]): void
export func layerBeginUpdate(borrow layer: Layer): void
export func layerEndUpdate(borrow layer: Layer): void
```

`newLayer` opens `layers.library` version 33, calls `NewLayerInfo`, and calls
`CreateUpfrontLayer(info, bitmap, x0, y0, x1, y1, LAYERSIMPLE, NULL)`. The
rectangle must lie inside the bitmap: `0 <= x0 <= x1 < width` and
`0 <= y0 <= y1 < height`. The `Layer_Info` bounds are left at the
`NewLayerInfo` defaults: writing `info->bounds` on 1.3 corrupts the library
structure and crashes `CreateUpfrontLayer`. The layer owns its `Layer_Info`;
it borrows the bitmap. Free the layer before the bitmap.

Each layer has its own `Layer_Info`, so separate layers do not stack, occlude,
or restore each other. A simple-refresh layer leaves stale pixels when moved or
resized; the demo repaints the background after its move/size round-trip.

Layers must be created on a **custom display bitmap**, not an Intuition screen;
the layers library documents Intuition screens as unsupported. The layer's
`RastPort` uses **layer-local** coordinates (0,0 is the layer's upper-left);
the layers library adds the layer origin and clips to the layer bounds. The
shim therefore passes coordinates through unchanged.

Drawing uses the layer's `RastPort`:

| Call | Implementation |
|---|---|
| `layerClear` | `SetRast(rp, pen)` |
| `layerFill` | `SetAPen` + `RectFill` |
| `layerText` | `SetFont` + `SetAPen` + `Move` + `Text` |
| `layerMove` / `layerSize` | `MoveLayer` / `SizeLayer` |
| `layerBeginUpdate` / `layerEndUpdate` | `BeginUpdate` (returns bool) / `EndUpdate(... TRUE)` |

`freeLayer` calls `DeleteLayer(0, layer)` then `DisposeLayerInfo(info)`. The
caller's handle is nulled by the generated `take` call, so the cleanup ladder
does not double-free.

## Scrolling demo

| Element | Value |
|---|---|
| Display | 320x256x4 custom `LoadView` display |
| Font | ROM `topaz.font` 8 through `openFont`, fallback `defaultFont()` |
| Measured text | `Clorraine layers` = 128x8 |
| Title layer | 160x32 at (40,24); a 22-character title wider than 160, clipped at the right edge |
| Scroll layer | 160x64 at (40,96); the measured text scrolls and parks at x=0 |
| Background | dark blue field, white top bar, orange side bars |
| Frames | 160 frames, one DOS tick each |
| Hold | 1200 ticks (24 s) |
| Final line | `text: ok width 128 height 8 layer 160x64 checks 510` |
| Check bits | 510 = begin/fill/text/end (2), move (4), size (8), move-back (16), size-back (32), redraw (64), out-of-bounds move rejected (128), out-of-bounds layer rejected (256); disk font is 0 |

The two layers are created on the **visible front bitmap** after the first
`displayFlip`. The scroller does not flip again: it draws into the displayed
front bitmap, so the layers update directly. The title layer is black text on
white and is clipped by the 160-pixel layer; the scroll layer is white text on
black and is cleared then redrawn each frame. The title layer is also moved by
(4,2), shrunk by (4,2), moved back, and grown back; the shim rejects any move or
resize that would leave the bitmap, and the demo reports the successful
round-trip in the check line.

## A500 smoke

`docker/scripts/a500-text-smoke.sh` mirrors the Boing smoke:

1. Build a clean `boot1x` ADF with `scroller` and `DF0:scroller`.
2. Decrypt the licensed KS1.3 ROM and verify the A500 SHA-1.
3. Boot FS-UAE as an A500.
4. Poll the held display crop `408x438+418+102` against
   `docs/amiga-sdk/a500-text.png`.
5. Poll the `430x22+160+88` line crop against
   `docs/amiga-sdk/a500-text-line.png`.
6. Stop the emulator and remove the decrypted ROM.

The held frame shows the clipped title and the parked scroll text. The line is
the measured font and layer size plus the check bits (`checks 510`).

## Errors

- A missing font falls back to the default; a missing layer prints
  `text: layer failed` and exits 4.
- The scroll and title layers are freed before the display, font, and bitmaps.
- The smoke treats a missing ROM/tool/window as a hard failure.
