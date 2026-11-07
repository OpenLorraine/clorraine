# Execution Plan: Generated-C Optimization

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Created**: 2026-11-07
> **Last Updated**: 2026-11-07 12:45
> **Progress**: 30/30 tasks (100%)
> **CodeOps Artifact Schema**: 1

## Overview

Deliver the benchmark harness and suite first, record the committed baseline,
then implement the four measured codegen changes in dependency order:
constant folding, fill compaction, and the two guard simplifications. Finish
with the full acceptance matrix, the base/head emission diff, and independent
reviews.

**🚨 Update this document after EACH completed task!**

## Dependencies

```
0.1.1 requirements ──> 0.1.2 plan gate
0.1.2 ──> 1.1.1 harness tests ──> 1.1.2 harness ──> 1.1.3 suite
1.1.2 ──> 1.1.4 base mode ──> 1.1.6 baseline record
1.1.3 ──> 1.1.5 check integration
1.1.3 ──> 2.1.1/2.1.2/2.1.3 spec suite ──> 2.2.1 impl pins
2.2.1 ──> 3.1.* const ──> 4.1.* fill ──> 5.1.* guards
1.1.6 ──> 3.1.4/4.1.4/5.1.4 benchmark deltas
3.1.4, 4.1.4, 5.1.4 ──> 6.1.2 final compare
6.1.2, 6.1.3 ──> 6.1.4 full gates ──> 6.2.1 emission diff ──> 6.2.2 reviews
```

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 0 | Requirements and plan gates | 2 |
| 1 | Harness and benchmark suite | 6 |
| 2 | Behavior oracle and emission pins | 4 |
| 3 | Const folding | 4 |
| 4 | Fill compaction | 4 |
| 5 | Guard simplification | 4 |
| 6 | Documentation, acceptance, and review | 6 |

**Total: 30 tasks across 7 phases.**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes below are the **single source of truth** for progress.
> Every task line appears exactly once. On implementation, mark `[~]` with a
> timestamp; on verify pass, promote to `[x]` with a timestamp; update the
> Progress header after every task. On a blocker, mark `[!]` and append
> `Blocked: <reason>`. Timestamps come from `date '+%Y-%m-%d %H:%M'`.

---

## Phase 0: Requirements and Plan Gates

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — the requirements and plan gates own the scope, the
> address-taken safety rule, and the benchmark acceptance.
> **Lenses**: compiler semantics · emission integrity · measurement quality
> **Verify**: both preflight reports show every critical/major finding resolved.

### Step 0.1: Requirements and plan preflight

**Reference**: [Requirements](01-requirements.md) · [Ambiguity Register](00-ambiguity-register.md) · [Requirements Preflight](00-preflight-report-requirements.md) · [Plan Preflight](00-preflight-report.md)
**Objective**: Freeze the requirements and plan; resolve every critical/major finding.

- [x] 0.1.1 Write the requirements, ambiguity register, current-state study, and requirements preflight; resolve all 27 findings ✅ (completed: 2026-11-07 10:00)
- [x] 0.1.2 Write the benchmark-harness, generated-C, and testing-strategy designs and this execution plan; run the independent plan preflight; resolve every critical/major finding ✅ (completed: 2026-11-07 12:00)

**Deliverables**:
- [x] R1–R8, AR-1–AR-22, and the requirements preflight report
- [x] The plan document set and the plan preflight verdict

---

## Phase 1: Harness and Benchmark Suite

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — the harness is the measurement authority for every
> later phase; a wrong metric invalidates the whole programme.
> **Lenses**: measurement quality · tooling security · reproducibility
> **Verify**: `make bench-test` and `make bench-fast` pass in the pinned container.

### Step 1.1: Harness tests and implementation

**Reference**: [Benchmark Harness](03-01-benchmark-harness.md) §CLI, §Schema, §Compare, §Harness Self-Tests · AR-3, AR-4, AR-11, AR-21
**Objective**: Make the harness testable and green before it measures anything.

- [x] 1.1.1 [spec-author] Write the harness unit tests from 03-01 §Harness Self-Tests — `tools/perf/tests/test_bench.py`; run red ✅ (completed: 2026-11-07 04:09)
- [x] 1.1.2 Implement `run`, `compare`, and `test` in `tools/perf/bench.py`; add the `make bench*` targets; run `make bench-test` green ✅ (completed: 2026-11-07 04:09)
- [x] 1.1.3 Add the four scored benchmarks and the calibration, each with `program.clo`, `expected-output.txt`, and `README.md`; verify `make bench-fast` output and hashes ✅ (completed: 2026-11-07 04:09)
- [x] 1.1.4 Add `docker/scripts/bench-base.sh` and `make bench-base`; build the base compiler, retain `build/perf/base-clc`, and write `build/perf/base.json` ✅ (completed: 2026-11-07 04:09)
- [x] 1.1.5 Add `bench-test` to `make check`; run the target in the pinned container and record the result ✅ (completed: 2026-11-07 04:09)
- [x] 1.1.6 Record the committed base/head baseline, exact commands, and toolchain identity in `05-findings.md`; confirm R3.2 selection triggers ✅ (completed: 2026-11-07 04:09)

