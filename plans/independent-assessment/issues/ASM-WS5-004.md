<!-- title: [bug] Clip drawText on screens that have no layer -->
<!-- labels: assessment,kind:finding,severity:critical,priority:P0,domain:shims,tier:core,tier:advanced,bug,amiga-sdk -->
<!-- parent: R5 -->

## Summary
A safe program that draws text near the screen edge (a scroller, a status line with a long string, an off-by-one y) overwrites chip memory — other bitplanes, copper lists, or another program's data. Memory unsafety from the safe surface.

| Field | Value |
|---|---|
| Finding | ASM-WS5-004 · DEF-585 |
| Severity / Priority / Rank | Critical / P0 / 4 of 58 |
| Domain / Lens | shims / A500, A1200 shipped |
| Confidence / Verdict | verified / confirmed (Critical; confidence probable→verified) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Code reading. `drawText(screen, font, x, y, pen, text)` passes the caller's `x`, `y`, and the full text length straight to `Text()` on the screen's `RastPort`. A screen `RastPort` has `Layer == NULL`; graphics clips only through layers (graphics autodocs: `ClipBlit` "takes into account the Layers and ClipRects"; `DrawEllipse` "does not clip … to a non-layered rastport"; RKM Graphics: an unlayered RastPort is not clipped). So a position or string extent outside the bitmap makes `Text()` write glyph rows past the bitplanes into adjacent chip memory. The shim's other drawing entries clamp (`cl_video_bitmap_fill`, `:3953-3968`) or draw through a window's layered RastPort (`cl_window_text_draw`).

## Root cause
the wrapper treats `Text()` as bounds-safe; it is not on an unlayered RastPort.

Location: `compiler/runtime/shim_hosted_amiga.c:5800-5815` (`cl_text_draw`: `Move` + `Text` on `&screen->RastPort`), `stdlib/std.amiga.text.clo:75` (`drawText`, a safe export), at e24a8752

## Recommended fix
in `cl_text_draw`, compute the extent with `TextExtent`/`TextLength` and the font's baseline/height and reject (or truncate to) text that does not fit the screen bitmap; or draw through a layer.

## Acceptance criteria
- The behavior stated under Recommended fix, checked against the cited spec section or autodoc
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; the `make verify` FS-UAE legs for the touched modules (`sdk-a500-*`, `sdk-a600-*`, `sdk-a1200-*`)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, amiga-platform-reviewer

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
ASM-WS5-019 (16-bit text length, `RectFill` order); WS6 (safe-surface review). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS5-shims.md`.
