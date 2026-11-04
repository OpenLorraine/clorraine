# Ambiguity Register: Amiga SDK Bundle 2 — Sprites and Collision

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)
> **Status**: ✅ GATE PASSED
> **Mode**: auto-design (eligible technical decisions delegated and recorded)

## Scope baseline

Bundle 2 (`2-sprites`) from `docs/amiga-sdk/registry.tsv`: eight hardware
sprite channels, sprite pixels/move/show/hide, attached pairs for 16 colors,
hardware collision detection, animation objects, and the sprite demo. No
language, grammar, diagnostic, or ABI change.

## Register

| # | Category | Decision / question | Resolution | Owner | Status |
|---|---|---|---|---|---|
| AR-1 | Scope | Which capabilities belong to Bundle 2? | The six `2-sprites` registry rows, and nothing else. | User (registry), AI — delegated | ✅ Resolved |
| AR-2 | Architecture | Extend `std.amiga.video` or add a module? | Add `std.amiga.sprite`. Sprites are global hardware channels independent of the display; the module can be used with or without the Bundle 1 display. | AI — delegated (auto-design) | ✅ Resolved |
| AR-3 | Hardware access | `graphics.library` sprites or raw custom registers? | The 1.3 simple-sprite machine (`GetSprite`, `ChangeSprite`, `MoveSprite`, `FreeSprite`) attached to an Intuition screen viewport. Raw sprite registers were tried first and were overwritten by the display's compiled list; the library path is the documented 1.3 route. | AI — delegated (auto-design), verified by A500 run | ✅ Resolved |
| AR-4 | Pixel format | What does the caller provide? | `u8[]` with 16 pixels per line, values 0..3; the shim packs the two bitplanes. An attached pair uses two calls, one for the low pair and one for the high pair, and `spriteAttach` sets the odd channel's ATTACH bit. | AI — delegated (auto-design) | ✅ Resolved |
| AR-5 | Position | How are coordinates represented? | `x` 0..511, `y` 0..255, 16-pixel-wide sprite. `MoveSprite` positions the sprite relative to the screen viewport. | AI — delegated (auto-design) | ✅ Resolved |
| AR-6 | Show/hide | How is a sprite hidden? | `ChangeSprite` with a full-size zero image and `RethinkDisplay`; show installs the real image and rebuilds. | AI — delegated (auto-design) | ✅ Resolved |
| AR-7 | Collision | How is collision exposed? | `enableCollisions` writes `CLXCON = 0xF000` (all sprite pairs, odd channels included); `collisions()` reads and clears `CLXDAT`. The demo parks pair 2/3 and single 4 together, masks bits 9..14, and reports bit 12 (`0x1000`). | AI — delegated (auto-design) | ✅ Resolved |
| AR-8 | Animation | What is an animation object? | A `SpriteAnimation` owns one sprite channel and a fixed set of chip-memory frame buffers; `setFrame` packs pixels, `showFrame`/`next` switch the pointer. | AI — delegated (auto-design) | ✅ Resolved |
| AR-9 | Acceptance | How is the sprite demo proven on real 1.3? | `examples/amiga/sprite-demo.clo` runs over an Intuition screen, shows a 16-color attached pair on 2/3, a single sprite on 4, and an animation on 6, forces a known overlap, reads the hardware collision bit, holds a deterministic frame, and prints `sprite-demo: collision 0x1000`. | User (programme acceptance), AI — delegated | ✅ Resolved |
| AR-10 | Compatibility | Does the 3.1 profile change? | The module is available on both hosted profiles; no existing API changes. The 1.3 whitelist adds `std.amiga.sprite`. | User (Bundle 0 rule), AI — delegated | ✅ Resolved |
| AR-11 | Errors | What happens on allocation or invalid input? | Constructors return `T?`; invalid channel, height, pixel count, or coordinates are no-ops or `null`; the shim never traps. | AI — delegated (auto-design) | ✅ Resolved |
| AR-12 | Documentation | Which docs move with the code? | `spec/appendix-amiga.md`, `spec/appendix-stdlib.md`, `AGENTS.md`, registry/coverage, issue #3, programme/roadmap, plan. | User (Bundle 0 rule), AI — delegated | ✅ Resolved |
| AR-13 | Language | Any language/grammar/diagnostic/ABI change? | None. | User (programme rule) | ✅ Resolved |
| AR-14 | Performance | Frame budget? | Sprite moves are register writes; the demo keeps the background static and moves only sprites. The A500 run reports the frame count and ticks. | AI — delegated (auto-design) | ✅ Resolved |
| AR-15 | Testing | Where do the tests live? | `compiler/tests/amiga_sprite_spec_test.rs`, `amiga_sprite_impl_test.rs`, and `docker/scripts/a500-sprite-smoke.sh`. | AI — delegated (auto-design) | ✅ Resolved |

## Open items

None. Every row is resolved; there is no deferred material decision.
