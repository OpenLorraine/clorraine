<!-- title: [bug] Read 2.0+ screen colours correctly in the native capture -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:shims,tier:core,tier:advanced,bug,amiga-sdk -->
<!-- parent: R9 -->

## Summary
Every native capture on 2.0+ records wrong colours; the `sdk-a1200-workbench` gate compares against a golden made by the same code, so it pins the defect instead of catching it.

| Field | Value |
|---|---|
| Finding | ASM-WS5-008 · DEF-597 |
| Severity / Priority / Rank | Major / P1 / 16 of 58 |
| Domain / Lens | shims / A1200 shipped (every 2.0+ capture); A500 unaffected (`Type == 0` branch) |
| Confidence / Verdict | verified / confirmed (Major, verified) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Measurement — `convert docs/amiga-sdk/a1200-workbench.png -format %c histogram:info:-` lists the Workbench capture's colours as `(170,170,0)` (617,972 px), `(102,187,0)`, and `(0,255,255)`. The NDK (`graphics/view.h:145-177`) keeps `ColorMap.ColorTable` as 12-bit `UWORD`s in every version (V39 adds `LowColorBits` separately). Decoding the KS3.1 default words `0AAA 0000 0FFF 068B` as byte triples with `(b & 0x0F) * 17` gives exactly `(AA,AA,00)`, `(00,FF,FF)`, `(66,BB,…)`: the expected grey, black, and white are captured as yellow, cyan, and green. The branch also reads `count` bytes past the end of the table.

## Root cause
the V36+ branch assumes a byte-triple table that no `ColorMap` type has.

Location: `compiler/runtime/shim_hosted_amiga.c:2764-2779` (`cl_screen_save_native`, the `else` branch reads `table[index * 3 + k]` bytes), used by `shim_hosted_amiga_20.c:494`, at e24a8752

## Recommended fix
read colours with `GetRGB4(colormap, index)` on every version (or `GetRGB32` on V39 for 8-bit precision); regenerate the A1200 Workbench goldens.

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
WS8 test integrity (a golden that encodes a defect); `docker/scripts/cscr2png.py`. Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS5-shims.md`.
