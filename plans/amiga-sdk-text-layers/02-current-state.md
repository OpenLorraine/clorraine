# Current State: Amiga SDK Bundle 4 — Text, Fonts, Layers

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing implementation

Bundles 0–3 provide the A500/1.3 profile, `std.amiga.video`, `std.math`,
`std.amiga.sprite`, `std.amiga.audio`, `std.amiga.input`, `std.amiga.timer`,
the Intuition screen and drawing surface, and four real-ROM A500 gates. The
shim includes graphics, intuition, diskfont/layers headers are not yet used.

## Gaps

| Gap | Required change |
|---|---|
| No text module | Add `stdlib/std.amiga.text.clo` with `Font`, `openFont`, `openDiskFont`, `defaultFont`, `drawText`, `textWidth`, `textHeight` |
| No layers module | Add `stdlib/std.amiga.layers.clo` with `Layer`, create/free/move/fill/text |
| No registration | Add both to `SOURCES`/`MODULES` and the 1.3 whitelist |
| No font/layer shim | `OpenFont`/`OpenDiskFont`/`CloseFont`, `TextLength`/`Text`, `CreateUpfrontLayer`/`DeleteLayer` |
| No demo/smoke | `scroller.clo` and the A500 text smoke |

## Risks

| Risk | Mitigation |
|---|---|
| `topaz.font` is not on the bare ADF | `openFont` from graphics.library can use the ROM font; fall back to `GfxBase->DefaultFont` |
| diskfont needs a `FONTS:` disk | Attempt `OpenDiskFont`; fall back and defer the row with evidence |
| Layer bitmap/lifetime order | The scene owns the screen and layer; free layer, bitmap, then screen |
| Text clipping not visible | Use a small 160x64 layer with text wider than the layer |
| Timing | Reuse `std.amiga.timer` from Bundle 3 |
