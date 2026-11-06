# Testing Strategy: Amiga RTG

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Principles

The specification suite derives from [Requirements](01-requirements.md) and
[API and Ownership](03-01-api.md), not from the shim. A failing spec test means
the implementation is wrong. The implementation suite pins the P96 calls,
ownership, bounds, and emitted C. The runtime gate is the only proof of the
actual A1200/UAEGFX stack.

## Specification Test Cases

| ID | Input | Expected result | Trace |
|---|---|---|---|
| ST-1 | Look up `std.amiga.picasso96` | module and every export exist; source is embedded | R1, R13 |
| ST-2 | Compile a source using screen/bitmap for host, `hosted-amiga-13`, freestanding | exit 1, `E11200`, no `out.c` | R1 |
| ST-3 | Compile the same source for `hosted-amiga` | exit 0; C contains the `cl_rtg_*` calls; no diagnostics | R1–R9 |
| ST-4 | Strict m68k GCC cross-build (`nix20`) | warning-free build | R9, R11 |
| ST-5 | Compile the source for `hosted-amiga` with vbcc | warning-free build; corpus RTG leg is explicit | R11 |
| ST-6 | Appendix §14.23 exports | every module export appears in the section | R10, R13 |
| ST-7 | Drop order: screen/bitmap lexical binding | generated cleanup calls the exact `cl_rtg_*` drop once | R4, R5, R13 |

## Implementation Test Cases

| ID | Pin | Trace |
|---|---|---|
| IT-1 | `OpenLibrary("Picasso96API.library", 2)` and one `P96Base` | R2 |
| IT-2 | `p96GetRTGDataTagList` + `P96RD_NumberOfBoards` | R2 |
| IT-3 | `p96BestModeIDTagList` with width/height/depth/formats | R3 |
| IT-4 | `P96SA_DisplayID` + `P96SA_Quiet` + `p96OpenScreenTagList` | R4 |
| IT-5 | `p96GetBitMapAttr` for screen/bitmap attributes | R4, R5 |
| IT-6 | `p96AllocBitMap` + `BMF_USERPRIVATE` + `p96FreeBitMap` | R5 |
| IT-7 | `p96WritePixel`/`p96ReadPixel`/`p96RectFill` routing | R6 |
| IT-8 | bitmap fill uses the bounded `p96WritePixel` loop, not `p96RectFill` | R6, AR-8 |
| IT-9 | `BltBitMapRastPort` with minterm `0xC0` | R7 |
| IT-10 | `p96ReadPixelArray` with `RGBFB_R8G8B8` and `width * 3` row | R7 |
| IT-11 | bounds checks: null handle, inverted rectangle, buffer too small, width > 4096 | R6, R12 |
| IT-12 | drop exactly once for screen and bitmap | R4, R5 |

## Runtime Gate

| Row | Gate | Artifact | Expected |
|---|---|---|---|
| `SDK-RTG-001` | `sdk-a1200-rtg` | `build/amiga-sdk/a1200-rtg/hd/rtg.txt` | `rtg: ok boards 1 mode 640x480x32 red 1 green 1 blue 1 blit 1` |
| `SDK-RTG-001` | `sdk-a1200-rtg` | `build/amiga-sdk/a1200-rtg/hd/rtg.ppm` | P6, 64x64, 255; relative red (0,0), green (1,0), blue (2,0), yellow (10,10), blue (11,11), blue (20,20) |
| `SDK-RTG-001` | `sdk-a1200-rtg` | `docs/amiga-sdk/a1200-rtg.png` | converted capture for inspection |

The gate uses a clean `build/amiga-sdk/a1200-rtg` directory, a private Xvfb
display, and a bounded 90-second wait. The host PPM parser validates the
header, payload length, and named pixels before indexing.

## Verification Matrix

| Command | What it proves |
|---|---|
| `cargo test --test amiga_rtg_spec_test` | module/profile/exports/cross-build |
| `cargo test --test amiga_rtg_impl_test` | shim calls, bounds, drop |
| `make check` | whole workspace, including the new suites |
| `make sdk-a1200-rtg` | actual P96 A1200/UAEGFX gate |
| `make corpus-amiga` | both toolchains plus the explicit RTG build-only leg |
| `make verify` | the full acceptance gate with the new leg |
