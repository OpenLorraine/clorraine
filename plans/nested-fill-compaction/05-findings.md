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
| sieve | 1,040,581 | 76,728 | 311,284 | 0.79 ms |
| matrix | 10,959 | 80,712 | 313,817 | 0.83 ms |
| strings | 2,964,051 | 77,604 | 312,685 | 1.25 ms |
| moves | 45,205 | 76,896 | 313,173 | 0.66 ms |

Calibration: `instructions` 3,372; `host_release_ns` best 0.504 ms. It is not a
scored benchmark and the JSON has no `.text`, C-byte, or median field for it.
Host medians are advisory.

**Base compiler source commit:** `b68dae5858585d43f8df7894a119798a4607ec62`.

**Head source commit:** `72a39a112072` (plan-docs commit); no compiler file
differs from `b68dae5`.

**Base compiler SHA-256:** `94e6d8f6e13cadd79b2f7759d32b933c300946eb7c848129dc335d53c16e90f0`.

**Head compiler SHA-256:** `22d7bf9f9300d37a7536a8d564cb81f767e878813195140855da39cf11455d37`.

The hashes differ because the base builds in `build/perf/base-worktree` and a
debug binary embeds its build path. The compiler source is identical; every
metric in the two documents is byte-identical.

**Dirty state:** `false` in both documents.

**Commands:** `make bench-base COMMIT=b68dae5`, `make bench`.

## Exploratory Probe (re-run with the base compiler)

The probe patches only the emitted C of matrix; the benchmark source and
expected output are unchanged. This re-run uses `build/perf/base-clc`, the
retained compiler from `b68dae5`, and the exact commands in
[Testing Strategy](07-testing-strategy.md#reproducible-audit-commands).

| Metric | Current emission | One `memset` | Delta |
|---|---:|---:|---:|
| matrix trace instructions (raw) | 14,331 | 13,280 | -1,051 |
| matrix net instructions (raw - 3,372) | 10,959 | 9,908 | -9.6% |
| matrix m68k `-O2` `.text` | 80,712 | 76,676 | -5.0% |
| matrix C bytes | 313,637 | 310,980 | -2,657 (-0.85%) |
| output | `matrix: 523776` | `matrix: 523776` | equal |

The probe C bytes are emitted from the clone root, so they match the head
document's path-dependent C size, not the base document's nested-worktree C
size (313,817). The instruction and `.text` metrics are path-independent and
match the base document exactly (10,959 / 80,712). The probe exceeds the
initial 5%/3% requirement, so the final requirement stays
`matrix:instructions:5 matrix:text:3`. This is the Phase 1 ruling.

Artifacts: `build/perf/audit/matrix-base.c`, `matrix-probe.c`, the two trace
files, and `sizes.txt`. They are gitignored.

## Stale Gate Evidence

`make bench-compare` on the flat baseline exited 2:

```
compare: FAILED
  - sieve: instructions target missed (1040581 > 1040581 * 0.80)
  - sieve: .text target missed (76728 > 76728 * 0.90)
  - sieve: C-byte target missed (311284 > 311284 * 0.75)
  - no benchmark improved by 20% instructions or 10% .text
```

This is the evidence for R2: the pre-change gate cannot pass a tree with no
regression, because it encodes PLAN-14's sieve targets and a generic
improvement rule.

## Tooling Gate Review

Implementation commit `6256c40`; fix commit `8b47312`. The change touches only
`tools/perf/bench.py`, `tools/perf/tests/test_bench.py`, and the `Makefile`;
`git diff --name-only 5f708a6..8b47312` contains no codegen, runtime, stdlib,
or measurement file. The emission audit for this phase is empty.

**Red evidence.** The new `CompareTests` run against the pre-change comparator
(`5f708a6`'s `bench.py`) reports 14 failures out of 19. The five that pass are
the unchanged invariants: output mismatch, instruction regression, `.text`
regression, missing trace, and structural mismatch. The requirement cases
(met, missed, flat-with-requirement, duplicate, boundary, invalid input) all
fail on the old code. Command: copy the old `bench.py` and the new tests into a
scratch tree and run
`python3 -m unittest discover -s tools/perf/tests -t tools/perf -k CompareTests -v`.

| Finding | Severity | Resolution |
|---|---|---|
| RV-001: the missed-text and flat-requirement tests passed on the old comparator for the wrong reason | MAJOR | Assert the exact named failure lines (`matrix: text -1.0% misses the required -3%`, `matrix: instructions +0.0% misses the required -5%`) and the `need <=` rows; the red run now fails both |
| RV-002: float boundary could false-fail an exact 80% or false-pass a tiny percentage | MINOR | Use `Decimal` and the exact inequality `head * 100 <= base * (100 - percent)`; boundary and `1e-15` tests added |
| RV-003: duplicate, colon-name, and no-table-before-error cases were missing | MINOR | Added the duplicate requirement, the `parse_args` colon-name case, and empty-output assertions after every rejected input |
| RV-004: stale "plan thresholds" wording and an undocumented constant | MINOR | Reworded the comparator and Makefile comments; documented `REQUIREMENT_METRICS` |
| RV-005: plan progress and the old-red/empty-emission records were missing | MINOR | This section and the Phase 2 checkboxes record both |

**Verify:** `make bench-test` 40 tests OK; flat comparison exit 0; the declared
5%/3% requirement on the flat pair exits 1 with both named failures.

## Head and Delta

Filled after the codegen commit with `make bench` and
`make bench-compare REQUIRE="matrix:instructions:5 matrix:text:3"`. The Phase 1
probe reproduced -9.6% instructions and -5.0% `.text`, so the initial values
are final.

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
