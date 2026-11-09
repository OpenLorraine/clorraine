# Execution Plan: Constant Array Initialization (PLAN-25)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-09 23:15
> **Progress**: 12/37 tasks (32%)
> **CodeOps Artifact Schema**: 1
> **Commit mode**: `--auto-commit --auto-design`; pushes are held to the final
> gate and performed with the git-commit skill in push mode.

## Overview

This plan delivers the measured constant-array data-template emission. Phase 1
commits the plan and benchmark and establishes the v1.1.0 and isolated
baselines. Phase 2 writes the specification and emission tests first and
records the red run. Phase 3 implements the guard-free predicate and the
template emission, then measures the win. Phase 4 hardens the cases and runs
the sanitizer legs. Phase 5 performs the emission audit, the four phase
reviews, the full gate, and the final focused re-review. Phase 6 updates the
documentation and roadmap and pushes.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 1 | Baseline and committed benchmark | 7 |
| 2 | Specification tests and red run | 5 |
| 3 | Implementation and first measurement | 8 |
| 4 | Hardening, sanitizers, and fix review | 6 |
| 5 | Emission audit, full gate, and reviews | 7 |
| 6 | Documentation and closeout | 4 |

**Total: 37 tasks across 6 phases.** No fabricated hour estimates.

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes below are the single source of truth. Every task line
> appears exactly once. Mark `[~]` with a timestamp on implementation, promote
> to `[x]` only after the task’s verify passes, and update the Progress header
> and Last Updated stamp after every task. A blocker is `[!]` with a short
> reason on the same line. Timestamps come from `date '+%Y-%m-%d %H:%M'`.

---

## Phase 1: Baseline and committed benchmark

> **Phase baseline tree**: recorded by the exec-plan skill.
> **Lenses**: compiler and language; performance engineering.
> **Reasoning**: high — this phase fixes the measurement identity and the
> benchmark that every later decision depends on.

### Step 1.1: Commit the plan, benchmark, and baseline

**Reference**: R1, AR-01, AR-02, AR-04
**Objective**: Make the plan and benchmark durable and establish the exact
base and isolated baselines before any compiler change.

- [x] 1.1.1 Commit the plan documents (index, requirements, register, requirements preflight report, plan preflight report, current state, design, testing strategy, execution plan, findings skeleton) with the git-commit skill; no compiler file may change. ✅ (completed: 2026-11-09 22:40)
- [x] 1.1.2 Add `benchmarks/lookup/{program.clo,expected-output.txt,README.md}` and update `benchmarks/README.md` (table, scored count, baseline-location note); confirm the host output is `lookup: 134217728`; run the targeted smoke to `build/perf/lookup-smoke.json` (not `head.json`); commit the benchmark with the git-commit skill. ✅ (completed: 2026-11-09 22:44)
- [x] 1.1.3 Remove the stale `build/perf/base.json`, `build/perf/head.json`, and `build/perf/base-clc`; run `make check` on the clean benchmark commit and record the result; confirm the tree is clean. ✅ (completed: 2026-11-09 22:48)
- [x] 1.1.4 Run `make bench-base COMMIT=v1.1.0` and `make bench`; copy `build/perf/head.json` to `build/perf/isolated-head.json`; copy the current `target/debug/clc` to `build/perf/isolated-base-clc`; record the compiler identities, the `clc 1.1.0` base version line, the clean/dirty flags, and confirm each harness document has `dirty=false` and the expected commit. ✅ (completed: 2026-11-09 23:02)
- [x] 1.1.5 Measure the small-table boundary and the 256-entry sensitivity: use `build/perf/isolated-base-clc` on `i32[4]/i32[8]/i32[16]/i32[32]` and `u8[64]`, plus a 256-entry lookup variant; record `.text` and instructions against the store form in `05-findings.md`; finalize the threshold. ✅ (completed: 2026-11-09 23:02)
- [x] 1.1.6 Re-run the committed `lookup` probe with `build/perf/isolated-base-clc`: current emission vs a block-scope `static const` template plus `memcpy`; record the exact `.text`, raw/net instructions, output, commands, and artifact paths in `05-findings.md`. ✅ (completed: 2026-11-09 23:02)
- [x] 1.1.7 Write the committed baseline table, threshold, three compiler identities, and benchmark sensitivity in `05-findings.md`; commit the baseline evidence with the git-commit skill. ✅ (completed: 2026-11-09 23:08)

