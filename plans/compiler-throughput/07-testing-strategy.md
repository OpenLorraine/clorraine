# Testing Strategy: Compiler Throughput (PLAN-38)

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## 1. Test Layers

| Layer | File | Oracle |
|---|---|---|
| Harness generation and records | `tools/perf/tests/test_compile_time.py` | Deterministic sources, medians, bounds, exit codes, document validation, comparison limits |
| Emitter index | `compiler/src/codegen.rs` unit module | Module keying and first-record-wins semantics |
| Compiler behaviour | existing `cargo test --workspace` suites | Emitted C, diagnostics, ownership, runtime |
| Emission identity | base/head `clc` audit | Zero byte differences over every successful workload |
| Scored generated code | `make bench-compare` | Zero instruction and `.text` deltas |
| Full gate | `make check`, sanitizers, `make verify` | The change is safe in the shipped pipeline |

The feature does not add a language rule, so there is no new `*_spec_test.rs`
oracle. The existing specification suites are the behavior oracle; the new
unit test pins the index seam.

## 2. Harness Cases

| # | Case | Expected | Traces |
|---|---|---|---|
| HT-1 | `function_farm(250)` | 250 `func work` declarations and one `main` | R5 |
| HT-2 | `call_farm(40)` | 40 calls plus the declaration | R5 |
| HT-3 | Statement and local farms at 40 | 40 assignments and 40 locals | R5 |
| HT-4 | `generated_sources()` twice | Identical text and the expected eight names | R5 |
| HT-5 | Mixed farm at 30 | 30 functions, each called once | R5 |
| HT-6 | Generated names | No `func f32`/`func f64` keyword collision | R5 |
| HT-7 | `write_generated_inputs` | Eight `.clo` files with matching stems | R5 |
| HT-8 | Summary of `[300, 100, 200]` | Median 200, bounds 100/300, exit codes sorted | R2 |
| HT-9 | Empty sample set | Rejected | R2 |
| HT-10 | Percent change | Signed and zero-safe | R3 |
| HT-11 | Successful child measurement | Exit `[0]`, positive time and RSS | R2 |
| HT-12 | Failing child measurement | Exit `[3]` | R2 |
| HT-13 | Compare two complete documents | Rows for every workload, signed deltas and RSS | R3 |
| HT-14 | Compare on the minimum metric | Uses `min_ns` | R3 |
| HT-15 | Unknown metric / different workload sets | Rejected | R3 |
| HT-16 | Compare with a regression limit | Fails past the limit, passes below it | R3 |
| HT-17 | Complete document load | Accepted | R3 |
| HT-18 | Missing field / unsupported schema | Rejected | R3 |

## 3. Emitter Cases

| # | Case | Expected | Traces |
|---|---|---|---|
| IT-1 | One body in module `a` | Index returns it under `a` | R7 |
| IT-2 | Two records for `a/same` | First record wins, one entry | R7 |
| IT-3 | Same name in modules `a` and `b` | Two distinct entries | R7 |
| IT-4 | Base/head emission over corpus, benchmarks, examples, generated | Zero differences | R8 |

## 4. Red/Green Order

1. Write the harness tests and the body-index unit test.
2. Run them on the base: the harness module does not exist (import failure),
   and `body_index` is missing (compile failure).
3. Add the harness, the make target, and the index.
4. Re-run the focused tests until green.
5. Run the full workspace suite, then the gates.

## 5. Measurement Verification

| Check | Method |
|---|---|
| Baseline table | `compile_time.py run` with the base `clc` at `9c44537` |
| Head table | `make bench-compile` with the head `clc` |
| Hot passes | Temporary stage instrumentation on base and head, recorded in findings |
| Machine noise | Interleaved base/head runs for corpus, benchmarks, examples, generated shapes, and the driver |
| No emission change | Compare exit codes, diagnostics, and emitted C for every workload |
| No scored change | `make bench-compare` with base `9c44537` and head |

## 6. Gate Coverage

| Gate | What it proves |
|---|---|
| `make check` | Workspace suites, spec check, harness tests, fuzz smoke |
| `make asan` / `make ubsan` | No memory or undefined-behaviour defect |
| `make bench-shape` | Emitted C still builds on m68k GCC, vbcc, and freestanding |
| `make bench-compare` | The scored `-O2` rows are unchanged |
| `make corpus-amiga` / `make differential` | m68k outputs are unchanged |
| `make verify` | The full acceptance gate |
