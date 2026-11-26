<!-- title: [bug] Handle runtime faults inside worker tasks without DOS calls or exit() -->
<!-- labels: assessment,kind:finding,severity:critical,priority:P0,domain:shims,tier:core,tier:advanced,bug,amiga-sdk -->
<!-- parent: R1 -->

## Summary
A worker that divides by zero crashes the machine instead of reporting a fault; the user cannot honor the "no DOS" rule because the compiler adds the DOS call.

| Field | Value |
|---|---|
| Finding | ASM-WS5-005 · DEF-586 |
| Severity / Priority / Rank | Critical / P0 / 5 of 58 |
| Domain / Lens | shims / A500, A1200 shipped |
| Confidence / Verdict | verified / confirmed (Critical; confidence probable→verified) (independent verification) |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Code reading. The documented contract allows arithmetic in the entry, and the compiler inserts fault checks into arithmetic and indexing (`compiler/src/codegen/expr.rs:985`, `codegen/guards.rs:2118`). A division by zero or an index fault in the entry calls `cl_trap`, which calls libnix `printf` (DOS `Write` from a task without a `Process` structure) and `exit()` (unwinding the main program's startup frame from another task's stack). RKM Libraries: DOS calls require a Process.

## Root cause
one fault path for every execution context; it assumes it runs in the main process.

Location: `compiler/runtime/shim_hosted_amiga.c:3120-3135` (`cl_task_entry` calls the Clorraine entry on a `CreateTask` task), `:544-549` (`cl_trap`), `stdlib/std.amiga.exec.clo:215-222` (the entry "must use only arithmetic … must not call DOS … `std.io`"), at e24a8752

## Recommended fix
in `cl_trap`/`cl_panic`, detect a non-process task (`FindTask(NULL)->tc_Node.ln_Type == NT_TASK`) and record the fault in the task record, signal the parent, and `Wait(0)`; `joinTask` reports it as a fault in the parent.

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
ASM-WS5-001. Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS5-shims.md`.
