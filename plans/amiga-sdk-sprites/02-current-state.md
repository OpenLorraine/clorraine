# Current State: Amiga SDK Bundle 2 — Sprites and Collision

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing implementation

Bundle 1 added `std.amiga.video` (chip memory, bitmaps, drawing, blitter,
copper, custom `LoadView` display, palette, checksum), `std.math`, and the A500
Boing acceptance. The display owns two views and restores `GfxBase->ActiView`
on close. `std.amiga.custom` has a volatile overlay for `DMACON` and color
registers.

The shim currently includes `<graphics/gfx.h>`, `<graphics/view.h>`,
`<graphics/gfxbase.h>`, and the 1.3 graphics/intuition headers. It has no
`hardware/custom.h` include and no sprite code.

## Gaps

| Gap | Required change |
|---|---|
| No sprite module | Add `stdlib/std.amiga.sprite.clo` with `Sprite` and `SpriteAnimation` |
| No registration | Add the module to `SOURCES`/`MODULES` and the 1.3 whitelist |
| No sprite hardware code | Chip image allocation, pixel packing, `SPRxPT/POS/CTL`, `DMACON`, `CLXCON`, `CLXDAT` |
| No attach pair | Odd-channel ATTACH bit and shared position |
| No collision API | `enableCollisions`/`collisions` |
| No animation | Frame buffers and pointer switching |
| No demo/smoke | `sprite-demo.clo` and the A500 sprite smoke |

## Risks

| Risk | Mitigation |
|---|---|
| `LoadView`/`MrgCop` clears sprite DMA | Every show/move path sets `DMACON` `DMAEN|SPREN`; the demo calls `show` after the first flip |
| Sprite data format is wrong | Pack two words per line and a two-word terminator; the live frame and collision line catch it |
| Collision bits include playfield noise | The API returns the raw `CLXDAT`; the demo masks bits 9..14 |
| Attached pair position mismatch | `attach` requires adjacent channels; the demo moves both to the same x/y |
| A500 timing | The background is static; only sprite registers move; the demo reports frames/ticks |
