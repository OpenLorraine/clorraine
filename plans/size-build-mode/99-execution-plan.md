# Execution Plan: Opt-in Size Build Mode (PLAN-37)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-13
> **Progress**: 24/24 tasks (100%)
> **CodeOps Artifact Schema**: 1

## Overview

Deliver issue #21 in five phases: capture the base and write the red tests,
implement the build-mode model and the `--size` surface, measure the
per-profile trade, run the full gate and independent review, then close the
record. The technical specification is [03-design.md](03-design.md); the
expected behavior is [07-testing-strategy.md](07-testing-strategy.md).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 1 | Baseline and red tests | 5 |
| 2 | Implementation | 6 |
| 3 | Measurement | 4 |
| 4 | Verification and review | 5 |
| 5 | Documentation and closeout | 4 |

**Total: 24 tasks across 5 phases**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes below are the single source of truth for progress. On
> implementation mark `[~]`; on verify pass mark `[x]`; on blocker mark `[!]`
> with the reason. Update the Progress header after every task. Timestamps
> come from `date '+%Y-%m-%d %H:%M'`.

---

## Phase 1: Baseline and Red Tests

> **Phase baseline**: `e65de1c` with a clean tree.

### Step 1.1: Base evidence

- [x] 1.1.1 Assert the base commit and clean tree; record the base help output and the `unknown option '--size'` error in [02-current-state.md](02-current-state.md)
- [x] 1.1.2 Capture the base debug and release command vectors with the toolchain unit suite

### Step 1.2: Red tests

- [x] 1.2.1 Add the CLI cases ST-1–ST-8 to `compiler/src/clorraine/cli.rs`
- [x] 1.2.2 Add the `BuildMode`/size-vector cases IT-1–IT-9 to `compiler/src/clorraine/toolchain.rs`
- [x] 1.2.3 Add the end-to-end cases ST-9–ST-14 to `compiler/tests/clorraine_driver_spec_test.rs`
- [x] 1.2.4 Run the focused suites on the base and record the expected red failures in [05-findings.md](05-findings.md)

**Phase 1 gate**: the new cases fail for the expected reasons and the base
vectors are recorded.

---

## Phase 2: Implementation

> **Phase baseline**: the Phase 1 red record.
> **Lenses**: correctness, CLI consistency.

### Step 2.1: Build-mode model and surface

- [x] 2.1.1 Add `BuildMode` and `Tool.size`; implement the `-Os` vectors per profile in `compiler/src/clorraine/toolchain.rs`
- [x] 2.1.2 Make `command()` select the mode slice and keep warnings debug-only
- [x] 2.1.3 Replace `Options.release` with `Options.mode`; parse `--size`, enforce the conflict, and reject it with `emit` in `compiler/src/clorraine/cli.rs`
- [x] 2.1.4 Thread the mode through `build.rs` and `mod.rs` for build, run, and the freestanding host-run leg
- [x] 2.1.5 Add the `--size` help line and update the `Options` doc comment
- [x] 2.1.6 Run the focused CLI, toolchain, and driver suites green

**Phase 2 gate**: every new case passes and the existing driver vectors are
byte-identical.

---

## Phase 3: Measurement

### Step 3.1: Per-profile comparison

- [x] 3.1.1 Build `calibration` and `strings` with `--release` and `--size` for `host`, `hosted-amiga`, `hosted-amiga-13`, and `freestanding`; record artifact bytes and m68k `.text`
- [x] 3.1.2 Verify the emitted C is byte-identical between the two modes for each program/profile
- [x] 3.1.3 Count dynamic instructions under `vamos --instr-trace` for the two programs on `hosted-amiga`; record host wall time as advisory
- [x] 3.1.4 Write the measurement table and the accepted trade in [05-findings.md](05-findings.md)

**Phase 3 gate**: the numbers show the size win and the `strings` runtime cost,
and the default `-O2` line is untouched.

---

## Phase 4: Verification and Review

> **Phase baseline**: the Phase 3 head.
> **Lenses**: correctness, CLI/documentation consistency.

### Step 4.1: Full gates

- [x] 4.1.1 Run `make check`
- [x] 4.1.2 Run `make asan` and `make ubsan`
- [x] 4.1.3 Run `make bench-shape`, `make bench-compare`, `make corpus-amiga`, and `make differential`
- [x] 4.1.4 Run the full `make verify` gate
- [x] 4.1.5 Dispatch an independent correctness review over the diff; resolve every critical/major finding

**Phase 4 gate**: every gate is green and the review is clean of
critical/major findings.

---

## Phase 5: Documentation and Closeout

### Step 5.1: The record

- [x] 5.1.1 Add the size-mode row to `docs/known-limitations.md` and the `--size` flag to the `AGENTS.md` driver row
- [x] 5.1.2 Write the final [05-findings.md](05-findings.md) with the measurements, test counts, review verdict, and gate evidence
- [x] 5.1.3 Mark this plan and the roadmap row Done; update the roadmap progress count
- [x] 5.1.4 Close issue #21 and commit/push the change set

**Phase 5 gate**: the plan, roadmap, known limitations, help text, and issue
agree with the delivered code and the gate evidence.
