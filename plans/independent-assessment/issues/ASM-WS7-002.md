<!-- title: [performance] Make compile time near-linear for call-heavy and generic code -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:perf,tier:core,performance -->
<!-- parent: R14 -->

## Summary
Compile time grows about quadratically on average (above quadratic on the top doubling) for stdlib-call-heavy and generic code: 8,000 such functions take 45.5 s (author run, `findings/records/WS7-runs.log`; 92.7 s in the loaded verifier run, `verification/verdicts-9.2.5.md`) against 0.4 s for 8,000 one-line functions. Name resolution in the ownership pass, instance de-duplication, and codegen's `resolve_function` fallback scan linear lists inside per-call and per-instance loops. Today's examples compile in milliseconds; large programs and the language server degrade first.

| Field | Value |
|---|---|
| Finding | ASM-WS7-002 · DEF-610 |
| Severity / Priority / Rank | Major / P1 / 29 of 58 |
| Domain / Lens | perf / host (`clc` itself) |
| Confidence / Verdict | verified / confirmed (Major; measurement verified, attribution probable and reproduced) (independent verification) |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Measurement — probe `probes/WS7/710-compile-scaling.py`, command `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS7/710-compile-scaling.py build/tmp/assessment/baseline/clc build/tmp/assessment/WS7/m710"` (pinned release `clc`, host profile, 3-run medians): 8,000 one-line functions 441 ms; 8,000 functions with owned `String`s and stdlib calls 45,475 ms (time exponent 2.37, 103× the one-line series at the same function count, 152× the Phase 0 `functions-8000` 299 ms); 8,000 `Array<S_i>` instantiations 77,566 ms (exponent 2.45). Attribution — probe `750-sample.sh` (gdb stack samples once a second; host `perf` is locked by `perf_event_paranoid` 4): 7 of 8 `ownership-4000` samples and 4 of 10 `generic-4000` samples are in `resolved_callee` or `function_identity`; 2 `generic` samples are in `Monomorphization::record`/`record_struct` comparisons (`records/WS7-runs.log`).

Phase 9 notes: average exponent over 1,000→8,000 is about quadratic (ownership 2.22, generic 1.85), above quadratic on the top doubling; add `compiler/src/codegen.rs:1009-1020` (`resolve_function` fallback scan) to the location — the residual generic-instance lookup `compiler-throughput` left open.

Probe `plans/independent-assessment/probes/WS7/710-compile-scaling.py` (baseline `e24a8752`):

```python
#!/usr/bin/env python3
"""WS7-06/07: compile time and peak RSS scaling of the pinned release `clc`.

Generators the `compile_time.py` series lacks, each producing a valid program whose
size is the parameter N (functions, instantiations, or closures; locals for the
`locals` series that WS1 handed over):

  ownership  N functions, each with three owned `String` locals, one conditional move
             into a `take` parameter, a struct with two `String` fields, and drops on
             two exits
  generic    N struct types, each used through `Array<S_i>` (push, len), so N
             instantiations of the Array surface and its drop glue
  closure    N functions, each passing a capturing lambda to a callable-bound generic
  locals     one function with N `i32` locals (compile_time.local_farm)
  functions  N one-line functions (compile_time.function_farm; the Phase 0 baseline)

For every series and N in 1000, 2000, 4000, 8000 the script compiles to C on the host
profile with `tools/perf/compile_time.measure` (median wall and CPU time of 3 runs,
peak RSS), requires exit 0 and a non-empty C file, and reports the scaling exponent
log2(t(8000) / t(4000)) for wall time and RSS.

Usage (SDK container, repo root): 710-compile-scaling.py <clc> <out-dir> [--seeded]
--seeded adds a series whose program has a type error; the run must report it as a
failure (the comparator rejects an output that differs from "compiles").
"""
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "tools" / "perf"))
import compile_time  # noqa: E402

SIZES = (1000, 2000, 4000, 8000)
RUNS = 3


def ownership(n: int) -> str:
    lines = ["module bench.Ownership;", "", "import { stringOf, bytes } from std.text;", "",
             "struct Pair {", "    a: String;", "    b: String;", "}", "",
             "func consume(take s: String): usize {", "    return len(bytes(s));", "}", ""]
    for i in range(n):
        lines += [f"func work{i}(flag: bool): usize {{",
                  f"    let s: String = stringOf(\"a{i % 97}\");",
                  "    let t: String = stringOf(\"b\");",
                  "    let p: Pair = Pair { a: stringOf(\"x\"), b: stringOf(\"y\") };",
                  "    if (flag) {",
                  "        return consume(s) + len(bytes(p.a));",
                  "    }",
                  "    return len(bytes(t)) + len(bytes(p.b));",
                  "}", ""]
    lines += ["func main(): i32 {", "    let total: usize = 0;"]
    lines += [f"    total = total + work{i}(true);" for i in range(min(n, 10))]
    lines += ["    return i32(total);", "}"]
    return "\n".join(lines) + "\n"


def generic(n: int) -> str:
    lines = ["module bench.Generic;", "", "import { Array, arrayNew } from std.mem;", ""]
    for i in range(n):
        lines += [f"struct S{i} {{", "    v: i32;", "}", "",
                  f"func use{i}(): usize {{",
                  f"    let xs: Array<S{i}> = arrayNew<S{i}>();",
                  f"    xs.push(S{i} {{ v: {i % 97} }});",
                  "    return xs.len();", "}", ""]
    lines += ["func main(): i32 {", "    let total: usize = 0;"]
    lines += [f"    total = total + use{i}();" for i in range(min(n, 10))]
    lines += ["    return i32(total);", "}"]
    return "\n".join(lines) + "\n"


def closure(n: int) -> str:
    lines = ["module bench.Closure;", "",
             "func apply<F: func(i32): i32>(f: F, x: i32): i32 {", "    return f(x);", "}", ""]
    for i in range(n):
        lines += [f"func call{i}(): i32 {{", f"    let k: i32 = {i % 97};",
                  "    return apply([k] (x: i32) => x + k, 1);", "}", ""]
    lines += ["func main(): i32 {", "    let total: i32 = 0;"]
    lines += [f"    total = total + call{i}();" for i in range(min(n, 10))]
    lines += ["    return total;", "}"]
… (46 more lines in the file)
```

## Root cause
name resolution in the ownership pass and instance deduplication use linear scans inside per-call and per-instance loops, so cost is the product of the call count and the module size.

Location: `compiler/src/own/call.rs:1261-1291` (`resolved_callee`: `declared_function` and `declared_ffi`, `:1453-1469`, scan the user module's `items` linearly before imports are consulted, and the resolver runs several times per call from `call_may_write_globals`, `infer_call_types`, `lower_intrinsic_call`, `call_view_source`, `check_dot_receiver`); `compiler/src/ir/mono.rs:690-711` (`Monomorphization::record` and `record_struct` compare each new instance against every recorded one), at e24a8752

## Recommended fix
build a per-module name index once (as `compiler-throughput` did for codegen bodies) and look callees up in it; key instances in a map (`BTreeMap`/`HashMap` by the instance key) instead of `iter().any`; add the 710 series to `compile_time.py` so regressions show.

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
`plans/compiler-throughput/05-findings.md` §2 (codegen fixed; "the ownership pass … close to linear" on the mixed shape; "residual generic-instance … lookups still scan the function table"); WS1-10 robustness (locals). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS7-performance.md`.
