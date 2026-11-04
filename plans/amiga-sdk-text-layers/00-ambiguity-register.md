# Ambiguity Register: Amiga SDK Bundle 4 — Text, Fonts, Layers

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)
> **Status**: ✅ GATE PASSED
> **Mode**: auto-design (eligible technical decisions delegated and recorded)

## Scope baseline

Bundle 4 (`4-text-layers`) from `docs/amiga-sdk/registry.tsv`: font
open/close, text draw/measure, diskfont.library (should), layers and clipping
(must), regions and damage (should, 2.0+), and the scrolling text/layers demo
(must). No language, grammar, diagnostic, or ABI change.

## Register

| # | Category | Decision / question | Resolution | Owner | Status |
|---|---|---|---|---|---|
| AR-1 | Scope | Which capabilities belong to Bundle 4? | Font, text draw/measure, diskfont, layers/clipping, and the demo. `SDK-TXT-005` regions/damage is 2.0+ advanced and is deferred. | User (registry), AI — delegated | ✅ Resolved |
| AR-2 | Module split | One module or two? | `std.amiga.text` for fonts/text and `std.amiga.layers` for layers; each owns a distinct resource and keeps the surface small. | AI — delegated (auto-design) | ✅ Resolved |
| AR-3 | Font source | diskfont, graphics `OpenFont`, or the ROM default? | `openFont` uses the 1.3 `graphics.library/OpenFont`; the ROM `topaz.font` opens and measures 128x8. `openDiskFont` uses `diskfont.library/OpenDiskFont`. `defaultFont` wraps `GfxBase->DefaultFont` as a non-owned fallback. | AI — delegated (auto-design), verified by A500 run | ✅ Resolved |
| AR-4 | Diskfont row | Can a bare ADF prove diskfont? | The ADF has no `FONTS:` volume; the A500 `openDiskFont` attempt returns `null`. The API is implemented and cross-builds, and the row is deferred with that evidence. | AI — delegated (auto-design) | ✅ Resolved |
| AR-5 | Text target | Screen or layer? | Both: `drawText` draws to the Intuition screen's RastPort; `layerText` draws to a layer's RastPort, which the layers library clips. | AI — delegated (auto-design) | ✅ Resolved |
| AR-6 | Layer ownership | Who owns the bitmap and layer info? | The layer owns its own `Layer_Info` and borrows the caller's `Bitmap`; the layer must be freed before the bitmap. Layers are built on a custom display bitmap, not an Intuition screen. The demo frees the layers, then the display/font/bitmaps. | AI — delegated (auto-design), verified by A500 run | ✅ Resolved |
| AR-7 | Layer bounds | What size is the layer bitmap, and what coordinates does drawing use? | The bitmap must be at least the layer's pixel bounds. The layer `RastPort` uses layer-local coordinates (0,0 is the layer's upper-left); the layers library adds the origin and clips. `newLayer` does not write `Layer_Info.bounds`; doing so crashed 1.3 `CreateUpfrontLayer`. | AI — delegated (auto-design), verified by A500 run | ✅ Resolved |
| AR-8 | Demo | What does the scrolling demo show? | A 320x256 custom `LoadView` display, a 160x32 title layer whose text is wider than the layer (clipped), a 160x64 scroll layer, a deterministic parked frame, and no second flip (the layers draw into the visible front bitmap). | AI — delegated (auto-design) | ✅ Resolved |
| AR-9 | Acceptance | How is the demo proven on real 1.3? | The demo opens the ROM font, measures 128x8, creates the title/scroll layers, scrolls the clipped text, holds a fixed frame, and prints `text: ok width 128 height 8 layer 160x64`. The A500 smoke compares the 408x438 held display crop and the line. | User (programme acceptance), AI — delegated | ✅ Resolved |
| AR-10 | Compatibility | Does the 3.1 profile change? | The modules are available on both hosted profiles; no existing API changes. The 1.3 whitelist adds both modules. Regions stay deferred. | User (Bundle 0 rule), AI — delegated | ✅ Resolved |
| AR-11 | Errors | What happens on font/layer failure? | Constructors return `T?`; the demo falls back to the default font and prints a distinct failure if the layer cannot open. | AI — delegated (auto-design) | ✅ Resolved |
| AR-12 | Documentation | Which docs move with the code? | `spec/appendix-amiga.md`, `spec/appendix-stdlib.md`, `AGENTS.md`, registry/coverage, issue #5, programme/roadmap, plan. | User (Bundle 0 rule), AI — delegated | ✅ Resolved |
| AR-13 | Language | Any language/grammar/diagnostic/ABI change? | None. | User (programme rule) | ✅ Resolved |
| AR-14 | Testing | Where do the tests live? | `compiler/tests/amiga_text_spec_test.rs`, `amiga_text_impl_test.rs`, and `docker/scripts/a500-text-smoke.sh`. | AI — delegated (auto-design) | ✅ Resolved |

## Open items

None. Every row is resolved; regions/damage is explicitly deferred, and the
diskfont row is resolved by the A500 attempt/fallback.