**Deliverables**:
- [ ] A committed harness, suite, and base/head path
- [ ] A green `make bench-test` and `make bench-fast`
- [ ] The committed baseline in `05-findings.md`

---

## Phase 2: Behavior Oracle and Emission Pins

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — the spec suite must pass on the base compiler, and the
> red impl pins must be recorded before implementation.
> **Lenses**: behavior preservation · test integrity · codebase grounding
> **Verify**: base spec suite green; the red/green pin list recorded.

### Step 2.1: Specification suite

**Reference**: [Testing Strategy](07-testing-strategy.md) ST-1–ST-41 · R8.1
**Objective**: Prove the base behavior before changing the emitter.

- [x] 2.1.1 [spec-author] Add the R4 const cases (ST-1–ST-11, ST-36–ST-39) to `compiler/tests/generated_c_optimization_spec_test.rs`; reference `RDC_ASAN` for the const cases; run green on the base compiler ✅ (completed: 2026-11-07 04:09)
- [x] 2.1.2 [spec-author] Add the R5 fill cases (ST-12–ST-20) to the same suite; reference `RDC_ASAN` for the fill cases; run green ✅ (completed: 2026-11-07 04:09)
- [x] 2.1.3 [spec-author] Add the R6/R7 guard cases (ST-21–ST-32, ST-40–ST-41) and the cross-cutting cases (ST-33–ST-35); run green ✅ (completed: 2026-11-07 04:09)

### Step 2.2: Implementation pins

**Reference**: [Testing Strategy](07-testing-strategy.md) IP-1–IP-19 · R8.2
**Objective**: Pin every shape and record the red/green base list.

- [x] 2.2.1 [spec-author] Add `compiler/tests/generated_c_optimization_impl_test.rs` with IP-1–IP-19; run it and record every pin's base status in `05-findings.md` ✅ (completed: 2026-11-07 04:09)

**Deliverables**:
- [ ] A green behavior oracle and a complete emission suite
- [ ] The red/green list recorded in `05-findings.md`

---

## Phase 3: Const Folding

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — the address-taken exclusion is a correctness boundary;
> the typed fold changes every function body that reads a scalar const.
> **Lenses**: compiler semantics · memory safety · emission integrity
> **Verify**: R4 spec suite and IP-1/IP-11/IP-12/IP-17/IP-18/IP-19/IP-20 green; sieve delta recorded.

### Step 3.1: Address-taken pre-pass and typed fold

**Reference**: [Generated-C Changes](03-02-generated-c-changes.md) §R4 · AR-7, AR-18, AR-19
**Objective**: Fold scalar const reads safely and preserve the exposed-storage behavior.

- [x] 3.1.1 Add the exhaustive address-taken walker and `GlobalInfo.const_init`; run the nested-address and shadowing cases ✅ (completed: 2026-11-07 04:09)
- [x] 3.1.2 Implement the scope-gated typed fold, declaring-module folder, and typed literal construction (negative, wrap/shift, finite `f32`, `bool`) ✅ (completed: 2026-11-07 04:09)
- [x] 3.1.3 Run the R4 spec suite and IP-1/IP-11/IP-12/IP-17/IP-18/IP-19; record the green result ✅ (completed: 2026-11-07 04:09)
- [x] 3.1.4 Run `make bench` and `make bench-compare` for the sieve delta; record the base/head values in `05-findings.md` ✅ (completed: 2026-11-07 04:09)

**Deliverables**:
- [ ] R4 implemented with the address-taken guard
- [ ] The sieve instruction delta recorded

---

## Phase 4: Fill Compaction

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — the fill touches parameter and return-slot byte counts;
> a wrong size is a buffer overrun.
> **Lenses**: memory safety · emission integrity · portability
> **Verify**: R5 spec suite and IP-2–IP-6/IP-14/IP-15/IP-16 green; `.text`/C delta recorded.

### Step 4.1: Compact scalar fills

**Reference**: [Generated-C Changes](03-02-generated-c-changes.md) §R5 · R5, AR-6
**Objective**: Replace the large per-element fill with one loop or `memset`.

