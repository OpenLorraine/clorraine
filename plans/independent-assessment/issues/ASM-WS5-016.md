<!-- title: [bug] Finish the clipboard read protocol so a later write does not hang -->
<!-- labels: assessment,kind:finding,severity:critical,priority:P0,domain:shims,tier:advanced,bug,amiga-sdk -->
<!-- parent: R5 -->

## Summary
After `clipboardRead`, a `clipboardWrite` on the same handle never returns on an A1200 with Kickstart 3.1 — the read never finishes the clipboard read protocol, so the device holds writes off. A program that reads, then writes the clipboard hangs.

| Field | Value |
|---|---|
| Finding | ASM-WS5-016 · DEF-588 |
| Severity / Priority / Rank | Critical / P0 / 7 of 58 |
| Domain / Lens | shims / A1200 shipped |
| Confidence / Verdict | verified / upgraded Major→Critical (confidence probable→verified) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Code reading. clipboard `CMD_READ`: "When io_Offset is beyond the end of the clip, this acts as a signal to the clipboard device that the application is through reading this clip… while an application is in the middle of reading a clip, any attempts to write new data to the clipboard are held off."

## Root cause
the read is a single request sized to the caller's buffer.

Location: `compiler/runtime/shim_hosted_amiga_optional_services.c:687-710` (`cl_clipboard_read`: one `CMD_READ`, never past the end), at e24a8752

## Recommended fix
after the data read, issue a read with `io_Data = NULL` and a large `io_Length` (the autodoc's skip-to-end idiom) to finish the clip.

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
