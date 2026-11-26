<!-- title: [bug] Give the busy pointer a complete sprite definition -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:shims,tier:core,tier:advanced,bug,amiga-sdk -->
<!-- parent: R9 -->

## Summary
`setPointer` shows a corrupted pointer (first row lost, trailing garbage lines from whatever follows in chip memory). Not visible to the bitplane-only capture of the GUI gate.

| Field | Value |
|---|---|
| Finding | ASM-WS5-010 · DEF-607 |
| Severity / Priority / Rank | Major / P1 / 26 of 58 |
| Domain / Lens | shims / A500, A1200 shipped |
| Confidence / Verdict | probable / confirmed (Major, probable) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Code reading against the RKM pointer format and graphics `ChangeSprite` (sprite data = 2 position/control words, then height × 2 words, then 2 zero words): a 16-row pointer needs 36 words (72 bytes). The buffer holds 32 words with neither part, so `ChangeSprite` writes the control words over row 0 and the DMA fetches row 16 and the terminator from 8 bytes past the allocation.

## Root cause
the template stores image words only.

Location: `compiler/runtime/shim_hosted_amiga.c:1412` (`CL_WINDOW_POINTER_BYTES 64`), `:2648-2680` (template of 32 words, `SetPointer(…, 16, 16, 0, 0)`), at e24a8752

## Recommended fix
allocate 72 bytes, place the image at word 2, zero the first and last two words.

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
ASM-WS5-019 (sprite channel on screen close). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS5-shims.md`.
