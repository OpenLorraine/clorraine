# Execution Plan: Nested Fill Compaction (PLAN-22)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Created**: 2026-11-08
> **Last Updated**: 2026-11-08
> **Progress**: 0/32 tasks (0%)
> **CodeOps Artifact Schema**: 1
> **Execution mode**: inline primary agent with `--auto-design` for eligible technical choices
> **Commit mode**: `--auto-commit` with pushes held until the final gate

## Overview

Freeze the requirements and design with two preflights, record the clean
baseline, parameterize the comparison gate, write the red emission pins, land
the nested zero-fill compaction, measure it, review it, and close the plan with
the full gate set.

**🚨 Update this document after EACH completed task.** `[ ]` not started,
`[~]` implemented with verification pending, `[x]` verified, `[!]` blocked with
a reason.

## Dependencies

```
0.1 requirements preflight ──> 0.2 plan preflight ──> 0.3 docs commit
0.3 ──> 1.1 clean isolated clone + make check
1.1 ──> 1.2 bench-base + bench ──> 1.3 attribution/probe/stale gate
1.3 ──> 1.4 baseline record ──> 1.5 baseline commit
1.5 ──> 2.1 gate unit tests ──> 2.2 gate implementation ──> 2.3 Makefile
2.3 ──> 2.4 tooling commit ──> 2.5 gate review
2.5 ──> 3.1 spec suite ──> 3.2 impl suite ──> 3.3 red run ──> 3.4 integrity plan
3.4 ──> 4.1 helper ──> 4.2 branch + fixture ──> 4.3 focused green
4.3 ──> 4.4 emission self-check + codegen commit ──> 4.5 red-pins commit
4.5 ──> 5.1 head benchmark ──> 5.2 emission audit
5.1, 5.2 ──> 5.3 reviews ──> 5.4 sanitizers
5.4 ──> 6.1 findings ──> 6.2 closeout docs commit ──> 6.3 full gates
6.3 ──> 6.4 gate-evidence commit ──> 6.5 push ──> 6.6 final report
```

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 0 | Requirements and plan gates | 3 |
| 1 | Clean baseline and audit | 5 |
| 2 | Parameterized comparison gate | 5 |
| 3 | Behaviour oracle and red pins | 4 |
| 4 | Nested zero-fill compaction | 5 |
| 5 | Measurement and review | 4 |
| 6 | Documentation and closeout | 6 |

**Total: 32 tasks across 7 phases.**

---

## Phase 0: Requirements and Plan Gates

> **Reasoning**: high — the scope, the gate change, and the emission rule are
> fixed here.
> **Lenses**: compiler semantics · measurement quality · test integrity
> **Verify**: both preflight reports show every critical/major finding resolved.

### Step 0.1: Requirements preflight

**Reference**: [Requirements](01-requirements.md) · [Ambiguity Register](00-ambiguity-register.md)
**Objective**: Adversarially review the requirements against the repository,
the measured evidence, and the goal. Record findings in
`00-preflight-report-requirements.md`; resolve every critical/major finding.

- [ ] 0.1.1 Run the requirements preflight; record every finding with severity,
      evidence, options, and ruling.
- [ ] 0.1.2 Resolve every critical/major finding; re-read the affected
      requirements; record the verdict.

**Verify**: the preflight report has no open critical/major finding.

### Step 0.2: Plan preflight

**Reference**: [Design](03-01-comparison-gate.md), [Design](03-02-nested-fill-compaction.md), [Testing Strategy](07-testing-strategy.md)
**Objective**: Verify the design is grounded in the current code, every shape
is testable, the gate change is sufficient, and no hidden ambiguity remains.

- [ ] 0.2.1 Run the plan preflight; check every cited file and line, every
      exclusion, the threshold, and the commit sequence.
- [ ] 0.2.2 Resolve every critical/major finding; update the owning documents;
      record the verdict in `00-preflight-report.md`.

**Verify**: the plan preflight report has no open critical/major finding.

### Step 0.3: Commit the plan documents

