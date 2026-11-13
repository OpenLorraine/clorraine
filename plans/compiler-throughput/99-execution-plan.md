# Execution Plan: Compiler Throughput (PLAN-38)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-13
> **Progress**: 25/25 tasks (100%)
> **CodeOps Artifact Schema**: 1

## Overview

Deliver issue #13 in five phases: establish the base and the red tests, add the
throughput harness, fix the measured quadratic in the emitter, measure the
result and run the gates and independent reviews, then close the record. The
technical specification is [03-design.md](03-design.md); the expected behavior
is [07-testing-strategy.md](07-testing-strategy.md).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 1 | Baseline and red tests | 5 |
| 2 | Throughput harness | 6 |
| 3 | Emitter index | 5 |
| 4 | Measurement and verification | 5 |
| 5 | Documentation and closeout | 4 |

**Total: 25 tasks across 5 phases**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes below are the single source of truth for progress. On
> implementation mark `[~]`; on verify pass mark `[x]`; on blocker mark `[!]`
> with the reason. Update the Progress header after every task. Timestamps
> come from `date '+%Y-%m-%d %H:%M'`.

---

## Phase 1: Baseline and Red Tests

> **Phase baseline**: `9c44537` with a clean tree.

### Step 1.1: Base evidence

- [x] 1.1.1 Assert the base commit and clean tree; record the release compiler identity and the absence of a throughput harness in [02-current-state.md](02-current-state.md)
- [x] 1.1.2 Build the base release `clc` and capture the corpus, benchmark, example, and generated baselines
- [x] 1.1.3 Instrument the base temporarily for stage timings and record parse/load/resolve/sema/own/codegen on the generated shapes

### Step 1.2: Red tests

- [x] 1.2.1 Add the harness unit cases HT-1–HT-18 to `tools/perf/tests/test_compile_time.py`
- [x] 1.2.2 Add the body-index case IT-1–IT-3 to `compiler/src/codegen.rs`
- [x] 1.2.3 Run the focused suites on the base and record the expected red failures in [05-findings.md](05-findings.md)

**Phase 1 gate**: the baseline table and stage attribution are recorded, and
the new tests fail for the expected reasons.

---

## Phase 2: Throughput Harness

> **Phase baseline**: the Phase 1 red record.
> **Lenses**: measurement integrity, path safety.

### Step 2.1: Harness and target

- [x] 2.1.1 Implement `tools/perf/compile_time.py` with the four workload classes and deterministic generated inputs
- [x] 2.1.2 Implement the fork/exec measurement, peak-RSS record, and JSON document
- [x] 2.1.3 Implement `compare` with the median/minimum metric and optional regression limit
- [x] 2.1.4 Add `make bench-compile` and the `AGENTS.md` / `benchmarks/README.md` entries
- [x] 2.1.5 Run `make bench-test` green
- [x] 2.1.6 Run `make bench-compile` end to end and inspect the document shape

**Phase 2 gate**: the harness tests pass and the document contains every
workload with timing, memory, and exit codes.

---

## Phase 3: Emitter Index

> **Phase baseline**: the Phase 2 harness.
> **Lenses**: correctness, emission integrity.

### Step 3.1: Bounded body lookup

- [x] 3.1.1 Add `body_index` and the `bodies` field to `compiler/src/codegen.rs`
- [x] 3.1.2 Replace the `emit_function` scan with the index lookup
- [x] 3.1.3 Replace the `param_c_names` scan with the index lookup
- [x] 3.1.4 Run the focused codegen unit test and the driver/codegen suites green
- [x] 3.1.5 Compare emitted C, diagnostics, and exit codes between base and head over every workload

**Phase 3 gate**: the index test passes and the emission audit reports zero
differences.

---

## Phase 4: Measurement and Verification

> **Phase baseline**: the Phase 3 head.
> **Lenses**: measurement integrity, correctness, memory safety.

### Step 4.1: Result and gates

- [x] 4.1.1 Re-measure the head with the committed harness and record the base/head tables
- [x] 4.1.2 Run the interleaved base/head checks for corpus, benchmarks, examples, generated shapes, and the driver
- [x] 4.1.3 Run `make check`, `make asan`, and `make ubsan`
- [x] 4.1.4 Run `make bench-shape`, `make bench-compare`, `make corpus-amiga`, and `make differential`
- [x] 4.1.5 Run the full `make verify` gate and dispatch the independent reviews; resolve every critical/major finding

**Phase 4 gate**: every gate is green, the emission audit is clean, and the
reviews have no critical/major findings.

---

## Phase 5: Documentation and Closeout

### Step 5.1: The record

- [x] 5.1.1 Write the final [05-findings.md](05-findings.md) with the baseline, hot passes, improvement, memory, test counts, reviews, and gate evidence
- [x] 5.1.2 Mark this plan and the roadmap row Done; update the roadmap progress count
- [x] 5.1.3 Close issue #13 with the delivered harness and measured result
- [x] 5.1.4 Commit and push the change set; confirm the working tree is clean

**Phase 5 gate**: the plan, roadmap, documentation, issue, and code agree with
the delivered gate evidence.
