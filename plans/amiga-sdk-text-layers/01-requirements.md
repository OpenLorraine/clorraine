# Requirements: Amiga SDK Bundle 4 — Text, Fonts, Layers

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Functional Requirements

### Must Have

- **R1 — Fonts.** `openFont(name, size, style)` opens a 1.3 `TextFont` with
  `graphics.library/OpenFont`; `openDiskFont` uses
  `diskfont.library/OpenDiskFont`; `defaultFont` wraps
  `GfxBase->DefaultFont`. `freeFont` closes only owned fonts.
- **R2 — Text.** `textWidth(font, text)` measures with `TextLength`;
  `textHeight(font)` returns `tf_YSize`; `drawText(screen, font, x, y, pen,
  text)` sets the font and pen on the screen's `RastPort` and draws.
- **R3 — Layers.** `newLayer(bitmap, x0, y0, x1, y1)` creates a `LAYERSIMPLE`
  front layer over a caller-owned custom display bitmap. The layer owns its own
  `Layer_Info` and borrows the bitmap; `freeLayer` deletes the layer and
  disposes the info. The rectangle must lie inside the bitmap. The layer must
  be freed before the bitmap. Layers are not created on Intuition screens.
- **R4 — Layer drawing and clipping.** `layerFill`, `layerClear`, and
  `layerText` draw through the layer's `RastPort` using layer-local coordinates
  (0,0 is the layer's upper-left), so the layers library adds the layer origin
  and clips to the layer bounds. `layerMove`/`layerSize` move and resize the
  layer and reject bounds outside the bitmap; `layerBeginUpdate` returns
  whether the caller may draw and `layerEndUpdate` completes the update.
- **R5 — Demo.** `examples/amiga/scroller.clo` opens a 320x256 custom display,
  opens the ROM `topaz.font` with `openFont` (falling back to `defaultFont`),
  measures `Clorraine layers`, creates a clipped 160x32 title layer and a
  160x64 scroll layer, scrolls the text through the scroll layer, holds a
  deterministic frame, exercises a title-layer move/size round-trip, the
  disk-font attempt, and the bitmap-bounds rejections, and prints
  `text: ok width 128 height 8 layer 160x64 checks 510`.
- **R6 — Runtime proof.** `docker/scripts/a500-text-smoke.sh` boots the demo on
  the real KS1.3 ROM and compares the held screen and the text line to
  committed goldens.
- **R7 — Compatibility.** The two modules are available on `hosted-amiga` and
  `hosted-amiga-13`, rejected by host, and leave the earlier A500 gates green.
- **R8 — Documentation.** Update the appendices, `AGENTS.md`, registry,
  coverage, issue #5, and the programme/roadmap.

### Should Have

- **R9 — Diskfont.** `openDiskFont` is implemented and attempted; the bare ADF
  has no `FONTS:` volume, so the attempt returns `null` and the row is deferred
  with that reason. The ROM `openFont` font is the visible path.
- **R10 — Deterministic measurement.** The fixed string's width and height are
  stable across runs and are printed in the final line.

### Won't Have (Out of Scope)

- 2.0+ regions/damage (`SDK-TXT-005`) and 2.0+ font outlines.
- Text rendering into the custom `Display` bitmaps; the demo uses the Intuition
  screen and a layer.
- Unicode, proportional font shaping, or a font asset pipeline.
- Any language, grammar, diagnostic, or ABI change.

## Technical Requirements

- The font handle owns `OpenFont` results and closes them with `CloseFont`; the
  default font is borrowed and never closed.
- The layer's bitmap must be at least the layer's pixel bounds.
- The layer's `RastPort` uses layer-local coordinates; the layers-library clip,
  not a manual coordinate test, bounds every draw.
- `newLayer` leaves `Layer_Info.bounds` at the `NewLayerInfo` defaults; writing
  it on 1.3 corrupts the structure.
- The demo uses `timer.device` pacing from Bundle 3.

## Acceptance Criteria

1. The modules compile on both hosted profiles, cross-build with
   `nix13`/`nix20`, and are rejected by host.
2. The spec tests cover font open/default/close, width/height, text draw,
   layer create/free/move/fill/text.
3. The A500 demo shows the clipped scrolling text and prints the deterministic
   line.
4. `make check`, `make verify`, `make sdk-registry`, and the text smoke pass.
5. The four earlier A500 gates stay green.
6. Registry Bundle 4 is 4/6 done; diskfont and regions are deferred with
   evidence.
