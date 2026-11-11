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
| Worktree | clean |

## 2. Probe

The pre-plan probe is owned by `02-current-state.md` §Pre-Plan Probe. In one
line: `strings` 1,132,051 net instructions and 2,002 allocations, versus
1,489,187 and 6,003 on the shim-only base.

## 3. Red/Green

| Case | Base result | Head result | Note |
|---|---|---|---|
| ST-1 two integers | `3` | `1` — passed | the allocation win |
| ST-2 `String` | `2` | `1` — passed | borrow, no copy |
| ST-3 `str` | `2` | `1` — passed | borrow, no copy |
| ST-4 bool | `2` | `1` — passed | static text part |
| ST-5 float | `2` — passed | `2` — passed | unchanged `%g` temporary |
| ST-6 literal-only | `0` — passed | `0` — passed | view only |
| ST-7 explicit `text` | `1` — passed | `1` — passed | converter unchanged |
| Impl pins | pending | 5 passed | written after the implementation |
| No-change cases | 6 passed | 6 passed | independent, once, benchmark loop, sanitizers, owned loop, explicit conversions |

Red command: `cargo test -p clc --test
template_direct_formatting_spec_test -- --test-threads=1` against `00eb6bc`
reported 9 passed, 4 failed: allocint 3, allocstring 2, allocstr 2,
allocbool 2 (all expected 1). The float, literal-only, and explicit-text
deltas already match.

Green commands: `cargo test -p clc --test
template_direct_formatting_spec_test --test
template_direct_formatting_impl_test -- --test-threads=1` (13 + 5 passed), the
existing `string_integer_formatting` suites (9 + 5), and
`cargo test --workspace`.

Two implementation details were fixed during the green run: the part helpers
are `static inline` so an unused placeholder type does not fail
`-Werror=unused-function`, and the bool text arrays include their NUL so GCC
16's `-Wunterminated-string-initialization` accepts them.

One existing emission pin changed with the intended lowering:
`element_loop_integrity_impl_test` asserted `cl_string_copy(arr[0])` for a
`String` element placeholder. The test's intent — the element is not bound as
a second owner and the result is released once — is preserved; the positive
pin now reads `cl_str_part_string(arr[0])`. No spec-test assertion changed.

## 4. Final Measurement

| Benchmark | Base instructions | Head | Delta | Base `.text` | Head | Delta | Output |
|---|---:|---:|---:|---:|---:|---:|---|
| sieve | pending | pending | pending | pending | pending | pending | pending |
| matrix | pending | pending | pending | pending | pending | pending | pending |
| strings | pending | pending | pending | pending | pending | pending | pending |
| moves | pending | pending | pending | pending | pending | pending | pending |
| lookup | pending | pending | pending | pending | pending | pending | pending |

Allocations on `strings`: pending.

## 5. Sanitizers, Profiles, and Emission

| Leg | Result |
|---|---|
| New suites, plain `cargo test` | pending |
| `make asan` | pending |
| `make ubsan` | pending |
| `make bench-shape` | pending |
| Target boundary under `vamos` | pending |
| Base/head emitted-C diff | pending |
| `make check` | pending |
| `make verify` | pending |

## 6. Reviews

| Review | Verdict | Findings |
|---|---|---|
| Correctness | pending | pending |
| Memory safety | pending | pending |
| Emission integrity | pending | pending |

## 7. Closeout

- Issues #25 and #19: pending.
- Roadmap: pending.
- Residual: the float placeholder keeps one `%g` temporary; every other
  placeholder is allocation-free.