- [x] 4.1.1 Add the threshold, the `compact` flag, and the `u8`/`i8` `memset` plus generic scalar loop for an array object ✅ (completed: 2026-11-07 04:09)
- [x] 4.1.2 Handle the adjusted parameter and hidden return slot with the element-count byte size; pass `compact = false` from the materialized caller ✅ (completed: 2026-11-07 04:09)
- [x] 4.1.3 Run the R5 spec suite and IP-2–IP-6/IP-14/IP-15/IP-16; record the green result ✅ (completed: 2026-11-07 04:09)
- [x] 4.1.4 Run `make bench` and `make bench-compare` for `.text` and C bytes; record the sieve delta in `05-findings.md` ✅ (completed: 2026-11-07 04:09)

**Deliverables**:
- [ ] R5 implemented for one-extent scalar statement fills
- [ ] The `.text` and C-size deltas recorded

---

## Phase 5: Guard Simplification

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: medium — two local dead-condition removals with trap
> preservation; the benchmark must confirm no regression.
> **Lenses**: compiler semantics · trap preservation · performance
> **Verify**: R6/R7 spec suite, fault suite, and IP-7–IP-10 green; no benchmark regression.

### Step 5.1: Slice/string and division guards

**Reference**: [Generated-C Changes](03-02-generated-c-changes.md) §R6, §R7 · R6, R7, AR-8, AR-9
**Objective**: Remove only the conditions the source semantics exclude.

- [x] 5.1.1 Implement the non-negative slice/string constant guard with the restricted constant helper; run IP-7/IP-8 ✅ (completed: 2026-11-07 04:09)
- [x] 5.1.2 Implement the constant-divisor guard with the restricted constant helper; run IP-9/IP-10 and the wrapping-divisor case ✅ (completed: 2026-11-07 04:09)
- [x] 5.1.3 Run the R6/R7 spec suite, the existing fault suite, and the impl pins; record green ✅ (completed: 2026-11-07 04:09)
- [x] 5.1.4 Run `make bench` and `make bench-compare`; confirm no instruction, `.text`, or `c_bytes` regression ✅ (completed: 2026-11-07 04:09)

**Deliverables**:
- [ ] R6/R7 implemented with trap preservation
- [ ] The no-regression evidence recorded

---

## Phase 6: Documentation, Acceptance, and Review

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Reasoning**: high — the feature changes emitted C for every profile; the
> full acceptance matrix, the emission diff, and independent reviews are the
> release gate.
> **Lenses**: emission integrity · memory safety · performance · semantics
> **Verify**: all named gates green, the emission diff attributed, and no critical/major review finding open.

### Step 6.1: Documentation and acceptance

**Reference**: R8.3–R8.7 · AGENTS.md phase-review dispatch
**Objective**: Make the work inspectable, reproducible, and green.

- [x] 6.1.1 Update `AGENTS.md` commands/layout, `benchmarks/README.md`, the plan docs, `05-findings.md`, `docs/known-limitations.md`, the production-readiness register, the AR-22 struct-size record, and `plans/00-roadmap.md` ✅ (completed: 2026-11-07 04:09)
- [x] 6.1.2 Run the full benchmark base/head compare and write the evidence table; confirm the R4.8/R5.7 sieve targets ✅ (completed: 2026-11-07 04:09)
- [x] 6.1.3 Add and run a focused vbcc build-only and freestanding compile leg for the new shapes ✅ (completed: 2026-11-07 04:09)
- [x] 6.1.4 Run `make check`, `make asan`, `make corpus-amiga`, `make differential`, and `make verify`; fix every failure ✅ (completed: 2026-11-07 12:45)

### Step 6.2: Emission diff and independent review

**Reference**: R8.5/R8.6 · AGENTS.md review packet rules
**Objective**: Resolve every critical/major finding before the final commit.

- [x] 6.2.1 Emit the corpus with `build/perf/base-clc` and the head `clc`; diff and attribute every change to R4–R7 ✅ (completed: 2026-11-07 04:09)
- [x] 6.2.2 Dispatch the correctness reviewer, memory-safety auditor, emission-integrity auditor, performance auditor, and semantics reviewer; resolve every critical/major finding and re-run the affected gate ✅ (completed: 2026-11-07 12:09)

**Deliverables**:
- [x] The documentation and roadmap updated
- [x] The full acceptance matrix green
- [x] The emission diff attributed
- [x] The review findings resolved and the final commit pushed via the git-commit skill in push mode

---

## Success Criteria

The plan is done when every deliverable is checked and:

- [x] R1–R8 are implemented and traced to a passing test or gate.
- [x] The final `make bench-compare` passes the generic 1% rows and the
      R4.8/R5.7 sieve targets.
- [x] `make check`, `make asan`, `make corpus-amiga`, `make differential`, and
      `make verify` are green.
- [x] The base/head corpus emission diff is attributed in `05-findings.md`.
- [x] No critical or major review finding remains open.
- [x] The roadmap row is Done and the final commit is pushed.
