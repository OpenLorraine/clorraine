<!-- title: [performance] Lower Array push and get without runtime calls and one-element views -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:optimizer,tier:core,tier:advanced,performance -->
<!-- parent: R13 -->

## Summary
Container-heavy code (lists of game objects, parsed records) pays about 40% more CPU on the 68000 than the equivalent C growable array.

| Field | Value |
|---|---|
| Finding | ASM-WS4-002 · DEF-611 |
| Severity / Priority / Rank | Major / P1 / 30 of 58 |
| Domain / Lens | optimizer / A500 shipped -O2 (also A500 -Os +57.8%, A1200 shipped +38.8%, 68020 probe +64.9%) |
| Confidence / Verdict | verified / confirmed (Major, verified) (independent verification) |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Measurement — Phase 0 matrix (A500 shipped -O2 (hosted-amiga-13, `-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`, `vamos` 68000), pinned `clc` sha256 `0c2fd66e…`, baseline e24a8752): `container` Clorraine 57,198 vs C reference 41,312 net instructions (+38.5%; raw 60,117 / 44,230). Attribution: `cl_buffer_reserve` 8,465 instructions (called on every one of 1,024 pushes), `memset` 1,028 (zeroing each grown region), `main` +5,987 (the view and its second check per element). Patches: 412 (`push` with an inline capacity test and a direct store) 60,117 → 46,960 (−23.0% of net); 411 (`get(i)[0]` as one check and an indexed read) → 56,045 (−7.1%); outputs identical.

## Root cause
`push` always calls the runtime reserve (which returns at once when the capacity suffices) and then builds a one-element view checked twice; `get` returns a view that the caller indexes with a second check.

Location: `stdlib/std.mem.clo` `push`/`get` (lines 110-132 per the emission `#line`s) lowered to `cl_buffer_reserve` per push and a one-element view per get; `compiler/runtime/shim_hosted_amiga.c:803-818` (`cl_buffer_reserve`), at e24a8752

## Recommended fix
make the `Array` element access compiler-known: inline the capacity test with the reserve call on the slow path only, store directly, and lower `get(i)[0]` to one bounds check and an indexed read.

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
WS4-09; DEF history of `Array` ownership (WS8); `plans/real-workload-audit/99-execution-plan.md` (concluded the `get`/`push` instances are inlined into `main` and left them unchanged; amortized reserve growth followed in `growable-array-growth`). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS4-code-quality.md`.