**Deliverables**:
- The plan, benchmark, and baseline evidence are committed.
- `base.json`/`base-clc` match the v1.1.0 document; `isolated-head.json`/`isolated-base-clc` match the benchmark commit. The two pairs deliberately differ.
- `05-findings.md` has the real baseline table, not placeholders.

**Verify**: `make check` green on the benchmark commit; the baseline documents
name `v1.1.0` and the benchmark commit; the probe output is equal.

---

## Phase 2: Specification tests and red run

> **Phase baseline tree**: the benchmark commit (no compiler change).
> **Lenses**: compiler and language; test integrity.
> **Reasoning**: high — the tests are the immutable oracle and must be red
> before the implementation. ✅ (completed: 2026-11-09 23:15)

### Step 2.1: Write the tests first

**Reference**: R3, ST-1..ST-48, AR-06
**Objective**: Encode the behavior and emission expectations before changing
codegen, and prove the positive pins fail on the base.

- [x] 2.1.1 [spec-author] Write `compiler/tests/constant_array_initializer_spec_test.rs` from ST-1..ST-41 and ST-46; size every positive 2/4-byte table above the finalized threshold; keep the 1-byte tables as old-store controls; carry the `RDC_ASAN`/`RDC_UBSAN` markers and add the sanitizer flags in the C builder; resolve a relative `RDC_CLC` against the workspace root; do not modify compiler code. ✅ (completed: 2026-11-09 23:15)
- [x] 2.1.2 [spec-author] Write `compiler/tests/constant_array_initializer_impl_test.rs` from ST-42..ST-45 and the emission assertions for ST-1..ST-41; use the finalized 2/4-byte threshold sizes and the 1-byte old-store controls; do not modify compiler code. ✅ (completed: 2026-11-09 23:15)
- [x] 2.1.3 Add a **module-level** eligible one-extent explicit-list non-volatile `let` `i32[64]` table to `tools/perf/fixtures/shapes.clo`; have `main` read it; run `make bench-shape` to prove the fixture is valid before the implementation. ✅ (completed: 2026-11-09 23:15)
- [x] 2.1.4 Run both new suites against the pre-change compiler (`RDC_CLC="$PWD/build/perf/isolated-base-clc"`); record the positive-pin failures and the behavior-suite pass in `05-findings.md`. ✅ (completed: 2026-11-09 23:15)
- [x] 2.1.5 Record the red evidence in `05-findings.md`; keep the red tests uncommitted until the implementation makes them green. Do not create a standalone red commit. ✅ (completed: 2026-11-09 23:15)

**Deliverables**:
- Both suites exist and build.
- The red record names the exact command, compiler identity, and failing pins.
- The behavior suite prints the expected values on the pre-change compiler.

**Verify**: the red run is recorded; every exclusion pin passes on the base;
the fixture is valid on every `bench-shape` leg; no red test is committed
before the implementation. ✅ (completed: 2026-11-09 23:15)

---

## Phase 3: Implementation and first measurement

> **Phase baseline tree**: the benchmark commit with the red tests
> uncommitted, then the implementation commit.
> **Lenses**: compiler and language; emission integrity; memory safety;
> performance.
> **Reasoning**: high — the change alters emitted C for every eligible global
> and must preserve behavior, traps, order, and C validity.

### Step 3.1: Emit the template

**Reference**: R2, 03-design §Emission, AR-03/AR-04/AR-05
**Objective**: Implement the guard-free predicate and the template emission,
make the focused suites green, and measure the win.

