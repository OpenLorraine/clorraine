<!-- title: [correctness] Evaluate call arguments left to right -->
<!-- labels: assessment,kind:finding,severity:critical,priority:P0,domain:semantics,tier:core,tier:advanced,bug -->
<!-- parent: R2 -->

## Summary
Any call whose arguments include two or more calls with side effects (I/O reads, allocators, counters, iterators, hardware reads) runs them in the opposite order to the source on every shipped profile; the result is a silent miscompile. Example: `copy(readByte(f), readByte(f))` swaps the two bytes.

| Field | Value |
|---|---|
| Finding | ASM-WS1-002 · DEF-583 |
| Severity / Priority / Rank | Critical / P0 / 2 of 58 |
| Domain / Lens | semantics / all |
| Confidence / Verdict | verified / confirmed (Critical) (independent verification) |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Reproduction — probes `probes/WS1/021-evaluation-order.clo` and `066-evaluation-order-shapes.clo`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS1/probe.py plans/independent-assessment/probes/WS1 --clc build/tmp/assessment/baseline/clc --only 021 066"`. Expected (spec/04-expressions.md §3: "Function call — callee expression, then arguments left to right"; "Evaluation is strictly left-to-right and deterministic"): `pair(next(), next())` prints `1 2`; `three(next(), one(next()), next())` prints `123`. Observed on host, hosted-amiga, and hosted-amiga-13, debug and release builds: `2 1` and `321`. The emitted C is `cl_Probe_Main_pair(cl_Probe_Main_next(), cl_Probe_Main_next());`, whose argument order C leaves unspecified (C99 6.5.2.2p10); GCC evaluates right to left on both targets. Struct-literal fields, array-literal elements, template pieces, operator chains, and assignment target-then-value printed the spec order. The binary operator `next() - next()` printed `-1` (left first), but it is emitted as `(u32)cl_Probe_Main_next() - (u32)cl_Probe_Main_next()`, whose operand order is also unspecified in C (C99 6.5p3), so its correct result rests on GCC's current choice.

Probe `plans/independent-assessment/probes/WS1/021-evaluation-order.clo` (baseline `e24a8752`):

```
module Probe.Main;

import { println } from std.io;

let counter: i32 = 0;

func next(): i32 {
    counter = counter + 1;
    return counter;
}

func pair(a: i32, b: i32): void {
    println($"{a} {b}");
}

func main(): i32 {
    let i: i32 = 0;
    let xs: i32[4] = [10, 20, 30, 40];
    let v: i32 = xs[i++];
    println($"{v} {i}");
    let w: i32 = ++i;
    println($"{w} {i}");
    pair(next(), next());
    counter = 0;
    let d: i32 = next() - next();
    println($"{d}");
    counter = 0;
    let s = $"{next()} {next()} {next()}";
    println(s);
    return 0;
}
```

## Root cause
argument lowering pins an argument into a statement temporary only when an argument contains a syntactic update (`++`, `--`, assignment; `contains_update`) or when a later argument needs a statement fill; an argument that is a call with side effects but no syntactic update is passed directly as a C call argument, so C's unspecified argument order decides. Binary operands follow the same direct-emission path.

Location: `compiler/src/own/call.rs:409-428`, `compiler/src/own/call.rs:565-575`, `compiler/src/own/lower.rs:7286-7316` at e24a8752

## Recommended fix
when two or more argument (or operand) positions have effects (`expression_has_effects`, which already counts calls), bind every effectful position except the last into a statement temporary in source order, as the template and aggregate-literal paths already do; add a run golden with two effectful arguments and two effectful operands.

## Acceptance criteria
- The behavior the specification requires: `pair(next(), next())` prints `1 2`; `three(next(), one(next()), next())` prints `123` (source: spec/04-expressions.md §3: "Function call — callee expression, then arguments left to right"; "Evaluation is strictly left-to-right and deterministic")
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; `make asan` and `make ubsan`; `make corpus-amiga` (m68k GCC and vbcc legs)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, memory-safety auditor, emission-integrity auditor, semantics reviewer

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
WS3-03, WS3-08; the same argument-order class was fixed for narrower shapes in DEF-184 (update operands), DEF-278 (owning argument before a later update), DEF-438 (take-deref carrier hoist), and DEF-552 (later statement-store literal) — each fix pinned only the shape it found, so plain effectful calls were never pinned (input to WS8-07/WS8-08). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS1-semantic-core.md`.
