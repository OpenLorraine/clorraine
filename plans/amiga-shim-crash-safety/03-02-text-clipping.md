# Screen text clipping: Amiga shim crash-safety

> **Document**: 03-02-text-clipping.md
> **Parent**: [Index](00-index.md)

## Overview

`drawText` must not write outside the screen bitmap (FR-2). Text that fits keeps today's exact
path; text partly outside is clipped at the pixel edge through a small temporary raster; anything
else draws nothing (AR-3).

## Architecture

### Current Architecture

`cl_text_draw` (`shim_hosted_amiga.c:6695-6713`): `SetFont`, `SetAPen`, `Move(x, y)`,
`Text(text, count)`, restore the font, all on `&screen->RastPort`, which has no layer.

### Proposed Changes

`cl_text_draw` keeps its null checks and font save/restore and gains a classification step:

1. **Range** (AR-5): `x` or `y` outside -32768..32767, or `count` above 32767 → draw nothing.
2. **Visible rectangle** (AR-5): `0..W-1` by `0..H-1`, with `W = min(screen->Width,
   BitMap->BytesPerRow * 8)` and `H = min(screen->Height, BitMap->Rows)` of the `RastPort`'s bitmap.
3. **Superset box** (AR-5): walk the characters from the pen position `x`, computing for each a
   box that contains every pixel `Text()` can touch for it:
   - glyph index: `c - tf_LoChar` for `tf_LoChar <= c <= tf_HiChar`, else the default glyph
     `tf_HiChar - tf_LoChar + 1`;
   - left edge: the smaller of the pen position and the pen position plus `tf_CharKern[i]` when
     `tf_CharKern` is non-NULL;
   - right edge: the larger of (left + `tf_CharLoc[i]` bit width), (pen + `tf_CharSpace[i]` when
     non-NULL), and (pen + `tf_XSize`); algorithmic bold (`rp->AlgoStyle & FSF_BOLD`) adds
     `tf_BoldSmear`, italic (`FSF_ITALIC`) adds `tf_YSize`;
   - advance: `tf_CharSpace[i]` (or `tf_XSize`) plus `tf_CharKern[i]` when present, plus
     `rp->TxSpacing`; an `FPF_REVPATH` font advances leftwards.
   - vertical: `y - tf_Baseline` to `y - tf_Baseline + tf_YSize - 1`.
   All arithmetic is `LONG`. The string's box is the union.
4. **Classify:** fully inside → today's path, unchanged. Fully outside → nothing.
   Colour font (`tf_Style & FSF_COLORFONT`) and not fully inside → nothing (AR-5).
5. **Clip path** (AR-4), for a partly visible plain font:
   - cull to the run of characters whose boxes meet `0..W-1` horizontally; its box `R` is the
     union of their boxes; a run wider than `W + 512`, or wider than 1008 px when
     `GfxBase->LibNode.lib_Version < 36`, draws nothing;
   - allocate a two-plane `BitMap` of `R`'s size (`AllocMem(MEMF_CHIP | MEMF_CLEAR)` for each plane,
     `InitBitMap`) and a temporary `RastPort` (`InitRastPort`) on it; copy the screen `RastPort`'s
     font, `DrawMode`, `AlgoStyle`, and `TxSpacing`; set APen 3, BPen 2;
   - `Move` to the run's first pen position relative to `R`, `Text()` the run;
   - visible sub-rectangle `V = R ∩ visible`; with `S` = the screen `RastPort`:
     - mode without `COMPLEMENT`: `BltTemplate(plane 1, V)` onto `S` in JAM1 with `S`'s BPen, then
       `BltTemplate(plane 0, V)` in JAM1 with `S`'s APen;
     - mode with `COMPLEMENT`: `BltTemplate(plane 0, V)` in `COMPLEMENT`;
     the `S` pens and mode are restored afterwards;
   - `WaitBlit()`, then free the planes. Any allocation failure draws nothing.
6. Restore the screen font as today.

Plane 1 marks every pixel `Text()` touched (APen 3 and BPen 2 both set it) and plane 0 every pixel
it drew with the APen, so the two template blits reproduce JAM1, JAM2, and `INVERSVID` exactly
for the visible part (AR-4).

### New Functions

- `static int cl_text_box(...)`: the superset walk of step 3 (and the cull of step 5).
- `void cl_text_draw_clipped(...)`: step 5. External linkage, like `cl_exit_hook`, so the
  release-image symbol check (ST-3) sees it; section GC still drops it from programs that never
  call `drawText` (AR-15).
- No stdlib-facing entry point is added.

### Integration Points

- Only `cl_text_draw` changes. `cl_window_text_draw` and `cl_layer_text` draw through layers and
  stay unchanged. The existing goldens draw only fully inside text (`examples/amiga/gui.clo:80-90`).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Coordinates or count out of range | draw nothing | AR-5 |
| Box fully outside | draw nothing | AR-3 |
| Partly visible colour font | draw nothing | AR-5 |
| Run too wide for the raster rule | draw nothing | AR-5 |
| Chip allocation fails | draw nothing | AR-4 |

## Testing Requirements

- Spec: ST-9 (A1200, the verifier's hang), ST-10 (A500 capture comparison and `chip` equality).
- Impl: the classification order; the default-glyph rule; `LONG` arithmetic; `WaitBlit` before the
  frees; the colour-font rule; that the fully inside path is the old call sequence.