- [ ] 3.1.1 Add the `guard_free_constant` predicate and its unit cases in `compiler/src/codegen/program.rs`: literals and non-float scalar casts and `Neg`/`BitNot`/`Not` are accepted; Binary, float casts, conditionals, materialize, calls, places, and intrinsics are rejected.
- [ ] 3.1.2 Add `emit_constant_array_template` and integrate it in `emit_global_init`: one extent, `elements.len() == extent`, 2/4-byte scalar leaf, the finalized threshold, not volatile, non-freestanding, extent-sized `static const` template with an explicit `__cl_ctmpl_<n>` counter, eight values per line, one `memcpy`, no per-element stores.
- [ ] 3.1.3 Run the focused spec and impl suites; make the positive pins green and keep every exclusion pin green.
- [ ] 3.1.4 Run `make bench-shape` (the fixture now exercises the template on hosted-amiga, nix13, and vbcc; freestanding keeps the stores) and `make check`; fix any C-validity or warning failure.
- [ ] 3.1.5 Commit the implementation, the now-green tests, the fixture, the red/green evidence, the `PLAN-25` roadmap row (Executing), and the plan index (Executing) with the git-commit skill.
- [ ] 3.1.6 Confirm the tree is clean and `head.json` will record `dirty=false`; run `make bench`; run the isolated comparison with `--require lookup:instructions:10 --require lookup:text:10` and the cumulative `make bench-compare REQUIRE="lookup:instructions:10 lookup:text:10"`; record the tables and outcomes in `05-findings.md` (commit those findings before the next measured run).
- [ ] 3.1.7 Dispatch the correctness reviewer, memory-safety auditor, emission-integrity auditor, and performance auditor on the Phase 3 diff with blast-radius and test-integrity packets; record verdicts.
- [ ] 3.1.8 Resolve every critical/major finding; if a fix changes codegen, continue to Phase 4 with a fix commit.

**Deliverables**:
- The implementation commit changes only the intended codegen and test/fixture files.
- The focused suites are green.
- The isolated and cumulative benchmark tables are recorded.
- Four phase-review records exist.

**Verify**: focused suites green; `make check` green; `make bench-shape` green;
the isolated comparison has no scored regression; the `lookup` requirement
passes.

---

## Phase 4: Hardening, sanitizers, and fix review

> **Phase baseline tree**: the implementation commit.
> **Lenses**: compiler and language; memory safety; emission integrity;
> performance.
> **Reasoning**: high — the edge cases include address-taken mutation, guard
> exclusions, profile gating, and template-name collisions.

### Step 4.1: Harden the edge cases

**Reference**: R3.4/R3.5, ST-42..ST-45, AR-06
**Objective**: Prove the exclusions and the test integrity, and run the
sanitizer legs. Phase 2 already writes the ST cases; Phase 4 adds only the
negative/mutation controls and any case a review finds.

- [ ] 4.1.1 Add the test-integrity controls: disable the fast path and confirm the positive pins fail; remove one exclusion at a time (short list, empty list, fill, guard-free, volatile, profile, 1-byte type, other type, threshold) and confirm the matching pin fails; add any review-driven case not already in ST-1..ST-46.
- [ ] 4.1.2 Run `make asan` and `make ubsan`; fix any memory-safety or undefined-behavior finding.
- [ ] 4.1.3 Run the focused behaviour suite under `RDC_ASAN` and `RDC_UBSAN`; confirm the address-taken mutation and guard cases are clean.
- [ ] 4.1.4 If any codegen or test changed, commit the hardening/fix changes with the git-commit skill; otherwise record that no change was needed.
- [ ] 4.1.5 If codegen changed, dispatch the same four reviewers on the Phase 4 diff; record verdicts.
- [ ] 4.1.6 Resolve every critical/major finding; commit the fix (if any), the review records, the red/green and threshold findings, and the roadmap/index updates with the git-commit skill; confirm the tree is clean. If codegen changed, rebuild `clc`, rerun `make bench`, rerun both comparisons and the emission audit, then repeat 4.1.5/4.1.6.

**Deliverables**:
- The hardening cases are committed.
- ASan/UBSan are green.
- The fix review (if any) is recorded.

**Verify**: `make asan` and `make ubsan` green; focused suites green; no
critical/major finding open.

---

## Phase 5: Emission audit, full gate, and reviews

> **Phase baseline tree**: the last codegen commit.
> **Lenses**: emission integrity; memory safety; performance; semantics only
> if a review raises a semantic question.
> **Reasoning**: max — this is the acceptance gate for the whole feature.

### Step 5.1: Prove the final state

**Reference**: R4, R6, AR-08
**Objective**: Attribute every emitted-C change, run every gate, and close the
reviews.

