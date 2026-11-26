<!-- title: [correctness] Emit value-position increments and decrements without signed overflow -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:codegen,tier:core,bug -->
<!-- parent: R2 -->

## Summary
A program that wraps an `i32` (or `isize`) through a value-position `++`/`--` relies on undefined behavior when built on the host profile or with vbcc; an optimizing compiler may assume the wrap never happens (loop bounds, comparisons).

| Field | Value |
|---|---|
| Finding | ASM-WS3-003 · DEF-602 |
| Severity / Priority / Rank | Major / P1 / 21 of 58 |
| Domain / Lens | codegen / host; vbcc leg (both build without `-fwrapv`); the m68k GCC legs are defined by `-fwrapv` |
| Confidence / Verdict | verified / confirmed (Major, verified) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Reproduction — probe `probes/WS3/310-increment-wrap.clo`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS3 --clc build/tmp/assessment/baseline/clc --sanitize --only 310"`. Expected (spec/02 §10 and spec/04 §5: every operator is defined and integer arithmetic wraps; spec/17 §14: "Signed overflow of +, -, *, unary - — wrapped through unsigned-based forms or -fwrapv"): prints the wrapped values, no UBSan report. Observed: the debug and release lines print the expected line on all profiles, but the host UBSan line reports `runtime error: signed integer overflow: 2147483647 + 1 cannot be represented in type 'int'` at `return ++y;`. The emitted C for that line is `return ++(y);`; WS1 probe 021 shows `__cl_tmp0 = (i)++;` for `xs[i++]`. Statement-position increments and compound assignments are lowered safely (`y = (i32)((u32)y + (u32)1);`), as is unary minus (`(i32)(-(u32)x)`).

Probe `plans/independent-assessment/probes/WS3/310-increment-wrap.clo` (baseline `e24a8752`):

```
module Probe.Main;

import { println } from std.io;

func incr(x: i32): i32 {
    let y: i32 = x;
    y++;
    return y;
}

func decr(x: i32): i32 {
    let y: i32 = x;
    y--;
    return y;
}

func pre(x: i32): i32 {
    let y: i32 = x;
    return ++y;
}

func addAssign(x: i32): i32 {
    let y: i32 = x;
    y += 1;
    return y;
}

func mulAssign(x: i32): i32 {
    let y: i32 = x;
    y *= 2;
    return y;
}

func negate(x: i32): i32 {
    return -x;
}

func main(): i32 {
    let m: i32 = -2147483647 - 1;
    println($"{incr(2147483647)} {decr(m)} {pre(2147483647)} {addAssign(2147483647)} {mulAssign(1073741824)} {negate(m)}");
    return 0;
}
```

## Root cause
the value-position increment is emitted as the C operator on the signed type instead of the unsigned-based form used everywhere else.

Location: value-position increment lowering (emitted as `++(y)` / `(i)++`) in `compiler/src/codegen/expr.rs`, at e24a8752

## Recommended fix
emit value-position increments through the same unsigned form (`(tmp = y, y = (i32)((u32)y + 1u), tmp)` or a statement temporary); add a UBSan run golden.

## Acceptance criteria
- The behavior the specification requires: prints the wrapped values, no UBSan report (source: spec/02 §10 and spec/04 §5: every operator is defined and integer arithmetic wraps; spec/17 §14: "Signed overflow of +, -, *, unary - — wrapped through unsigned-based forms or -fwrapv")
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; `make asan` and `make ubsan`; `make corpus-amiga` (m68k GCC and vbcc legs)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, memory-safety auditor, emission-integrity auditor

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
ASM-WS1-002 (the same expressions are where evaluation order is pinned). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS3-codegen.md`.