- [ ] 0.3.1 Review `git diff --cached`; commit
      `docs(plans): add the nested fill compaction plan` with the index,
      requirements, current state, designs, testing strategy, findings
      skeleton, ambiguity register, execution plan, and both preflight
      reports.
- [ ] 0.3.2 Confirm `git status --porcelain` is empty.

**Verify**: commit exists; clean tree.

---

## Phase 1: Clean Baseline and Audit

> **Reasoning**: medium — mechanical measurement, but the tree must be clean
> and the identities recorded before any code change.
> **Verify**: base and head JSON identities are recorded; the baseline table is
> filled; `dirty=false` in both documents.

### Step 1.1: Clean clone and pre-change gate

- [ ] 1.1.1 Confirm the clone is clean at the plan-docs commit; record
      `git status --porcelain`, `git rev-parse HEAD`, and
      `git rev-parse b68dae5`.
- [ ] 1.1.2 Run `make check`; record the final line and any skipped m68k tests.

**Verify**: clean status; `make check` green.

### Step 1.2: Base and head documents

- [ ] 1.2.1 Run `make bench-base COMMIT=b68dae5`; record the retained
      `build/perf/base-clc` SHA-256 and `base.json` identity.
- [ ] 1.2.2 Run `make bench`; record `head.json` identity and confirm
      `dirty=false`. Record both SHA-256 values; do not require equality.

**Verify**: both documents exist; identities recorded; trace counts present.

### Step 1.3: Attribution, probe, and stale-gate evidence

- [ ] 1.3.1 Emit the four benchmark C files with `build/perf/base-clc` into
      `build/perf/audit`, build them with m68k GCC `-O2`, and record the trace
      totals and PC buckets; write the attribution table.
- [ ] 1.3.2 Run the pre-change `make bench-compare`; record the failure and
      name the hardcoded rules that caused it.
- [ ] 1.3.3 Re-run the matrix probe with `build/perf/base-clc`: emit the C,
      replace the row temporary and copies with one `memset`, build, trace,
      and record the exact commands and the observed instructions, `.text`,
      and C bytes. Record the final `REQUIRE` values and the ruling.
- [ ] 1.3.4 Retain the emitted C and trace logs under `build/perf/audit` for
      the emission audit; record the paths in `05-findings.md`.

**Verify**: attribution table, probe evidence, and stale-gate output are in
the log; the final requirements are recorded.

### Step 1.4: Baseline record

- [ ] 1.4.1 Fill the Committed Baseline table, the calibration line, and the
      identities in [Findings](05-findings.md).

**Verify**: the table has no `pending` cell in the baseline section.

### Step 1.5: Commit the baseline record

- [ ] 1.5.1 Review `git diff --cached`; commit
      `docs(plans): record the nested fill baseline`.
- [ ] 1.5.2 Confirm `git status --porcelain` is empty.

**Verify**: commit exists; clean tree.

---

## Phase 2: Parameterized Comparison Gate

> **Reasoning**: medium — small CLI and test change; the risk is weakening the
> regression guard, which stays untouched.
> **Verify**: `make bench-test` green; flat comparison passes; missed
> requirement fails.

### Step 2.1: Unit tests

- [ ] 2.1.1 Add the comparison-gate tests listed in
      [Comparison Gate](03-01-comparison-gate.md); run `make bench-test` and
      record the expected failures on the old comparator.

**Verify**: the new tests fail for the intended reason before the fix.

### Step 2.2: Comparator implementation

- [ ] 2.2.1 Implement `--require`, validation, the requirement rows, and the
      removal of the hardcoded sieve and generic improvement rules.
- [ ] 2.2.2 Promote the tests to green.

**Verify**: `make bench-test` green.

### Step 2.3: Make target

- [ ] 2.3.1 Add the `REQUIRE` make variable and the `--require` expansion to
      `bench-compare`.
