<!-- title: [bug] Wait for the blitter before CPU access after layer drawing -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:shims,tier:core,tier:advanced,bug,amiga-sdk -->
<!-- parent: R9 -->

## Summary
A checksum or pixel read right after a layer fill can see partly drawn data; CPU writes can be overwritten by the finishing blit. Intermittent, timing-dependent wrong output.

| Field | Value |
|---|---|
| Finding | ASM-WS5-017 · DEF-605 |
| Severity / Priority / Rank | Major / P1 / 24 of 58 |
| Domain / Lens | shims / A500, A1200 shipped |
| Confidence / Verdict | probable / confirmed (Major, probable) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Code reading. The layer helpers draw into a caller-owned `cl_video_bitmap` with blitter-backed graphics calls and return; the same bitmap can then be read or written by the CPU paths (`bitmapPlot`, `bitmapChecksum`, `bitmapClear`). The shim's own blitter paths call `WaitBlit()` for exactly this reason (`:3973`, `:4148`, `:4180`); RKM: wait for the blitter before the CPU touches memory it may still be writing.

## Root cause
missing `WaitBlit` in three helpers.

Location: `compiler/runtime/shim_hosted_amiga.c:5974-5975` (`RectFill`), `:5986` (`SetRast`), `:6007` (`Text`) in the layer helpers; CPU readers at `:3919-3926`, `:3986`, `:4198-4205`, `:4227`, at e24a8752

## Recommended fix
`WaitBlit()` before returning from the layer drawing helpers (or before every CPU access path).

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
—. Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS5-shims.md`.
