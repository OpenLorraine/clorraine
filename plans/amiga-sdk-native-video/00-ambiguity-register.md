# Ambiguity Register: Amiga SDK Bundle 1 — Native Video

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)
> **Status**: ✅ GATE PASSED
> **Mode**: auto-design (eligible technical decisions delegated and recorded)

## Scope baseline

Bundle 1 (`1-native-video`) from `docs/amiga-sdk/registry.tsv`: chip RAM
allocator, bitmap create/free and plane access, bitmap fill/plot/line, blitter
copy and masked copy, copper list builder/install/restore, VBlank/double
buffer/screen flip, 4-bit palette, sin/cos/sqrt helpers, frame checksum oracle,
and the Boing acceptance. No language, grammar, diagnostic, or ABI change.

## Register

| # | Category | Decision / question | Resolution | Owner | Status |
|---|---|---|---|---|---|
| AR-1 | Scope | Which capabilities belong to Bundle 1? | The eleven `1-native-video` registry rows, and nothing else. Sprites, audio, input, text, DOS assets, Intuition UI, Exec, Workbench, and AGA are later bundles. | User (registry), AI — delegated | ✅ Resolved |
| AR-2 | Architecture | One module or several? | Add `std.amiga.video` for the native-video surface and `std.math` for fixed-point helpers. `std.math` is host-available so its arithmetic can be tested without an Amiga; `std.amiga.video` is Amiga-only. | AI — delegated (auto-design) | ✅ Resolved |
| AR-3 | Ownership | Who owns the frame bitmaps? | The caller owns the two `Bitmap` handles; `Display` borrows them for its lifetime and never frees them. The display must be dropped before the bitmaps; the documentation states this and the Boing example follows it. | AI — delegated (auto-design) | ✅ Resolved |
| AR-4 | Display mechanism | Intuition screen, custom `LoadView`, or raw copper takeover? | A custom `graphics.library` view: `InitView` + `InitVPort` + `MakeVPort` + `MrgCop` + `LoadView`. A C spike on the real KS1.3 ROM proved this path and the restore path; Intuition is untouched. | AI — delegated (auto-design), verified by spike | ✅ Resolved |
| AR-5 | Copper | How is the copper list installed and restored? | `Copper` builds a 1.3 user copper list (`UCopperListInit`/`CWait`/`CMove`). `displaySetCopper(take)` attaches one list per viewport before the first flip; `displayFree` restores the saved `GfxBase->ActiView` and frees the merged lists. No raw `COP1LC` takeover is exposed. | AI — delegated (auto-design) | ✅ Resolved |
| AR-6 | Blitter | Raw blitter registers or graphics.library? | `BltBitMap` (minterm `0xC0`) for copies and `BltMaskBitMapRastPort` (minterm `0xE0`) for masked copies. The library owns the register setup and waits for the blitter. | AI — delegated (auto-design) | ✅ Resolved |
| AR-7 | Palette | Static palette or per-line animation? | A static 4-bit palette is set before the first flip. Animated per-line colors use the `Copper` list. Changing the palette after the first flip is not part of Bundle 1. | AI — delegated (auto-design) | ✅ Resolved |
| AR-8 | Math | libm doubles or fixed point? | Integer fixed-point `sin`/`cos` (angle 0..255, result -256..256) and integer `sqrt`. No soft-float dependency; results are deterministic on 68000. | AI — delegated (auto-design) | ✅ Resolved |
| AR-9 | Checksum | What is the frame oracle? | FNV-1a 32-bit over every plane byte, in plane order, seeded with depth/width/height. The Boing demo prints the final front-buffer checksum; the A500 smoke compares the printed line. | AI — delegated (auto-design) | ✅ Resolved |
| AR-10 | Acceptance | How is Boing proven on real 1.3? | Build `examples/amiga/boing.clo` for `hosted-amiga-13`, boot it on FS-UAE A500 with the decrypted licensed KS1.3 ROM, capture the live display screenshot, and compare the final `boing: checksum ...` shell line to a golden crop. | User (programme acceptance), AI — delegated (capture mechanism) | ✅ Resolved |
| AR-11 | Compatibility | Does the 3.1 profile change? | The two new modules are available on `hosted-amiga` and `hosted-amiga-13`; no existing 3.1 API changes. The 3.1 graphics corpus and Exec tests stay green. | User (Bundle 0 rule), AI — delegated | ✅ Resolved |
| AR-12 | Errors | What happens on allocation/display failure? | Every constructor returns `T?`; the caller checks. Drawing calls on a null handle are impossible by type. Invalid coordinates are clipped by the shim; invalid pen bits are masked to the bitmap depth. | AI — delegated (auto-design) | ✅ Resolved |
| AR-13 | Documentation | Which docs move with the code? | `spec/appendix-amiga.md` and `spec/appendix-stdlib.md` gain the module surface; `AGENTS.md` and the registry/coverage/issue #2 are updated. No frozen chapter changes. | User (Bundle 0 rule), AI — delegated | ✅ Resolved |
| AR-14 | Language | Any language/grammar/diagnostic/ABI change? | None. The bundle only adds embedded standard-library sources, C shim helpers, tests, and an example. | User (programme rule) | ✅ Resolved |
| AR-15 | Performance | What is the frame-rate target? | Boing renders at 50 Hz on the A500 using a small masked ball blit per frame plus a static background tile restore; no full-frame copy. The demo prints a frame count and the checksum so the run can be checked. | AI — delegated (auto-design) | ✅ Resolved |
| AR-16 | Testing | Where do the tests live? | `compiler/tests/amiga_video_spec_test.rs` for the spec surface, `compiler/tests/amiga_video_impl_test.rs` for emitted-C pins, `stdlib` unit tests for `std.math`, and `docker/scripts/a500-boing-smoke.sh` for the real-ROM runtime. | AI — delegated (auto-design) | ✅ Resolved |

## Open items

None. Every row is resolved; there is no deferred material decision.