- [ ] 2.3.2 Add the `make -n` expansion test for empty and two-value `REQUIRE`.
- [ ] 2.3.3 Run a flat comparison with an empty `REQUIRE` and confirm exit 0.
- [ ] 2.3.4 Record the `AGENTS.md` `make bench-compare` row change for the
      closeout docs commit; do not mix it into the tooling commit.

**Verify**: flat compare passes; `make bench-test` still green; the AGENTS row
is queued for the docs commit.

### Step 2.4: Commit the tooling change

- [ ] 2.4.1 Review `git diff --cached`; commit
      `feat(perf): parameterize the benchmark comparison gate`.
- [ ] 2.4.2 Record the commit and the empty emission audit (no codegen files
      changed).

**Verify**: commit exists; tree clean; no codegen file in the diff.

### Step 2.5: Gate review

- [ ] 2.5.1 Dispatch `correctness-reviewer` on the tooling diff with a
      blast-radius check (every compare caller and test) and a test-integrity
      check (the 1% guard and each requirement case fail when broken).
- [ ] 2.5.2 Record the verdict in [Findings](05-findings.md); a critical or
      major finding blocks Phase 3 until resolved and re-reviewed once.

**Verify**: the tooling review verdict is clean and recorded.

---

## Phase 3: Behaviour Oracle and Red Pins

> **Reasoning**: medium — the tests are the oracle; a weak pin would let a
> wrong emission pass.
> **Verify**: behaviour suite green on base; impl pins red on base.

### Step 3.1: Specification suite

- [ ] 3.1.1 Add `compiler/tests/nested_fill_compaction_spec_test.rs` with the
      ST cases in [Testing Strategy](07-testing-strategy.md).
- [ ] 3.1.2 Run it with the unmodified clone compiler; record green and
      the observed output values that become the expected text.

**Verify**: all ST cases pass before any compiler change.

### Step 3.2: Implementation suite

- [ ] 3.2.1 Add `compiler/tests/nested_fill_compaction_impl_test.rs` with the
      IP pins, including the 63/64 boundary and every exclusion.

**Verify**: the suite compiles; it is not yet green.

### Step 3.3: Red run

- [ ] 3.3.1 Run the impl suite against the unmodified compiler; record every
      failing pin and the exact observed old spelling.

**Verify**: the expected IP pins fail; the behaviour suite stays green.

### Step 3.4: Test integrity

- [ ] 3.4.1 Confirm each pin targets a distinct shape and that no assertion is
      satisfied by the old emission (inspect the failure output).
- [ ] 3.4.2 Record the plan to disable the compaction condition and to remove
      each exclusion in turn after the fix.

**Verify**: the red record lists each pin, its observed old emission, and the
planned exclusion-removal checks.

---

## Phase 4: Nested Zero-Fill Compaction

> **Reasoning**: high — emission and C-validity change.
> **Lenses**: codegen · C99 · ownership (none) · threshold
> **Verify**: focused suites green; strict C builds; emission self-check clean.

### Step 4.1: Helper

- [ ] 4.1.1 Add the free uniform-zero predicate and the eligibility helper in
      `compiler/src/codegen/function.rs`; use `constant_fill_integer`,
      `checked_mul`, and the existing `FILL_LOOP_THRESHOLD`.
- [ ] 4.1.2 Add a `#[cfg(test)]` module in `codegen/function.rs` and unit-test
      the predicate's `0u`, `i32(0)`, `-0`, `false`, `Char(0)`, `Materialize`,
      non-zero, float-leaf, and `i32(f32(0.0))` cases.
- [ ] 4.1.3 Unit-test the eligibility helper for 63/64 slots, a missing
      extent, `usize` overflow, and `covered_rows == 0`.

**Verify**: helper tests green; no behaviour change yet.

### Step 4.2: Emission branch

- [ ] 4.2.1 Insert the compaction branch before the row-temporary block; emit
      one `memset` with the first covered row address, the covered row count,
      and the row size.
- [ ] 4.2.2 Update `tools/perf/fixtures/shapes.clo` with a nested zero fill
      above 64 slots, a nested `bool` zero fill, a prefix case, an
      array-parameter case, and a hidden return case.
