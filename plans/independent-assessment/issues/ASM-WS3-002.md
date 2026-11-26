<!-- title: [bug] Keep generic instance symbols apart from module function symbols -->
<!-- labels: assessment,kind:finding,severity:critical,priority:P0,domain:codegen,tier:core,tier:advanced,bug -->
<!-- parent: R3 -->

## Summary
An accepted program emits invalid C when a module's lowercase path plus an underscore-led function name spells another generic's instance symbol. The trigger needs an unusual spelling (a module named like a generic function and a function name starting with `_`), so it is rare in practice.

| Field | Value |
|---|---|
| Finding | ASM-WS3-002 · DEF-592 |
| Severity / Priority / Rank | Critical / P0 / 11 of 58 |
| Domain / Lens | codegen / all |
| Confidence / Verdict | verified / confirmed (Critical, verified) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Reproduction — probe folder `probes/WS3/342-generic-key-collision/` (module `get` exports `_Point`; module `Probe.Main` instantiates `get<Point>`), command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS3 --clc build/tmp/assessment/baseline/clc --sanitize --only 342"`. Expected (spec/17 §7: exported function `cl_<module>_<name>`, generic instance `cl_<name>__<typekey>`; "a user identifier cannot collide with a generated key"): builds and prints `1 102`. Observed on all three profiles: `error: conflicting types for 'cl_get__Point'; have 'ClPoint(ClPoint)'`.

## Root cause
the two mangling schemes share the `cl_` prefix and the separator, so `cl_` + `get` + `_` + `_Point` equals `cl_` + `get` + `__` + `Point`; the key escaping protects the type key, not the module/name boundary.

Location: symbol mangling in `compiler/src/codegen/program.rs` and the generic key scheme of spec/17 §7, at e24a8752

## Recommended fix
give generic instances a distinct prefix (for example `clg_`) or escape a function name that begins with `_`; add a collision golden.

## Acceptance criteria
- The behavior the specification requires: builds and prints `1 102` (source: spec/17 §7: exported function `cl_<module>_<name>`, generic instance `cl_<name>__<typekey>`; "a user identifier cannot collide with a generated key")
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; `make asan` and `make ubsan`; `make corpus-amiga` (m68k GCC and vbcc legs)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, memory-safety auditor, emission-integrity auditor

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
ASM-WS3-001; `docs/known-limitations.md` §7 (the user-symbol collision check renames functions that collide with shim names and with `cl_init_*`, `cl_zero_ClSlice_*`, `cl_lambda_*` helpers, but generic instance symbols `cl_<name>__<key>` are not in that list); DEF-140 (a different generic-key collision, fixed). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS3-codegen.md`.
