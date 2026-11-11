# Findings: Direct Template Formatting

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

## 1. Base Identity

| Item | Value |
|---|---|
| Base commit | `00eb6bc` (shim-only formatter) |
| Compiler | `clc 1.1.2` |
| Base metrics | `strings` 1,489,187 net, 24,020 `.text`, 6,003 allocations |
| Base compiler SHA-256 | `12fc41714314…` (full value in `build/perf/base.json`) |
| Base/pre-change compare | passed, every metric `+0.0%`; harness `dirty=false` |

## 2. Probe

The pre-plan probe is owned by `02-current-state.md` §Pre-Plan Probe. In one
line: the unsafe-borrow probe measured `strings` 1,132,051 net instructions
and 2,002 allocations. The safe final shape keeps the integer/bool win and
copies `String`/`str` for evaluation-order safety (AR-13), so the final count
is 2,003 and the instruction count is measured in §4.

## 3. Red/Green

| Case | Base result | Head result | Note |
|---|---|---|---|
| ST-1 two integers | `3` | `1` | the integer allocation win |
| ST-2 `String` | `2` | `2` | copy plus result; no change |
| ST-3 `str` | `2` | `2` | copy plus result; no change |
| ST-4 bool | `2` | `1` | static text part |
| ST-5 float | `2` | `2` | unchanged `%g` temporary |
| ST-6 literal-only | `0` | `0` | view only |
| ST-7 explicit `text` | `1` | `1` | converter unchanged |
| ST-8 null view | `[]` | `[]` | the zero-view regression is fixed |
| ST-10 call-once | `7-7 2` | `7-7 2` | each call once; order not pinned |
| Impl pins | n/a | 5 passed | written after the implementation |
| Other no-change cases | 6 passed | 6 passed | independent, benchmark loop, sanitizers, owned loop, explicit conversions |

Red command: `cargo test -p clc --test
template_direct_formatting_spec_test -- --test-threads=1` against `00eb6bc`
reported the ST-1 delta `3` and the ST-4 delta `2` (expected `1`), plus the
expected no-change passes. The unsafe-borrow head (`ef03479`) was then
rejected by the emission review:

| Review finding | Symptom | Fix |
|---|---|---|
| EI-001 (CRITICAL) | `$"{s}{reset(s)}"` freed the source before the builder read the borrowed part (`z\0…`, ASan heap-use-after-free) | copy `String`/`str` at part construction; the alias probe matches the base `abcdef1` under ASan |
| EI-002 / RV-001 (MAJOR) | a zero/NULL `str` view (`ptr == NULL, len == 0`) was read as scalar kind 0 and printed `0` | scalar kinds are 2/3; a `ptr == NULL, len == 0` piece is the empty byte piece; the null-view case prints `[]` |
| RV-002 (MINOR) | emitted designated initializers were forbidden by `spec/17-codegen.md` | the codegen emits `cl_str_part_i32/u32/bytes/owned/bool/f64` helpers; no program-body designator |
| RV-003 (MINOR) | the ST-10 table said `1-2` while the test pinned only once | the table now matches `7-7 2`; left-to-right order is recorded as not guaranteed |
| RV-004 (MINOR) | the committed plan record was stale | this document and the execution plan are committed with the phase |

Green commands: `cargo test -p clc --test
template_direct_formatting_spec_test --test
template_direct_formatting_impl_test -- --test-threads=1` (14 + 5 passed), the
existing `string_integer_formatting` suites (9 + 5), and
`cargo test --workspace`.

Two implementation details from the green run: the part helpers are
`static inline` so an unused placeholder type does not fail
`-Werror=unused-function`, and the bool text arrays include their NUL so GCC
16's `-Wunterminated-string-initialization` accepts them. No existing spec
assertion changed; `element_loop_integrity_impl_test` is back to its original
`cl_string_copy(arr[0])` pin because `String` placeholders copy again.

## 4. Final Measurement

Head `TBD` (`dirty=false`), base `00eb6bc` (`dirty=false`).

| Benchmark | Base instructions | Head | Delta | Base `.text` | Head | Delta | Output |
|---|---:|---:|---:|---:|---:|---:|---|
| sieve | 1,040,152 | pending | pending | 23,720 | pending | pending | pending |
| matrix | 9,431 | pending | pending | 23,496 | pending | pending | pending |
| strings | 1,489,187 | pending | pending | 24,020 | pending | pending | pending |
| moves | 44,680 | pending | pending | 23,520 | pending | pending | pending |
| lookup | 8,849 | pending | pending | 39,792 | pending | pending | pending |

Allocations on `strings`: pending (expected 2,003).

## 5. Sanitizers, Profiles, and Emission

| Leg | Result |
|---|---|
| New suites, plain `cargo test` | spec 14/14, impl 5/5 |
| Existing `string_integer_formatting` suites | 9/9 and 5/5 |
| `cargo test --workspace` | passed |
| `make asan` | pending (re-run after the fix) |
| `make ubsan` | pending (re-run after the fix) |
| `make bench-shape` | pending (re-run after the fix) |
| Target boundary under `vamos` | pending (re-run after the fix) |
| Base/head emitted-C diff | pending (re-run after the fix) |
| `make check` | pending |
| `make verify` | pending |

## 6. Reviews

| Review | Verdict | Findings |
|---|---|---|
| Correctness | first round: 🟠 RV-001, 🟡 RV-002/003/004 | fixed; re-review pending |
| Memory safety | no memory-safety finding in the diff; three pre-existing template crashes (nullable String, take sibling, view-live-over-move) | record and file separately |
| Emission integrity | 🔴 EI-001, 🟠 EI-002 | fixed; re-review pending |

## 7. Closeout

- Issues #25 and #19: pending.
- Roadmap: pending.
- Residual: `String`/`str` placeholders copy at part construction for safety;
  the integer and bool placeholders allocate only the result. The float
  placeholder keeps its `%g` temporary.
- Pre-existing template defects found by the memory audit: file a separate
  issue with the three reproductions.
