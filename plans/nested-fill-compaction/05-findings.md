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
| RV2-001: a huge percentage raised a decimal traceback and `1e-27` on a flat pair still passed | MINOR | Bound percentages to at most 100 and six decimal places; compare with exact scaled integers. Tests cover 101, `1e1000000`, `1e-7`, `1e-15`, and `1e-27` |

**Re-review verdict:** no critical or major finding; RV-001 and RV-003/004/005
fixed, RV-002 and RV2-001 fixed by the exact scaled-integer comparison. The
gate phase is clean.

**Verify:** `make bench-test` 40 tests OK; the new `CompareTests` still report
14 failures out of 19 against the old comparator; the flat comparison exits 0;
the declared 5%/3% requirement on the flat pair exits 1 with both named
failures.

## Phase Reviews

Four independent reviews ran on the codegen commit `85d0fde`:

| Review | Verdict | Evidence |
|---|---|---|
| Correctness | no critical/major; four minor | diff `85d0fde^..85d0fde`; focused suites 19/19 and 13/13; base red run 10/9 |
| Memory safety | no findings | 40+ host ASan probes (locals, parameters, return slots, prefixes, 3D/4D, bool/u8/u16/i8, closure env, temps, generics, address-taken); all clean, base behavior equal |
| Emission integrity | no findings | base/head diff over 7 corpus, 5 benchmark, 25 Amiga example, 6 fault programs, 76 probes; one matrix hunk (65 removed, 1 added); host/m68k/vbcc/freestanding strict builds; no unexplained diff |
| Performance | land; three minor + one info | matrix -9.6% / -5.0%; no scored regression; independent threshold and byte-leaf runs |

| Finding | Severity | Resolution |
|---|---|---|
| RV-001: no float-to-integer cast leaf pin | MINOR | Added `uniform_zero_rejects_a_float_to_integer_cast`, `0u`/`i32(0)`/`-0` unit cases, and IP-23 (`i32[8][9] = [; [; i32(f32(0.0))]]` keeps the old spelling) |
| RV-002: the full-prefix early return is outside `compact`, contradicting the design snippet and unpinned for a non-compact target | MINOR | The design now shows the early return outside the flag; IP-24 pins a field-target full prefix with no `__cl_fillrow` and no memset |
| RV-003: the shape fixture comment claimed a 3D case it did not add; two new front-end diagnostics | MINOR | Corrected the comment to two-dimensional; the two front-end diagnostics are recorded as accepted fixture noise (the strict C legs are warning-free) |
| RV-004: helpers sat between the import groups | MINOR | Moved all helpers below the `use` block |
| PE-001: 64-slot exact fills may add 6-7 instructions while saving `.text` | MINOR | Recorded as an accepted limit; the 72+ shapes win both metrics and the scored matrix covers 1,024 slots |
| PE-002: byte leaves are neutral to +6 instructions | MINOR | Recorded as an accepted limit; the measured shape is `i32` |
| PE-003: matrix overwrites the zero fill before reading it | INFO | Recorded in the attribution: the win is the emitted fill cost, not a live-data saving |

The pre-existing volatile-target invalid C is reported separately by both the
memory and emission auditors; base and head emit byte-identical `memcpy`
spellings and both fail the strict host build. It stays an accepted limit.

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
| Red on base (`b68dae5`) | IP-01, 02, 03, 04, 06, 15, 16, 17, 19, 20, 21, 24 (11 test functions) |
| Green on base | IP-05, 07, 08, 09, 10, 11, 12, 13, 14, 23 (10 exclusion and regression guards) |
| Behaviour oracle on base | 13 of 13 cases pass |
| Behaviour oracle on head | 13 of 13 cases pass |
| Emission pins on head | 21 of 21 cases pass |

**Base evidence:** the unmodified compiler emits `i32 __cl_fillrow_1[32]`, 32
scalar zero stores, and 32 `memcpy` calls for matrix `[; [; 0]]`. The positive
pins fail on that spelling; the exclusion guards pass.

**Test-integrity mutations** (each applied to the fix, then restored):

| Mutation | Observed failure |
|---|---|
| Disable the compaction branch and the full-prefix return | the 10 positive pins fail; the 9 guards pass |
| Remove the `compact` requirement | the volatile-global and field-target guards fail |
| Accept any constant instead of zero | the non-zero-fill guard fails |
| Lower the threshold to 1 slot | the 63-slot boundary guard fails |

The float guard is protected twice: the leaf type check and the zero constant
test both reject it. The struct and explicit-row guards are structural: those
fill shapes have no uniform constant-zero leaf.

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
| Materialized, conditional-assignment, and self-read callers stay non-compact | The caller passes `compact=false`; the nested branch does not override it for stores. A hidden-return conditional arm passes true and may compact; IP-19 pins it. The zero-covered early return is outside the flag and emits nothing for any target; IP-24 pins a non-compact full prefix | R3.4 |
| A volatile nested target keeps the per-row `memcpy` spelling | That spelling discards the volatile qualifier and is rejected by a strict host build; the defect is pre-existing at `b68dae5` and outside this change. The optimization does not compact it and does not fix it | R3.4 |
| An exactly 64-slot nested fill may add 6-7 instructions while saving about 216 B `.text` | The independent 64-slot probes measure +7 (`i32[8][8]`), +6 (`i32[64][1]`), and -70 (`i32[1][64]`). The 72-slot and larger shapes win both metrics. 64 is kept as a code-size-preferring boundary; the scored matrix covers 1,024 slots | R3.1 |
| Byte leaves are neutral to +6 instructions with smaller `.text` | The measured shape is `i32`; `u8[8][16]` measures +4, `u8[16][16]` +6, both with smaller `.text`. Admitted for code size and consistency, not a measured runtime win | R3.1 |
| The matrix benchmark overwrites its zero fill before reading it | The 9.6% measures the emitted fill cost, not a live-data saving. The benchmark is unchanged; changing it would tune the benchmark to the optimization | R5.2 |
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
