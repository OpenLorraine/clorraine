<!-- title: [performance] Keep large switch statements within vbcc's compile-time budget -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:codegen,tier:core,performance -->
<!-- parent: R11 -->

## Summary
A program with a large dispatch `switch` (a bytecode interpreter, a key or event table) cannot be built with the vbcc leg in reasonable time; the GCC legs are unaffected.

| Field | Value |
|---|---|
| Finding | ASM-WS3-007 · DEF-609 |
| Severity / Priority / Rank | Major / P1 / 28 of 58 |
| Domain / Lens | codegen / vbcc leg of hosted-amiga (build-only validity leg) |
| Confidence / Verdict | verified / confirmed (Major, verified) (independent verification) |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Measurement — `380-shapes.py` (`switch-3000`: vbcc did not finish within 1,200 s; limit 12.2 s = 10 × the 14,114-line Phase 0 vbcc time, `findings/WS0-emission-record.tsv`) and the scaling series `build/tmp/assessment/WS3/main2.sh` (records `records/WS3-scripts.log`): vbcc compile-only of the hosted-amiga emission with a 250-, 500-, 1,000-case `switch`: 1,831 ms, 7,995 ms, 57,489 ms. m68k GCC compiles the 3,000-case unit in 3.6 s and host cc in 0.28 s. Lens: hosted-amiga, `vc +aos68k -c99 -O2 -cpu=68000`, container host, baseline e24a8752.

## Root cause
vbcc's optimizer is super-linear in the number of `switch` arms; the emitter gives it one large function.

Location: emitted `switch` shape (one `case N: { … break; }` per arm) compiled by `vc +aos68k -O2`, at e24a8752

## Recommended fix
document the vbcc limit, or have the emitter split very large `switch` statements (for example a two-level dispatch), or compile very large functions with `-O1` on the vbcc leg.

## Acceptance criteria
- The behavior stated under Recommended fix, checked against the cited spec section or autodoc
- A regression test that fails without the fix and passes with it
- The measured target on the stated lens, with the same lens and flags as the evidence (net 68000 instructions under `vamos` or `.text` bytes; compile time for the compile-time items)

## Verification
- AGENTS.md gates: `make check`
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, memory-safety auditor, emission-integrity auditor

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
WS7-06 (compile-time scaling); `docs/known-limitations.md` lists vbcc limits for bsdsocket headers only. Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS3-codegen.md`.
