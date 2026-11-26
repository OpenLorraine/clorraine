<!-- title: [bug] Report E10305 for polymorphic recursion with explicit type arguments -->
<!-- labels: assessment,kind:finding,severity:critical,priority:P0,domain:semantics,tier:core,bug -->
<!-- parent: R4 -->

## Summary
A small, plausible generic program (a recursive container walker that wraps its element type) hangs the compiler and the language server.

| Field | Value |
|---|---|
| Finding | ASM-WS1-005 · DEF-591 |
| Severity / Priority / Rank | Critical / P0 / 10 of 58 |
| Domain / Lens | semantics / host (the compiler process; every profile is affected) |
| Confidence / Verdict | verified / confirmed (Critical) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Reproduction — `065-robustness.py` case `generic-recursion` (288 bytes: `func g<T>(x: T, n: i32): i32 { … return g<W<T>>(w, n - 1); }`). Expected (spec/07-structs.md STR-24: instantiation recursion beyond the cap is E10305; 03-01 §Severity: a hang is Critical): a coded diagnostic within the limit. Observed: no exit within 600 s (killed); a second run under `timeout 20` also did not finish (`rc=124`).

## Root cause
E10305 is judged only on the inferred-type-argument path (`if infer { … is_recursive_instantiation … }`); an explicit type-argument list skips the check, and monomorphization has no instantiation-depth cap of its own.

Location: `compiler/src/sema/call.rs:1314-1330`, `compiler/src/sema/call.rs:957-971` at e24a8752

## Recommended fix
judge `is_recursive_instantiation` for explicit type arguments too, and add an instantiation-depth cap in monomorphization that reports E10305 as a backstop.

## Acceptance criteria
- The behavior the specification requires: a coded diagnostic within the limit (source: spec/07-structs.md STR-24: instantiation recursion beyond the cap is E10305; 03-01 §Severity: a hang is Critical)
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; `make asan` and `make ubsan`
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, memory-safety auditor, semantics reviewer

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
WS2-08 (generics); DEF-397 (`qualified-access-residue`, fixed): "an explicit type-argument list fixes the instance", which is why the explicit path skips the recursion check. Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS1-semantic-core.md`.