- [ ] 4.2.3 Run `cargo build --workspace`.

**Verify**: build green; the shape fixture covers the new emission.

### Step 4.3: Focused green

- [ ] 4.3.1 Run both focused suites; all green.
- [ ] 4.3.2 Re-run the impl suite with the compaction condition disabled and
      confirm the positive pins fail; restore.
- [ ] 4.3.3 Remove each exclusion in turn (non-zero, float, struct, explicit
      row, volatile, field, threshold) and confirm its guard fails; restore.
- [ ] 4.3.4 Run `make bench-shape`; strict C builds warning-free.

**Verify**: focused suites green; both integrity checks reproduced;
`bench-shape` green.

### Step 4.4: Emission self-check and commit

- [ ] 4.4.1 Emit matrix with the head compiler; confirm one `memset`, no
      `__cl_fillrow`, and the expected count expression.
- [ ] 4.4.2 Review `git diff --cached`; commit
      `perf(codegen): compact nested zero fills to one memset`.

**Verify**: commit exists; tree clean; matrix emission matches the design.

### Step 4.5: Commit the pre-measurement records

- [ ] 4.5.1 Fill the red/green record and the tooling-review verdict in
      [Findings](05-findings.md).
- [ ] 4.5.2 Review `git diff --cached`; commit
      `docs(plans): record the nested fill red pins`.
- [ ] 4.5.3 Confirm `git status --porcelain` is empty before the head
      benchmark.

**Verify**: commit exists; clean tree.

---

## Phase 5: Measurement and Review

> **Reasoning**: high — the win and the safety case are decided here.
> **Verify**: `bench-compare` meets the declared requirements with no
> regression; four review verdicts recorded.

### Step 5.1: Head benchmark

- [ ] 5.1.1 Run `make bench`; record the head identity, SHA-256, and
      `dirty=false`.
- [ ] 5.1.2 Run
      `make bench-compare REQUIRE="<final requirements>"`; the final
      requirements are recorded in Phase 1 after the clean probe. Record the
      table and the verdict.

**Verify**: requirements met; no benchmark regresses by more than 1%; output
hashes equal.

### Step 5.2: Emission audit

- [ ] 5.2.1 Emit base and head C over all seven corpus programs, the
      benchmarks, and the Amiga examples; diff.
- [ ] 5.2.2 Classify every diff; record unexplained diffs as blockers.

**Verify**: every diff is a nested zero fill; no unexplained unit.

### Step 5.3: Phase reviews

- [ ] 5.3.1 Dispatch `correctness-reviewer` and `performance-auditor` on the
      codegen diff.
- [ ] 5.3.2 Dispatch read-only `memory-safety-auditor` and
      `emission-integrity-auditor` on the same diff.
- [ ] 5.3.3 Merge findings; record rulings; resolve all critical/major; re-review
      the fix diff once.
- [ ] 5.3.4 If any codegen change landed, commit it, rerun 5.1 and 5.2 on the
      new clean commit, and replace the head table and emission audit.

**Verify**: no open critical/major finding; the head document and emission
audit describe the final codegen commit.

### Step 5.4: Sanitizers

- [ ] 5.4.1 Run `make asan`; record the suite count and result.
- [ ] 5.4.2 Run `make ubsan`; record the suite count and result.
- [ ] 5.4.3 If a sanitizer finding changes codegen, commit the fix, rerun
      5.1 and 5.2 on the new clean commit, replace the head table and
      emission audit, and re-review the fix once.

**Verify**: both green; the head document describes the final codegen commit.

---

## Phase 6: Documentation and Closeout

> **Reasoning**: medium — mechanical closeout, but the full gate set is the
> acceptance.
> **Verify**: full gate set green; clean tree; `origin/main..HEAD` zero.

### Step 6.1: Findings ledger

- [ ] 6.1.1 Fill the head/delta table, the emission audit, and the review
      findings in [Findings](05-findings.md). Leave the final acceptance gate
      rows pending until 6.4.