- [ ] 5.1.1 Confirm the tree is clean (no uncommitted findings/review/roadmap edits), rebuild `clc` on the final head, run `make bench`, run the isolated comparison with `--require lookup:instructions:10 --require lookup:text:10`, run the cumulative `make bench-compare REQUIRE="lookup:instructions:10 lookup:text:10"`, and confirm `head.json` records the feature commit with `dirty=false`.
- [ ] 5.1.2 Run the isolated and cumulative emission audits over the explicit matrix in R4.3 (corpus with its run profile, benchmarks with host and hosted-amiga, Amiga examples with hosted-amiga/hosted-amiga-13); record every changed unit and the benchmark-source diff/hashes.
- [ ] 5.1.3 Run `make check`, `make asan`, `make ubsan`, `make corpus-amiga`, `make differential`, `make bench-shape`, and `make verify`; record every result.
- [ ] 5.1.4 Run the comment-hygiene check on the new source/tests/fixture for plan/requirement/task identifiers; record no hits.
- [ ] 5.1.5 Run the final focused re-review on the complete feature diff; confirm no critical/major finding remains.
- [ ] 5.1.6 Update `05-findings.md` with the final tables, threshold, emission audit, review verdicts, and gate evidence; commit the gate evidence with the git-commit skill.
- [ ] 5.1.7 If any fix changed codegen after a gate, commit the fix, rebuild `clc`, rerun `make bench`, both comparisons, the emission audit, the affected suites, the reviews, and the full gate before closeout.

**Deliverables**:
- The emission audit is explained.
- Every gate is green.
- The final focused re-review is clean.
- `05-findings.md` is complete.

**Verify**: all named gates green; the isolated comparison has no scored
regression; the `lookup` requirement passes; no critical/major finding open.

---

## Phase 6: Documentation and closeout

> **Phase baseline tree**: the gate-evidence commit.
> **Lenses**: documentation and roadmap consistency.
> **Reasoning**: medium — documentation and closeout only; no code change.

### Step 6.1: Close the plan

**Reference**: R5, AR-09
**Objective**: Make the repository state and the roadmap match the delivered
feature.

- [ ] 6.1.1 Update `AGENTS.md` if the measurement boundary or commands changed; update `benchmarks/README.md`; add the freestanding multi-byte limit to `docs/known-limitations.md`; update the `PLAN-25` roadmap row/progress count and the plan index Status/Last Updated.
- [ ] 6.1.2 Run `make check` after the documentation changes (no compiler file changed) and record the result.
- [ ] 6.1.3 Commit the closeout with the git-commit skill.
- [ ] 6.1.4 Push with the git-commit skill in push mode; verify the tree is clean and `origin/main..HEAD` is 0.

**Deliverables**:
- The documentation and roadmap are current.
- The branch is pushed and clean.

**Verify**: `make check` green; the roadmap and plan index are current; the
push record shows a clean tree and zero unpushed commits.

---

## Dependencies

```
Phase 1 (baseline/benchmark)
    ↓
Phase 2 (spec tests, red)
    ↓
Phase 3 (implementation/first measurement)
    ↓
Phase 4 (hardening/sanitizers)
    ↓
Phase 5 (audit/full gate)
    ↓
Phase 6 (docs/closeout)
```

The phases are sequential. A red Phase 2 is intentional; Phase 3 is the first
phase that must be green. A codegen-changing fix in Phase 3 or 4 loops back
through the affected reviews and gates before Phase 5.

---

## Success Criteria

The feature is complete when:

1. ✅ The plan, benchmark, and baseline evidence are committed.
2. ✅ The specification and emission suites pass; the red record exists.
3. ✅ An eligible module-level `let` constant scalar array emits one
   extent-sized `static const` template and one `memcpy`, with no per-element
   stores.
4. ✅ Every reachable exclusion keeps its old spelling and behavior; every
   front-end-invalid shape is recorded.
5. ✅ The isolated and cumulative benchmark tables show the `lookup` win and no
   scored regression beyond 1%.
6. ✅ The emission audit and benchmark-source diff are attributed.
7. ✅ Every codegen-changing phase has the four reviews; the final focused
   re-review is clean.
8. ✅ `make check`, `make asan`, `make ubsan`, `make corpus-amiga`,
   `make differential`, `make bench-shape`, `make bench-compare`, and
   `make verify` are green.
9. ✅ The roadmap and plan index are current; the tree is clean and the branch
   is pushed.
