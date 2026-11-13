# Current State: Compiler Throughput (PLAN-38)

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## 1. Pipeline

`compiler/src/analysis.rs` runs the shared front end:

| Stage | Entry point | Output |
|---|---|---|
| Parse | `Parser::parse_program` | One AST per file |
| Stdlib injection | `inject_stdlib` | Embedded modules the program imports |
| Load | `load::load_with_entry_check` | Modules and declarations |
| Resolve | `resolve::resolve` | Name and import resolution |
| Sema | `sema::check` | Types, expression table, closure table |
| Ownership | `own::check` | Lowered IR, moves, drops, loans |
| Codegen | `codegen::emit` | One C99 translation unit |

`compiler/src/clorraine/build.rs` runs the same front end in process, writes
the translation unit, and then invokes the profile's C compiler. For the host
profile the C compiler dominates the wall time of `clorraine build`; the
front end dominates `clc` and large generated inputs.

## 2. There Was No Throughput Baseline

The repository measured generated-code size and runtime (`tools/perf/bench.py`,
`tools/perf/flag_audit.py`) but not compiler wall time or memory. The only
compiler-speed evidence was incidental timing during codegen closeouts. Issue
#13 needs a repeatable baseline and a hot-pass attribution.

## 3. The Quadratic Emitter Scans

Before this feature, the emitter found a function body by scanning the whole
ownership output twice:

- `compiler/src/codegen/function.rs`, `emit_function`:
  `self.ownership.functions.iter().find(|function| ...)`.
- `compiler/src/codegen.rs`, `param_c_names`:
  `self.ownership.functions.iter().find(|function| ...)`.

`emit_function` runs once per declaration and `param_c_names` runs once per
prototype and again per emitted body. With `F` lowered bodies the work is
`O(F²)` string comparisons. On hand-written programs `F` is small, so the cost
is hidden. On generated programs with thousands of functions it dominates.

## 4. Base Behaviour

At `9c44537` there is no throughput harness. The release `clc` on the pinned
container measured:

| Workload | Base wall time |
|---|---:|
| Spec corpus, 130 files, host | 323.6 ms total (sum of per-file medians) |
| Seven benchmarks, host | 3.5–10.8 ms each |
| Six Amiga examples, hosted-amiga | 8.3–51.0 ms each |
| Generated functions, 1,000 | 53.1 ms |
| Generated functions, 2,000 | 120.4 ms |
| Generated functions, 4,000 | 374.9 ms |
| Generated functions, 8,000 | 1,455.7 ms |
| Mixed generated, 4,000 functions + 4,000 calls | 819.3 ms |

The generated series grows faster than linearly: doubling from 4,000 to 8,000
functions nearly quadruples the time. That is the signature of the body scan.

## 5. Base Stage Attribution

A temporary instrumented build (not committed) timed each stage on the base:

| Stage | functions-8,000 | mixed-4,000 |
|---|---:|---:|
| Parse | 26.4 ms | 22.1 ms |
| Load | 11.2 ms | 14.9 ms |
| Resolve | 2.5 ms | 2.6 ms |
| Sema | 19.1 ms | 18.5 ms |
| Ownership | 110.2 ms | 357.6 ms |
| Codegen | 980.1 ms | 305.6 ms |

Codegen is 67% of the function-farm compile and the clear bottleneck. The
mixed shape also spends materially in ownership, but that stage is linear and
stays below the codegen cost.

## 6. Existing Test Seams

- `compiler/src/codegen.rs` has a small unit-test module for identifier
  helpers; the body index adds one focused case there.
- The specification suites (`codegen_run`, `corpus`, `staged`, feature suites)
  are the behavior oracle for emitted C.
- `tools/perf/tests/` is discovered by `make bench-test`, which is part of
  `make check`; the new harness tests live there.
- `make bench-compare` is the scored generated-code gate and must stay at zero
  deltas.
