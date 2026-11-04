# Current State: Amiga SDK Bundle 1 — Native Video

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing implementation

- `stdlib/std.amiga.graphics.clo` and `stdlib/std.amiga.intuition.clo` provide
  an Intuition custom-screen surface: `openScreen`, `setColor`, `fill`, `draw`,
  and `clear`. They lower to `cl_open_screen`, `cl_set_color`, `cl_fill`,
  `cl_draw`, and `cl_clear` in `compiler/runtime/shim_hosted_amiga.c`.
- The shim includes `<proto/graphics.h>`, `<proto/intuition.h>`, and
  `<graphics/rastport.h>`; it opens graphics/intuition at version 33 on both
  hosted profiles. It has no `View`, `BitMap`, `RastPort`, blitter, or copper
  code.
- `compiler/src/stdlib.rs` lists ten modules: `std.mem`, `std.text`, `std.fmt`,
  `std.io`, `std.amiga.dos`, `std.amiga.interrupt`, `std.amiga.intuition`,
  `std.amiga.graphics`, `std.amiga.custom`, and `std.amiga.exec`. The host
  profile accepts every non-Amiga module; the 1.3 profile uses an explicit
  whitelist; the freestanding profile accepts only `std.io`.
- The 1.3 toolchain is `m68k-amigaos-gcc -std=c99 -mcrt=nix13 -m68000 -fwrapv
  -O0 -g -Wall -Wextra -Werror ... -lamiga -lm`; the 3.1 profile is the same
  with `-mcrt=nix20`.
- `docker/scripts/a500-smoke.sh` decrypts the licensed `amiga-os-130.rom`,
  verifies SHA-1 `891e9a547772fe0c6c19b610baf8bc4ea7fcb785`, builds a clean
  `boot1x` ADF, boots FS-UAE as an A500, captures the inner window, and compares
  a 430x22 crop at +160+88 to `docs/amiga-sdk/a500-hello.png`.
- `make verify` runs `check`, `sdk-a500-smoke`, `differential`,
  `corpus-fs-uae`, and `run-fault-amiga`.

## Verified spike

A throwaway C spike in `build/spike/` (not committed) established the display
pipeline on the real ROM:

1. `AllocRaster(320, 256)` for four planes, `memset` to zero, plane 0 set to
   `0xFF`.
2. `InitView`/`InitVPort`, `DWidth=320`, `DHeight=256`, `DxOffset=0x81`,
   `DyOffset=0x2c`, `RasInfo.BitMap` set.
3. `MakeVPort(&view, &view_port)` then `MrgCop(&view)` then `LoadView(&view)`.
4. `LoadRGB4(&view_port, colors, 16)` for the palette.
5. Save `GfxBase->ActiView` before loading; restore it with `LoadView(oldview)`,
   then two `WaitTOF` calls.
6. `FreeCprList(view.LOFCprList)`, `FreeVPortCopLists(&view_port)`, free the
   raster planes.

The captured screen showed the red plane and the restore returned to the
AmigaDOS 1.3 shell. The earlier white/black results were caused by omitting
`MakeVPort`/`MrgCop`; this is the reason the design uses the full pipeline.

The same run also confirmed that the ADF takes roughly 18–20 seconds to reach
the startup sequence, so runtime captures must wait at least 25–30 seconds.

## Gaps

| Gap | Required change |
|---|---|
| No native video module | `stdlib/std.amiga.video.clo` with `ChipBuffer`, `Bitmap`, `Copper`, `Display` |
| No math module | `stdlib/std.math.clo` with fixed-point `sin`/`cos`/`sqrt` |
| No module registration | `SOURCES`/`MODULES` entries and the 1.3 whitelist |
| No chip/bitmap/blitter code | C shim allocation, plane drawing, `BltBitMap`, `BltMaskBitMapRastPort`, FNV-1a |
| No display/copper code | C shim `View`/`ViewPort` pipeline, `ColorMap`, `UCopList` builder, flip/restore |
| No runtime demo | `examples/amiga/boing.clo` and the A500 Boing smoke |
| No doc/registry updates | Appendix, registry, coverage, issue #2, roadmap |

## Risks

| Risk | Mitigation |
|---|---|
| `LoadView` restore leaves the system view broken | Save `GfxBase->ActiView`; restore it and wait two frames; the spike already proved the restore |
| The user copper list is freed twice | `displaySetCopper` takes ownership; the display frees it through `FreeVPortCopLists`; the standalone `Copper` drop is not called after a move |
| Masked blit is wrong on OCS | Use the documented minterm `0xE0` with a single-plane mask, and pin the result with the frame checksum |
| Palette changes after the first flip do not stick | Document and enforce "set colors before first flip"; the demo follows the rule |
| The A500 run is too slow | Two small blits per frame, no full-frame copy; report the frame count and use the checksum |
| FS-UAE capture misses the live display | Capture at 30 seconds, after the ~20-second boot; the demo holds the display for about 6 seconds |
