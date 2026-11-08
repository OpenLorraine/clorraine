# Findings and Evidence: Nested Fill Compaction (PLAN-22)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

## Purpose

This ledger owns the committed base/head measurements, the red/green pin
record, the emission audit, the accepted limits, the rejected candidates, the
review verdicts, and the final acceptance table. The exploratory study stays in
[Current State](02-current-state.md).

## Committed Baseline

Filled after `make bench-base COMMIT=b68dae5` and `make bench` on the clean
clone. The base compiler source is the recorded commit and is retained as
`build/perf/base-clc`. The head is built from the plan-docs commit, which
changes no compiler file. The two SHA-256 values are recorded; they are not
required to be equal because the base builds in a nested worktree and a debug
binary can embed a different path.

| Benchmark | `instructions_net` | m68k `.text` `-O2` | C bytes | Host `-O2` median |
|---|---:|---:|---:|---:|
| sieve | pending | pending | pending | pending |
| matrix | pending | pending | pending | pending |
| strings | pending | pending | pending | pending |
| moves | pending | pending | pending | pending |

Calibration is a separate line, not a scored benchmark: it records only
`calibration.instructions` (raw) and `calibration.host_release_ns` (best).
It has no `.text` row, no C-byte row, and no median in the JSON.

**Base compiler source commit:** `b68dae5` (40-hex recorded in `base.json`).

**Head source commit:** the plan-docs commit; no compiler file differs from
`b68dae5`.

**Base / head compiler SHA-256:** pending (recorded, not required to be equal).

**Dirty state:** must be `false` in both documents.

**Commands:** `make bench-base COMMIT=b68dae5`, `make bench`.

## Exploratory Probe

The probe below was run on the current compiler before the plan. It patched
only the emitted C of matrix; the benchmark source and expected output were
unchanged. It selected the shape; the committed baseline above is the gate.

| Metric | Current emission | One `memset` | Delta |
|---|---:|---:|---:|
| matrix trace instructions (raw) | 14,331 | 13,280 | -1,051 |
| matrix net instructions | 10,959 | 9,908 | -9.6% |
| matrix m68k `-O2` `.text` | 80,712 | 76,676 | -5.0% |
| matrix C bytes (exploratory) | 315,167 | 312,510 | -0.8% |
| output | `matrix: 523776` | `matrix: 523776` | equal |

The absolute C-byte values are exploratory. A later reproduction measured
313,637 / 310,980, the same 2,657-byte delta. Phase 1 re-runs the probe with
`build/perf/base-clc` and records the committed numbers; the gate uses
instructions and `.text`.

## Head and Delta

Filled after the codegen commit with `make bench` and
`make bench-compare REQUIRE="<final requirements>"` (the Phase 1 ruling sets
the final values; the initial 5/3 is provisional).

| Benchmark | Base instructions | Head instructions | Delta | Base `.text` | Head `.text` | Delta | C bytes delta | Output |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| sieve | pending | pending | pending | pending | pending | pending | pending | equal |
| matrix | pending | pending | pending | pending | pending | pending | pending | equal |
| strings | pending | pending | pending | pending | pending | pending | pending | equal |
| moves | pending | pending | pending | pending | pending | pending | pending | equal |

**Head identity:** pending.

**Verdict:** pending.

If any codegen change lands after the head measurement (a review fix or a
sanitizer fix), the head document is stale. Re-run `make bench` and the
emission audit on the new clean commit and replace this table; a stale head
document blocks the closeout.

## Red/Green Pin Record

Filled before and after the codegen change.

| Status | Pins |
|---|---|
| Red on base (`b68dae5`) | pending |
| Green on head | pending |
| Behaviour oracle on base | pending |

The red run is the unmodified compiler with the impl suite present. The
behaviour suite must pass on both.

## Emission Audit

The base `clc` (`build/perf/base-clc`) and the head `clc` emit the corpus,
benchmark, and Amiga example sources. The audit is filled after the codegen
commit.

| Unit class | Expected diff | Observed |
|---|---|---|
| Nested constant-zero fill | one `memset`, no row temporary | pending |
| Everything else | byte-identical | pending |

## Accepted Limits

| Limit | Reason | Requirement |
|---|---|---|
| Float leaf types do not compact | Float constant folding has separate accepted limits; the measured shape is integer | R3.4 |
| One-extent fills keep their current rule | PLAN-14 measured the byte/loop thresholds; this feature adds no new one-extent rule | R3.4 |
| Non-zero nested fills keep the row temporary | A repeated 32-bit pattern cannot be a byte `memset`; no measured need | R3.4 |
| Row literals with explicit prefix elements do not compact | The row is not uniform; a partial-row `memset` would need a second rule | R3.4 |
| Struct rows keep the row temporary | Field stores and drop rules are out of this shape | R3.4 |
| A non-empty covered region below 64 leaf slots keeps the row temporary | Avoid a small-case call/replacement regression; the threshold does not apply when `prefix >= extent`, where a uniform zero fill emits nothing | R3.1 |
| Slice views with a missing extent do not compact | A fixed-array rule; a slice has no compile-time row count | R3.1 |
| Closure-environment and compiler-temp targets inherit the compact gate | They name real storage and the existing gate admits them; no nested-fill fixture exists in those positions, so no separate pin is added | R3.1 |
| Materialized, conditional-assignment, and self-read callers stay non-compact | The caller passes `compact=false`; the nested branch does not override it. A hidden-return conditional arm passes true and may compact; IP-19 pins it | R3.4 |
| Host time stays advisory | Target metric is the deterministic m68k instruction count | R5.1 |
| Strings cost stays in stdlib | Runtime and stdlib changes are out of scope | R5.2 |
| Moves struct copies stay | Calls use the by-value ABI; elision is a separate ABI change | R5.2 |

## Rejected Candidates

| Candidate | Evidence | Decision |
|---|---|---|
| Add a new benchmark for nested fills | The existing matrix benchmark already exposes the shape; a new benchmark would duplicate it and risk tuning | Rejected |
| Fold float zero fills | No measured need; float semantics have separate accepted limits | Rejected |
| Elide by-value struct copies in moves | Requires an internal calling-convention change; out of scope | Rejected |
| Remove bounds guards by hand | PLAN-14 measured a regression; m68k GCC already removes them in sieve | Rejected |
| Compact 63-slot nested fills | Below the measured non-byte threshold; small-case call overhead risk | Rejected |
| Compact a row with explicit prefix elements | Not uniform; a second, unmeasured rule | Rejected |
| Change the 1% no-regression guard | The guard is the safety net for every scored benchmark | Rejected |

## Review Findings and Resolutions

Filled after the phase reviews. No critical or major finding may remain open.

| Finding | Severity | Resolution |
|---|---|---|
| pending | pending | pending |

## Final Acceptance

| Gate | Result |
|---|---|
| `make check` | pending |
| `make asan` | pending |
| `make ubsan` | pending |
| `make corpus-amiga` | pending |
| `make differential` | pending |
| `make bench-shape` | pending |
| `make bench-compare REQUIRE="<final requirements>"` | pending |
| `make verify` | pending |

## Residual Opportunities

| Opportunity | Evidence | Disposition |
|---|---|---|
| Strings interpolation allocation churn | 2.97M instructions, stdlib/libnix dominated | Out of scope; separate stdlib or runtime work |
| By-value struct copy elision | moves 45,205 instructions, 36-byte copies | Separate ABI review |
| Sieve loop work | 4-5 instructions per iteration, guards removed | Saturated |
| One-extent non-byte fills below 64 slots | PLAN-14 threshold measurement | Unchanged |
