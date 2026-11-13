# Design: Compiler Throughput (PLAN-38)

> **Document**: 03-design.md
> **Parent**: [Index](00-index.md)

## 1. Harness Shape

`tools/perf/compile_time.py` is a standard-library-only script beside
`bench.py` and `flag_audit.py`. It has two commands:

| Command | Behaviour |
|---|---|
| `run` | Measure one `clc` over every workload class and write one JSON document |
| `compare` | Require the same workload set and runs, then print the base/head table |

Workload classes and profiles:

| Class | Profile | Inputs |
|---|---|---|
| Corpus | `host` | Every `spec/tests/**/*.clo`, including compile-fail fixtures |
| Benchmark | `host` | Every `benchmarks/*/program.clo` |
| Real | `hosted-amiga` | Six large Amiga examples |
| Generated | `host` | Eight deterministic shapes written under `build/perf/compile-time/inputs` |

The generated shapes are:

| Name | Shape |
|---|---|
| `functions-1000/2000/4000/8000` | N small functions; `main` calls ten |
| `calls-4000` | One callee, 4,000 calls |
| `statements-4000` | One function, 4,000 assignments |
| `locals-4000` | One function, 4,000 locals and reads |
| `mixed-4000` | 4,000 functions, each called once from `main` |

## 2. Measurement Method

Each sample forks, redirects stdout/stderr to `/dev/null`, sets an alarm, and
execs `clc`. `wait4` returns the child's exit status and `ru_maxrss`. A record
keeps the median, minimum, maximum, peak RSS, and the set of exit codes.

The median is the decision number. The minimum is recorded because the harness
runs on a shared machine: an interleaved base/head check can use the sample
least disturbed by scheduling. Corpus time is the sum of per-file medians, not
a median of whole-corpus totals, because the corpus has no single invocation.
The per-file detail stays in the document.

`ru_maxrss` has a floor from the forked child's pre-exec pages. The document
records `rss_floor_kb` (a `/bin/true` sample), so a reader can distinguish a
small compile that sits at the floor from a large compile that exceeds it.

## 3. Body Index

`compiler/src/codegen.rs` builds one index in `Emitter::collect`:

```rust
bodies: BTreeMap<String, BTreeMap<String, &'a ir::Function>>
```

`body_index(functions)` groups by module, then by declaration name. The first
record wins when the same `(module, name)` appears twice, which is the
semantics of the old `Iterator::find`: a loop's speculative pass may lower one
body twice, and the emitter must keep the first record.

Both old scans become map lookups:

- `emit_function` resolves its body through `bodies[module][name]`.
- `param_c_names` resolves the body for a signature through the same index.

The complexity changes from `O(F²)` comparisons to `O(F log F)` index
construction plus `O(log F)` per lookup. The index is built once per compile.

## 4. Safety Argument

| Concern | Why it is safe |
|---|---|
| Emitted C | The same body is selected for every declaration; the independent emission audit reports zero byte differences |
| Duplicate lowerings | First-record-wins is preserved exactly, pinned by a unit test |
| Module identity | The outer key is the module, so same-named functions in two modules stay distinct |
| Missing body | The index returns `None`, the same fall-through the old scan took |
| Diagnostics and exit codes | The emitter's control flow is unchanged; only the lookup mechanism changed |
| Memory | The index stores references and cloned module/name keys; measured RSS is unchanged within noise |
| Scored runtime | No codegen decision changed, so `make bench-compare` stays at zero deltas |
| Spec | No `spec/` change; the frozen matrix and golden data are untouched |

## 5. Rejected Alternatives

| Alternative | Reason |
|---|---|
| Ship the harness without an optimization | The measurement found a real quadratic path on generated inputs; leaving it would fail the spirit of the issue |
| Optimize the ownership pass first | It is linear and below codegen on the function-farm shape; the 8,000-function stage table shows codegen at 980 ms vs ownership at 110 ms |
| Parallel compilation | A large new surface (threads, ordering, diagnostics, determinism) with no measured need at current input sizes |
| Incremental caching or a daemon | Changes the compiler's contract and introduces stale-state risk; far beyond the measured bottleneck |
| Replace `BTreeMap` with a hash map | No measured need; the map is built once and lookups are a small fraction of the compile |
| Change the release flags or LTO | Would trade generated-code quality and violate the issue's no-regression rule |
| Add a permanent `clc --timings` flag | The stage attribution is a one-off diagnostic; the committed harness needs only total time and memory |
