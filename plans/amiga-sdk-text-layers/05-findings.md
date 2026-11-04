# Findings: Amiga SDK Bundle 4 — Text, Fonts, Layers

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Done

## Implementation findings

| # | Finding | Evidence | Disposition |
|---|---|---|---|
| F-1 | Layers must be created on a custom display bitmap; the layers library documents Intuition screens as unsupported. | The first layer stage on an Intuition screen showed a black box and the screen text did not render. The custom `LoadView` display path worked. | `newLayer(bitmap, ...)` takes a custom `Bitmap`, owns its `Layer_Info`, and the demo uses the Bundle 1 display. |
| F-2 | Writing `Layer_Info.bounds` after `NewLayerInfo()` corrupts the 1.3 structure and crashes `CreateUpfrontLayer`. | The A500 run produced a Software Error task-held dialog and only the pre-layer marks. Removing the bounds write made `newLayer` succeed. | `newLayer` leaves the `NewLayerInfo` defaults; the layer bounds argument alone defines the layer. |
| F-3 | A layer's `RastPort` uses layer-local coordinates; drawing with screen coordinates is clipped away. | `layerFill`/`layerText` at local (0,0)/(4,24) drew after removing the screen offset; with the offset the layer stayed blank. | `layerFill`, `layerClear`, and `layerText` pass layer-local coordinates; the layers library adds the origin and clips. |
| F-4 | The ROM `topaz.font` opens with `graphics.library/OpenFont` and renders on a layer. | The A500 line reports `width 128 height 8`, and the held frame shows `Clorraine layers` in the black scroll layer. | The demo uses `openFont` with a `defaultFont` fallback. |
| F-5 | A bare ADF has no `FONTS:` volume, so `openDiskFont` returns null. | The stage printed `disk false`; the demo's disk-font attempt is a null fallback. | `SDK-TXT-003` is deferred with the bare-ADF reason; the API is implemented and cross-builds. |
| F-6 | Creating a layer clears its rectangle to pen 0. | The display showed a black 160x64 box immediately after `newLayer` and before any draw. | The demo calls `layerClear`/`layerText` after creation; the title is white. |
| F-7 | The held frame and text line are deterministic. | The 50-second and 60-second held-frame crops compared at 0 differing pixels; the line crop matched the golden across the smoke. | The smoke compares the held display crop and the line byte-exact. |
| F-8 | `MoveLayer`, `SizeLayer`, and `DeleteLayer` return TRUE on success, not 0. | The first demo printed `text: checks 66`; the move/size calls had succeeded but the shim inverted the result, and `freeLayer` leaked the `Layer_Info`. After the fix the A500 line is `checks 510`. | The shim follows the documented BOOL convention; the demo gates the round-trip and the bounds rejections in the check bits. |

## Red phase

The spec and impl tests were written before the modules existed:

```text
error E11102: 'Font' is not exported from module 'std.amiga.text'
error E11102: 'Layer' is not exported from module 'std.amiga.layers'
error E11102: 'defaultFont' is not exported from module 'std.amiga.text'
...
test result: FAILED. 0 passed; 2 failed
```

## Green phase

| Gate | Result |
|---|---|
| `cargo test --test amiga_text_spec_test` | 6 passed (host/A500 compile, host rejection, `nix13`/`nix20` cross-builds, Exec still gated) |
| `cargo test --test amiga_text_impl_test` | 2 passed (wrapper routing, null-guarded drops) |
| `make sdk-a500-text` | OK; held layer frame and `text: ok width 128 height 8 layer 160x64 checks 510` matched |
| `make verify` | exit 0; hello, Boing, sprites, paddle, and text A500 gates passed |
| `make sdk-registry` | OK (79 capabilities, 10 bundles; Bundle 4 4/6 with 2 deferred) |

## Runtime evidence

| Evidence | Value |
|---|---|
| Held layer frame | `docs/amiga-sdk/a500-text.png` (408x438 crop) |
| Golden text line | `docs/amiga-sdk/a500-text-line.png` |
| Line | `text: ok width 128 height 8 layer 160x64 checks 510` |
| Font | ROM `topaz.font` 8 through `openFont`; 128x8 measured |
| Layers | Title 160x32 clipped; scroll 160x64 parked at x=0 |
| Machine | FS-UAE A500, real Kickstart 1.3 rev 34.5, decrypted licensed ROM |
| ROM SHA-1 | `891e9a547772fe0c6c19b610baf8bc4ea7fcb785` |

## Review ledger

The independent review found no critical and four major findings. All critical
and major findings were fixed and re-tested.

| ID | Severity | Finding | Fix |
|---|---|---|---|
| L-1 | Major | `newLayer` checked only the rectangle span, not its position inside the bitmap. | `newLayer` requires `0 <= x0 <= x1 < width` and `0 <= y0 <= y1 < height`; the spec/impl tests use a 240x180 bitmap. |
| L-2 | Major | `layerMove`/`layerSize` could move or grow a layer outside the bitmap. | The handle stores the bitmap dimensions; both calls validate the prospective bounds and return `false` outside. The demo exercises the round-trip and prints `checks 126`. |
| L-3 | Major | `drawText`/`layerText` left the RastPort font pointing at a freed font. | Both save and restore the previous RastPort font after the draw. |
| P-1 | Major | Bundle 4 was marked done before `make verify`; issue #5 was still open. | The final closeout runs `make verify`, then updates the statuses and closes #5. |
| T-1 | Minor | Tests used a 160x64 bitmap with a layer at (40,80)-(199,143), outside the bitmap; `BeginUpdate` ran after drawing. | Tests use 240x180 and call `BeginUpdate` before the draw. |
| L-4 | Minor | `layerBeginUpdate` discarded `BeginUpdate`; `EndUpdate` was called with FALSE. | `layerBeginUpdate` returns `bool`; `layerEndUpdate` uses `EndUpdate(... TRUE)`. |
| T-2 | Minor | The demo did not exercise diskfont, fill, move/size, or update; the impl test missed those calls. | The demo now calls `openDiskFont`, `layerFill`, move/size round-trip, and begin/end; the check line reports 126. `drawText` remains compile-checked in the committed demo (the development screen stage rendered it). |
| D-1 | Minor | Docs claimed a demo diskfont attempt that did not exist. | The demo calls `openDiskFont` and reports `checks 126` (disk bit 0); the ADF lacks `LIBS:diskfont.library` and `FONTS:`. |
| D-2 | Minor | Docs said a missing layer exits 1 and claimed a move without one; the clear/region wording was imprecise. | Exit code corrected to 4; the move/size round-trip is real and reported; the creation clear is described as observed; the 2.0+ region API is distinguished from the 1.3 update calls. |
| L-5 | Minor | Each layer has its own `Layer_Info`, so layers do not stack/occlude and simple refresh leaves stale pixels. | Documented in the module and both appendices; the demo repaints the background after the move/size round-trip. |

Accepted residuals:

- `GfxBase`, `LayersBase`, and `DiskfontBase` are opened once and never
  closed; this matches the existing process-lifetime bases.
- Bitmap/font lifetimes are not enforced by the type system; the layer module
  and appendices document the free order and the returned-handle hazard.
- If `DeleteLayer` fails, the shim frees the handle and leaks the `Layer_Info`
  rather than disposing it under a live layer.
- `drawText` is compile-checked in the committed demo; the earlier development
  stage rendered text on an Intuition screen, and the layer text path renders
  on the A500.
- The A500 golden is a 408x438 crop of the layer display, not the full
  320x256 frame.
