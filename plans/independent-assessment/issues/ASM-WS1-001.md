<!-- title: [correctness] Range-check decimal literals in inferred-type bindings -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:semantics,tier:core,tier:advanced,bug -->
<!-- parent: R4 -->

## Summary
A program with an un-annotated large decimal constant (for example a 32-bit mask or address written in decimal) compiles and runs with a different value, or emits C that the strict build rejects; the spec requires a compile error.

| Field | Value |
|---|---|
| Finding | ASM-WS1-001 · DEF-601 |
| Severity / Priority / Rank | Major / P1 / 20 of 58 |
| Domain / Lens | semantics / all |
| Confidence / Verdict | verified / confirmed (Major) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Reproduction — probe `probes/WS1/002-default-i32-literal.clo` (`let a = 2147483648;`), command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS1 --clc build/tmp/assessment/baseline/clc --only 002"`. Expected (spec/01-lexical.md §7 LEX-14/LEX-15, spec/02-types.md §9 TY-15): reject with E10202, because a decimal literal defaults to `i32` and 2147483648 does not fit. Observed: accepted on host, hosted-amiga, hosted-amiga-13 (`rc=0`); the emitted C is `i32 a = 0; … a = 2147483648u;`. A follow-up program (`let a = 2147483648; let c = 4294967296;`, `build/tmp/assessment/WS1/diag/lit2.clo`) printed `-2147483648 0` on host, and host GCC reports `-Woverflow` for `c = 4294967296u`, so the emitted C fails the strict debug line (`-Werror`). Every annotated or operator context rejects correctly: `let a: i32 = 2147483648;`, `a = 2147483648;`, `1 + 2147483648`, and `let a: u32 = 4294967296;` are all E10202; a literal above `u64` is E20004.

Probe `plans/independent-assessment/probes/WS1/002-default-i32-literal.clo` (baseline `e24a8752`):

```
module Probe.Main;

import { println } from std.io;

func main(): i32 {
    let a = 2147483648;
    println($"{a}");
    return 0;
}
```

## Root cause
`check_int` performs the fit check only when an expected integer type is present; with no context it returns `i32` without checking that the value fits, and codegen spells the original literal with a `u` suffix.

Location: `compiler/src/sema/expr.rs:690-707` at e24a8752

## Recommended fix
in `check_int`, when there is no expected type, check the value against `i32` and report E10202 (spec LEX-14/LEX-15); add a compile-fail golden for `let a = 2147483648;`.

## Acceptance criteria
- The behavior the specification requires: reject with E10202, because a decimal literal defaults to `i32` and 2147483648 does not fit (source: spec/01-lexical.md §7 LEX-14/LEX-15, spec/02-types.md §9 TY-15)
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; `make asan` and `make ubsan`
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, memory-safety auditor, semantics reviewer

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
WS1-04 (literal typing); WS3-01 (strict build). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS1-semantic-core.md`.
