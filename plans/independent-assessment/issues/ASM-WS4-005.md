<!-- title: [performance] Stop linking unused libamiga stubs and CreatePort -->
<!-- labels: assessment,kind:finding,severity:opportunity,priority:P2,domain:optimizer,tier:core,performance,enhancement -->
<!-- parent: R12 -->

## Summary
Every Amiga program links libamiga DOS stubs and `CreatePort` it does not use: about 440 B of the 776 B fixed `.text` gap of the calibration program over C (Phase 9, `verification/verdicts-9.2.3.md`).

| Field | Value |
|---|---|
| Finding | ASM-WS4-005 |
| Severity / Priority / Rank | Opportunity / P2 / 55 of 58 |
| Domain / Lens | optimizer / A500 shipped -O2 |
| Confidence / Verdict | verified / confirmed (Opportunity, verified), magnitude corrected (independent verification) |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Measurement — `425-size.sh` with a link map (`build/tmp/assessment/WS4/size/calibration.clo.map`): the `calibration` link includes `libamiga.a(createdir.o … unlock.o)` stubs (20–28 B each) and `CreatePort.o` (188 B) although the program's own retained `.text` is 412 B; Clorraine `calibration` `.text` 5,900 B vs C 5,124 B (+776 B, +15%).

## Root cause
the archive members are extracted for references in shim functions that `--gc-sections` later removes, and the stub objects stay linked.

Location: references from the embedded shim (`shim_hosted_amiga.c`) to `CreateDir`, `CurrentDir`, `DateStamp`, `Delay`, `DeleteFile`, `Examine`, `Info`, `Lock`, `ParentDir`, `Rename`, `UnLock`, `CreatePort`, at e24a8752

## Recommended fix
move rarely used shim functions to the optional suffixes (as already done for 2.0+ services), or reference the DOS calls through the profile-specific suffix only when the module is imported.

## Acceptance criteria
- The behavior stated under Recommended fix, checked against the cited spec section or autodoc
- A regression test that fails without the fix and passes with it
- The measured target on the stated lens, with the same lens and flags as the evidence (net 68000 instructions under `vamos` or `.text` bytes; compile time for the compile-time items)

## Verification
- AGENTS.md gates: `make check`; `make bench` and `make bench-compare` (and `make bench-compile` for compile time)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, memory-safety auditor, emission-integrity auditor

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
ASM-WS4-003. Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS4-code-quality.md`.