**Verify**: every section except the acceptance gate table is filled.

### Step 6.2: Roadmap and docs commit

- [ ] 6.2.1 Add the PLAN-22 roadmap row with status `Executing` and the
      measured result; set it to `Done` only in 6.4 after the gates pass.
- [ ] 6.2.2 Update the AGENTS `bench-compare` row; update
      `docs/known-limitations.md` only if the measurement boundary changed.
- [ ] 6.2.3 Review `git diff --cached`; commit
      `docs(plans): record the nested fill closeout`.
- [ ] 6.2.4 Confirm `git status --porcelain` is empty.

**Verify**: docs commit exists; clean tree; roadmap row consistent.

### Step 6.3: Full gates

- [ ] 6.3.1 Run `make check`, `make asan`, `make ubsan`,
      `make corpus-amiga`, `make differential`, `make bench-shape`,
      `make bench-compare REQUIRE="<final requirements>"`, and `make verify`;
      record each final line in a scratch log outside the repository.
- [ ] 6.3.2 If a gate is blocked, stop and report the exact blocker; do not
      push a red tree.

**Verify**: every gate green.

### Step 6.4: Record the gate evidence

- [ ] 6.4.1 Fill the Final Acceptance table from the 6.3 logs.
- [ ] 6.4.2 Set the PLAN-22 roadmap row to `Done` with the measured result.
- [ ] 6.4.3 Review `git diff --cached`; commit
      `docs(plans): record the nested fill gate evidence`.
- [ ] 6.4.4 Confirm `git status --porcelain` is empty.

**Verify**: the acceptance table has no `pending` cell; the evidence commit
exists; clean tree.

### Step 6.5: Push and tree check

- [ ] 6.5.1 Push to `origin/main` only after 6.3 and 6.4 are green.
- [ ] 6.5.2 Verify `git status -sb` clean and
      `git rev-list --count origin/main..HEAD` is 0.

**Verify**: both commands satisfy the acceptance.

### Step 6.6: Final report

- [ ] 6.6.1 Report the benchmark table, emitted-shape attribution, test
      coverage, gate evidence, and residual opportunities.

**Verify**: the report includes every acceptance item.

---

## Specialist Agents

| Phase | Agent | Mode |
|---|---|---|
| 0 | `preflight-auditor` | read-only |
| 2 | `correctness-reviewer` | review |
| 5 | `correctness-reviewer` | review |
| 5 | `memory-safety-auditor` | read-only |
| 5 | `emission-integrity-auditor` | read-only |
| 5 | `performance-auditor` | review |

## Commit Plan

| Order | Message | Content |
|---|---|---|
| 1 | `docs(plans): add the nested fill compaction plan` | Plan documents, preflight reports |
| 2 | `docs(plans): record the nested fill baseline` | Baseline table, calibration line, identities |
| 3 | `feat(perf): parameterize the benchmark comparison gate` | `bench.py`, tests, Makefile |
| 4 | `perf(codegen): compact nested zero fills to one memset` | Codegen, spec/impl suites, helper unit tests, `shapes.clo` |
| 5 | `docs(plans): record the nested fill red pins` | Red/green record, tooling-review verdict |
| 6 | `docs(plans): record the nested fill closeout` | Head table, emission audit, reviews, roadmap, AGENTS |
| 7 | `docs(plans): record the nested fill gate evidence` | Final acceptance table |

No push before the Phase 6 full gate.

## Risks and Controls

| Risk | Control |
|---|---|
| `memset` on a volatile target | `compact` gate unchanged; volatile cases pinned |
| Wrong byte count for a parameter row | `ArrayTarget` row size pinned by IP-15 |
| A small nested fill regresses | 63/64 threshold pinned by IP-05/IP-06 |
| The gate change weakens regression detection | The 1% guard is untouched; unit tests cover it |
| A hidden emission diff | Full base/head emission audit by the emission-integrity auditor |
| Float or struct row breaks C99 | Excluded by type and pinned by IP-08/IP-09 |
