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
template_direct_formatting_impl_test -- --test-threads=1` (15 + 5 passed), the
existing `string_integer_formatting` suites (9 + 5), and
`cargo test --workspace`.

The alias regression is committed as
`an_alias_sibling_does_not_free_a_placeholder_source`. It fails on the unsafe
head `ef03479` with `z\0\0\0\0\01` against `abcdef1` and passes on the base
and on the fixed head. ST-10 now returns the counter and asserts `1-2 2`; the
spec's left-to-right guarantee is the oracle.

Two implementation details from the green run: the part helpers are
`static inline` so an unused placeholder type does not fail
`-Werror=unused-function`, and the bool text arrays include their NUL so GCC
16's `-Wunterminated-string-initialization` accepts them. No pre-existing spec test changed; the new feature suite's ST-2/ST-3 pin the
safe two-allocation copy contract, and ST-10 pins `1-2 2`. The
`element_loop_integrity_impl_test` file is byte-identical to the base again.

## 4. Final Measurement

Head `b681fde` (`dirty=false`), base `00eb6bc` (`dirty=false`).

| Benchmark | Base instructions | Head | Delta | Base `.text` | Head | Delta | Output |
|---|---:|---:|---:|---:|---:|---:|---|
| sieve | 1,040,152 | 1,040,081 | −0.01% | 23,720 | 23,816 | +0.4% | equal |
| matrix | 9,431 | 9,361 | −0.7% | 23,496 | 23,592 | +0.4% | equal |
| strings | 1,489,187 | 1,221,232 | **−18.0%** | 24,020 | 23,972 | −0.2% | equal |
| moves | 44,680 | 44,630 | −0.1% | 23,520 | 23,684 | +0.7% | equal |
| lookup | 8,849 | 8,779 | −0.8% | 39,792 | 39,884 | +0.2% | equal |

`make bench-compare`: passed; every output hash equal; every metric inside the
1% guard (the largest `.text` growth is +0.5%).

Allocations on `strings`: **2,003** (2,000 results in the loop + `toHex`, its
String copy, and the result outside), down from 6,003. The safe copy of a
`String`/`str` placeholder is 1 extra allocation for those templates; the
integer/bool hot path keeps the one-result contract. The final instruction
count is 89,181 above the unsafe-borrow probe (1,221,232 vs 1,132,051),
reflecting the restored copies and the corrected kind checks.

## 5. Sanitizers, Profiles, and Emission

| Leg | Result |
|---|---|
| New suites, plain `cargo test` | spec 15/15, impl 5/5 |
| Existing `string_integer_formatting` suites | 9/9 and 5/5 |
| `cargo test --workspace` | passed |
| `make asan` | 32 suites passed on `b681fde`; focused suite 15/15 after the alias test |
| `make ubsan` | 6 suites passed on `b681fde`; focused suite 15/15; the null-view case is UBSan-clean |
| `make bench-shape` | GCC, nix13, vbcc, freestanding all warning-free after the fix |
| Target boundary under `vamos` | equal to host, including bool/float/String/str and the null view |
| Base/head emitted-C diff | 5 hunks per profile: the append guard, the part/helpers, the builder, and two template calls; no program-body change |
| `make check` | passed |
| `make verify` | passed, `verify: all legs passed` |

## 6. Reviews

| Review | Verdict | Findings |
|---|---|---|
| Correctness | first round: 🟠 RV-001, 🟡 RV-002/003/004; re-review: ⚠️ missing alias regression + doc sync | all fixed: alias test added (red on `ef03479`, green on head), test docs and plan records synced, ST-10 strengthened |
| Memory safety | no memory-safety finding in the diff; pre-existing nullable String, `str` double-eval, take-sibling, and view-over-move defects | first audit: no diff finding; re-audit: no diff finding; pre-existing cases filed as #26 |
| Emission integrity | 🔴 EI-001, 🟠 EI-002 | fixed; re-audit: no emission finding; the only diff is the intended shim/template block |

No open critical/major finding remains. The `make check`/`make verify` and
32/6 sanitizer legs are green on the code commit `b681fde`; the alias test and
the plan-record sync are test/documentation changes on top.

## 7. Closeout

- Issues #25 and #19: closing with this evidence.
- Pre-existing template defects: #26 (nullable String, `str` double eval,
  take sibling, view over move).
- Roadmap: Done at 26/26.
- Residual: `String`/`str` placeholders copy at part construction for safety;
  the integer and bool placeholders allocate only the result. The float
  placeholder keeps its `%g` temporary.
- Pre-existing template defects found by the memory audit: file a separate
  issue with the three reproductions.
