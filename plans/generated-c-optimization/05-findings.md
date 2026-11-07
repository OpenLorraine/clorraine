# Findings and Evidence: Generated-C Optimization

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

## Purpose

This is the evidence ledger for the plan. It owns the committed base/head
measurements, the red/green pin record, the accepted limits, and the rejected
candidates. The exploratory study stays in
[Current State](02-current-state.md); the requirements and design stay in
[Requirements](01-requirements.md) and
[Generated-C Changes](03-02-generated-c-changes.md).

## Committed Baseline

Filled by the execution task after `make bench-base` and before the first
optimization. The base compiler is the recorded commit, built by
`docker/scripts/bench-base.sh`; the retained binary is
`build/perf/base-clc`.

| Benchmark | `instructions_net` | m68k `.text` `-O2` | C bytes | Host `-O2` median |
|---|---:|---:|---:|---:|
| calibration | 3,373 | — | — | 0.55 ms |
| sieve | 1,520,792 | 101,396 | 489,983 | 0.90 ms |
| matrix | 22,059 | 80,732 | 313,475 | 0.93 ms |
| strings | 2,972,051 | 77,540 | 312,281 | 2.36 ms |
| moves | 45,205 | 76,880 | 312,756 | 2.16 ms |

**Base commit:** `0ac87d3e03ece01a0df474f1cd43826df90c0f41`

**Toolchain identity:** host `cc` Debian 12.2.0; `m68k-amigaos-gcc`
16.2.0b; `amitools 0.8.1.dev274+gd0b9727da`; Python 3.11.2.

**Commands:** `make bench-base COMMIT=0ac87d3`, `make bench`,
`make bench-compare`. The retained base compiler is `build/perf/base-clc`.

## Head and Delta

Filled after each optimization phase and at the final acceptance.

| Benchmark | Base instructions | Head instructions | Delta | Base `.text` | Head `.text` | Delta | C bytes delta | Output |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| sieve | 1,520,792 | 1,040,581 | −31.6% | 101,396 | 76,664 | −24.4% | −36.6% | equal |
| matrix | 22,059 | 11,154 | −49.4% | 80,732 | 77,992 | −3.4% | −0.3% | equal |
| strings | 2,972,051 | 2,972,051 | 0.0% | 77,540 | 77,492 | −0.1% | −0.01% | equal |
| moves | 45,205 | 45,205 | 0.0% | 76,880 | 76,832 | −0.1% | −0.01% | equal |

**Final verdict:** `make bench-compare` passed. The R4.8 sieve target
(−20% instructions) and the R5.7 targets (−10% `.text`, −25% C bytes) are
met; no benchmark regressed beyond 1%. The matrix instruction drop is the
same const-fold mechanism: its loop bound `N` is now a literal.

## Emission Audit

The base compiler (`build/perf/base-clc`) and the head compiler emitted the
same 40 sources: the four corpus programs, the four benchmark programs plus
calibration, and every `examples/amiga/*.clo` program (three corpus programs
needed the hosted-amiga profile). Every changed unit falls into one of four
classes; no diff is unexplained:

| Class | Example | R |
|---|---|---|
| Scalar module const read replaced by a literal | `cl_std_io_MODE_NEWFILE` → `1006`, `cl_std_amiga_custom_COP_DMA` → `128u`, Boing `WIDTH/HEIGHT/TILE` → literals | R4 |
| Large one-extent scalar fill replaced by `memset` or a loop | sprite-demo `pixels` fill, imageviewer/paddle/boing buffers | R5 |
| Dead `< 0` half removed for a non-negative slice/str constant | `(0 < 0)` in `std.mem` and `buffer-container` | R6 |
| Constant divisor guard removed | `2 == 0`, `3 == 0`, and folded `TILE` guards | R7 |

The non-constant-divisor guard remains (for example `std.math`'s
`x == 0 || (value == MIN && x == -1)`). The full check, ASan, corpus, and
differential legs build and run the same units; the emission diff is the
attribution record, not a substitute for those gates.

## Red/Green Pin Record

The base run of `generated_c_optimization_impl_test.rs` records:

| Status | Pins |
|---|---|
| Red on base | IP-1, IP-2, IP-3, IP-5, IP-6, IP-7, IP-9, IP-10, IP-14, IP-15, IP-17, IP-20 |
| Green on base | IP-4, IP-8, IP-11, IP-12, IP-13, IP-16, IP-18, IP-19 |

**Base evidence:** the base compiler emitted 8,192 per-element stores for the
sieve fill, read `cl_..._N` in the loop tests, wrote `(1 < 0)` for a slice
constant index, and wrote `2 == 0` in the `% 2` guard. The base compiler was
not asked to run the new impl suite (its tests use `CARGO_BIN_EXE_clc`), so the
red set is established from those emitted-C probes plus the base/head
instruction and size deltas.

**After the change:** `generated_c_optimization_impl_test` has 19 passing
tests covering the 20 pins (the 15/16 threshold is one test with two emits).

## Accepted Limits

| Limit | Reason | Requirement |
|---|---|---|
| Module-level initializer reads do not fold | Separate lowering path; no measured need | R4.7 |
| Float arithmetic and non-finite float consts do not fold | The folder evaluates floats in `f64`; the measured need is integer constants | R4.1, R4.5 |
| Nested-row, struct-literal, array-copy, and deferred expression fills keep their spelling | No measured need; the measured fill is a one-extent scalar statement fill | R5.1 |
| The upper index bound is not eliminated by the compiler front end | Manual elimination measured slower on m68k; the C compiler removes it after const folding | R6.4 |
| Host time is advisory | 6–10% run-to-run spread; the target metric is the m68k instruction count | R1.5, AR-2 |
| The harness has no FS-UAE cycle counter | FS-UAE 3.1.66 has no stable headless counter; `vamos` is deterministic | AR-16 |
| The harness does not report per-struct sizes | The linked section sizes are the accepted code-size metric; the ABI is frozen | AR-22 |

## Rejected Candidates

| Candidate | Evidence | Decision |
|---|---|---|
| Manual upper-bound elimination | m68k instructions 2,165,514 → 2,436,214 after const folding; container host `-O2` 31.0 → 35.0 ms median | Rejected |
| `static const` / read-only storage | Separate storage/init change; would break accepted `unsafe &N` mutation | Rejected |
| `if (sizeof(...))` marker removal | Compile-time no-op; no measured cost | Rejected |
| Trailing `continue` removal | Removed by the C compiler at `-O2`; no measured delta | Rejected |
| `bool` `memset` | `bool` is 32-bit on the target | Rejected |
| `memset` for the whole fill when a prefix exists | Must start at `&target[prefix]` | Rejected (design corrected) |
| Binary constant divisors in the guard fold | A wrapping binary divisor can lose the `MIN / -1` trap | Rejected |

## Review Rulings

| Source | Finding | Ruling | Evidence |
|---|---|---|---|
| Requirements preflight | PF-REQ-001–027 | Resolved; see [Requirements Preflight](00-preflight-report-requirements.md) | All critical/major closed |
| Plan preflight | PF-PLAN-001–031 | Resolved; see [Plan Preflight](00-preflight-report.md) | All critical/major closed |
| Correctness review | pending | — | — |
| Memory-safety review | pending | — | — |
| Emission-integrity review | pending | — | — |
| Performance review | pending | — | — |
| Semantics review | pending | — | — |
