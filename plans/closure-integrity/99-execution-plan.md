# Execution Plan: Closure Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-10 10:58
> **Progress**: 13/22 tasks (59%)
> **CodeOps Artifact Schema**: 1

## Overview

Three phases close the six closure/callable defects ([AR-1](00-ambiguity-register.md)): the
capture listing and validity gaps (DEF-92, DEF-93, DEF-94), the captured-owner integrity holes
(DEF-104, DEF-109), and the callable-bound rejection (DEF-110). Each phase follows the
specification-first ordering and cites its owning [03-XX](00-index.md) document and the ST-cases
in [07-testing-strategy.md](07-testing-strategy.md).

Execution commit mode: **auto-commit** (user instruction) — commit and push each verified task
via the git-commit skill in push mode.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Capture listing and validity (DEF-92, DEF-93, DEF-94) | 8 |
| 2 | Captured-owner integrity (DEF-104, DEF-109) | 7 |
| 3 | Callable bounds (DEF-110) | 7 |

**Total: 22 tasks across 3 phases** (no fabricated hour estimates — scope is bounded by the
task-size criteria in the make-plan quality checklist)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the Last Updated
>    stamp after EVERY task — never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is resumed
>    first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line.
>    The plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

---

## Phase 1: Capture listing and validity (DEF-92, DEF-93, DEF-94)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer + semantics reviewer (capture-rule changes; the report
> matrix in `AGENTS.md`, AR-15)
> **Reasoning**: high — language-rule judgments with new diagnostics

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-capture-listing-and-validity.md) §Proposed Changes ·
[AR-2, AR-4, AR-6, AR-8, AR-9, AR-12](00-ambiguity-register.md)
**Objective**: Land the authorized `E10406` row and golden, then pin the corrected and preserved
capture-list validity judgments before any implementation.

- [x] 1.1.1 [spec] Add the `E10406` registry row (`spec/14-diagnostics.md`) and the golden `spec/tests/compile-fail/E10406_module_capture.clo`, registering the code in the compile-fail driver (`compiler/tests/compile_fail_spec_test.rs`) — the authorized modification set (AR-2, AR-12) ✅ (completed: 2026-10-10 10:28)
- [x] 1.1.2 [spec-author] Write specification tests from ST-1 – ST-9 — `compiler/tests/closure_integrity_spec_test.rs` ✅ (completed: 2026-10-10 10:30)
- [x] 1.1.3 Run the spec suite and record the red phase (expected: ST-1, ST-3, ST-4, ST-6, ST-9 fail; ST-2, ST-5, ST-7, ST-8 pass as deliberate pins, each recorded with its justification) ✅ (completed: 2026-10-10 10:30)

**Red-phase evidence (2026-10-10 10:30)** — the new suite (9 tests) fails exactly as designed: 5
failed, 4 passed. ST-1 (the unlisted relay), ST-3 and ST-4 (the module-level entries), ST-6 (the
dotted copy-capture write reports `E10024`), and ST-9 (the golden) fail against the baseline;
ST-2 (the listed relay builds and runs — exit 3), ST-5 (a capture-less module read — exit 7),
ST-7 (the borrow-capture write — exit 2), and ST-8 (the copy-capture read — exit 1) pass as
deliberate pins. The compile-fail driver reports the same designed red state for the golden
(`E10406_module_capture.clo: no 'error E10406: ...' diagnostic line (exit Some(0))`), and
`./spec/check.sh` is green (27 files, 107 compile-fail tests).

**Deliverables**:
- The registry row and golden
- Specification suite with ST-1 – ST-9
- Red-phase evidence with per-pin justification

**Verify**: `make -C docker exec CMD="cargo test --test closure_integrity_spec_test"`; `./spec/check.sh`

---

### Step 1.2: Implementation

**Reference**: [03-01](03-01-capture-listing-and-validity.md) §Proposed Changes
**Objective**: Report module-level entries (`E10406`) and unreachable relayed entries (`E10405`),
and recognize the dotted capture root for the write rule.

- [x] 1.2.1 Add the capture-entry validity check (module-level bindings and the enclosing-lambda reachability rule) — `compiler/src/sema/expr.rs`, `compiler/src/sema/mod.rs` ✅ (completed: 2026-10-10 10:32)
- [x] 1.2.2 Return the first segment of any `Qualified` from `place_root_name`, so dotted targets reach the write and move rules — `compiler/src/sema/mod.rs` ✅ (completed: 2026-10-10 10:32)
- [x] 1.2.3 Run the spec suite and `./spec/check.sh` to green (ST-1 – ST-9 and the golden); on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-10 10:32)

**Green-phase evidence (2026-10-10 10:32)** — the suite passes 9/9 in the pinned container; the
compile-fail driver asserts the new golden green; `./spec/check.sh` is OK (27 files, 107
compile-fail tests); and `make check` is fully green (53 suites, zero failures), so no existing
capture case regresses — the relay rejection fires only for entries the enclosing environment
cannot carry, and the dotted root only reaches the two capture rules.

**Deliverables**:
- The capture-entry validity check and the dotted root
- Green spec suite and green frozen-suite check

**Verify**: `make -C docker exec CMD="cargo test --test closure_integrity_spec_test"`; `./spec/check.sh`

---

### Step 1.3: Implementation Tests & Hardening

**Reference**: [03-01](03-01-capture-listing-and-validity.md) §Testing Requirements
**Objective**: Pin the internal shapes and record the phase review evidence.

- [x] 1.3.1 Write the Phase 1 implementation pins (report count and span per shape, the three-level relay reporting once, the dotted-root resolution, byte-identical re-emission of the accepted relay) — `compiler/tests/closure_integrity_impl_test.rs` ✅ (completed: 2026-10-10 10:35)
- [x] 1.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ✅ (completed: 2026-10-10 10:41)

**Phase review evidence (2026-10-10 10:41)** — two independent reviews ran on the phase diff
(correctness reviewer, semantics reviewer; AR-15). The semantics review found the phase
conformant (no critical or major findings); the correctness review found one major: the `E10406`
predicate covered only `let`/`const`/imports, so function and type names in a capture list still
emitted the same environment-less call. The predicate was broadened to every module-level name
(the user-approved trigger text already reads "a module-level binding"; AR-8 amended with the
reopen evidence), the import spelling gained coverage in both suites, and the relay span pin and
the entry-loop comment were corrected. The one scoped re-review accepted the major remediation,
the comment fix, and the import fix; it rejected only the span pin's precision (now pinned at the
entry's column) and noted a stale doc sentence (fixed). No third review — the phase budget is one
re-review. Rulings are recorded in [05-findings.md](05-findings.md).

**Deliverables**:
- Implementation pins for Phase 1
- Review evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test closure_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test closure_integrity_impl_test"`; `make check`

---

## Phase 2: Captured-owner integrity (DEF-104, DEF-109)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer + memory-safety auditor (ownership and emission changes; AR-15)
> **Reasoning**: high — memory-safety consequences and a per-instance duty

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-captured-owner-integrity.md) §Proposed Changes · [AR-5, AR-7, AR-10, AR-11](00-ambiguity-register.md)
**Objective**: Pin the corrected return judgment and the release-once capture call before any
implementation.

- [x] 2.1.1 [spec-author] Write specification tests from ST-10 – ST-13 — `compiler/tests/closure_integrity_spec_test.rs` ✅ (completed: 2026-10-10 10:48)
- [x] 2.1.2 Run the spec suite and record the red phase (expected: ST-10 and ST-11 fail; ST-12 and ST-13 pass as deliberate pins — ST-13's leak is asserted by the separate ASan probe task, not the plain run) ✅ (completed: 2026-10-10 10:48)

**Red-phase evidence (2026-10-10 10:48)** — the extended suite (14 tests) fails exactly as
designed: 13 passed, 1 failed. ST-10 (the generic owning instance) fails — the declaration accepts
the returning copy and the instance double-frees; ST-11 (the concrete spelling) passes because the
existing E10401 rule already judges it (the plan's red list was conservative); ST-12 (the
duplicable instance — exit 42) and ST-13 (the called func-typed capture — exit 3 under the plain
build; its leak is asserted by the separate ASan probe) pass as deliberate pins.

**Deliverables**:
- Specification suite with ST-10 – ST-13
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test closure_integrity_spec_test"`

---

### Step 2.2: Implementation

**Reference**: [03-02](03-02-captured-owner-integrity.md) §Proposed Changes
**Objective**: Reject the captured-owner return (directly and per instance) and mask the capture
name so a called func-typed capture resolves to the capture.

- [x] 2.2.1 Judge the lambda-body return of a by-value capture in the E10401 rule, and record the generic case as a per-instance duty audited under substitution — `compiler/src/sema/expr.rs`, `compiler/src/sema/mod.rs`, `compiler/src/sema/call.rs`, `compiler/src/own/instance_check.rs` ✅ (completed: 2026-10-10 10:58)
- [x] 2.2.2 Route the capture declaration through the callee-name masking, correcting the stale comment — `compiler/src/own/lower.rs` ✅ (completed: 2026-10-10 10:58)
- [x] 2.2.3 Run the spec suite to green (ST-10 – ST-13); run the host ASan probes for the DEF-104 owning shape (rejected, no unit) and the DEF-109 shape (release-once); on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-10 10:58)

**Green-phase evidence (2026-10-10 10:58)** — the suite passes 14/14; the host ASan probes are
clean for both memory shapes (the owning instance is rejected before emission; the lender-named
capture shape runs with leak detection on at exit 1 with no report), and `make check` is fully
green (54 suites, `spec check: OK (27 files, 107 compile-fail tests)`).

**Implementation notes (mechanical corrections).** The duty's recording needed the sema output
plumbing (`sema/mod.rs`) and the take-argument sites (`sema/call.rs`); the fix landed in the
may-lend gate (`lowered_lend_source` in `own/lower.rs`) rather than the IR-spelling mask the
design first named — masking leaked the masked spelling into the emitted C, while the gate guard
keeps the capture's source spelling and stops the module lender from claiming the call (AR-10
amended). Two pre-existing tests that pinned the owning-instance acceptance
(`residual_defect_completion_{spec,impl}_test.rs`) were corrected to the user-ruled behavior; the
concrete `String` spelling already reported `E10401` at the baseline. A sibling leak found while
probing — a temporary argument to a func-value call leaks (name-independent) — is filed as
DEF-125.

**Deliverables**:
- The capture-return judgment and the masked capture spelling
- Green spec suite and clean ASan probes

**Verify**: `make -C docker exec CMD="cargo test --test closure_integrity_spec_test"`; the host ASan probes are the memory oracle (emit, `cc -std=c99 -g -fsanitize=address -no-pie`, run with leak detection; AR-13)

---

### Step 2.3: Implementation Tests & Hardening

**Reference**: [03-02](03-02-captured-owner-integrity.md) §Testing Requirements
**Objective**: Pin the internals and record the phase review evidence.

- [ ] 2.3.1 Write the Phase 2 implementation pins (one report per shape with the substituted instance first, the duty naming, the masked capture IR name with the source name kept for diagnostics, the release counts on ST-13's shape, byte-identical re-emission) — `compiler/tests/closure_integrity_impl_test.rs`
- [ ] 2.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode

**Deliverables**:
- Implementation pins for Phase 2
- Review evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test closure_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test closure_integrity_impl_test"`; `make check`

---

## Phase 3: Callable bounds (DEF-110)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer + semantics reviewer (a new diagnostic and a language-rule
> rejection; AR-15)
> **Reasoning**: medium — a bounded checker change verified by strict builds

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-callable-bounds.md) §Proposed Changes · [AR-3, AR-12](00-ambiguity-register.md)
**Objective**: Land the authorized `E10307` row and golden, then pin the rejection before any
implementation.

- [ ] 3.1.1 [spec] Add the `E10307` registry row (`spec/14-diagnostics.md`) and the golden `spec/tests/compile-fail/E10307_unbounded_call.clo`, registering the code in the compile-fail driver (`compiler/tests/compile_fail_spec_test.rs`) — the authorized modification set (AR-3, AR-12)
- [ ] 3.1.2 [spec-author] Write specification tests from ST-14 – ST-16 — `compiler/tests/closure_integrity_spec_test.rs`
- [ ] 3.1.3 Run the spec suite and `./spec/check.sh`; record the red phase (expected: ST-14 and ST-16 fail; ST-15 passes as the accepted control)

**Deliverables**:
- The registry row and golden
- Specification suite with ST-14 – ST-16
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test closure_integrity_spec_test"`; `./spec/check.sh`

---

### Step 3.2: Implementation

**Reference**: [03-03](03-03-callable-bounds.md) §Proposed Changes
**Objective**: Reject a call of an unbounded type-parameter value with one `E10307`.

- [ ] 3.2.1 Add the unbounded-parameter rejection to the callee typing — `compiler/src/sema/call.rs`
- [ ] 3.2.2 Run the spec suite and `./spec/check.sh` to green (ST-14 – ST-16 and the golden); on any failure STOP and fix the implementation, never the test

**Deliverables**:
- The checker change
- Green spec suite and green frozen-suite check

**Verify**: `make -C docker exec CMD="cargo test --test closure_integrity_spec_test"`; `./spec/check.sh`

---

### Step 3.3: Implementation Tests & Hardening

**Reference**: [03-03](03-03-callable-bounds.md) §Testing Requirements
**Objective**: Pin the internals and close the plan.

- [ ] 3.3.1 Write the Phase 3 implementation pins (one report per site, the span and the type-parameter name, the unchanged bounded path, no translation unit) — `compiler/tests/closure_integrity_impl_test.rs`
- [ ] 3.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode; run `make check` and the `make verify` legs for the close-out

**Deliverables**:
- Implementation pins for Phase 3
- Review evidence recorded; `make check` and `make verify` green

**Verify**: `make -C docker exec CMD="cargo test --test closure_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test closure_integrity_impl_test"`; `make check`; `make verify`

---

## Dependencies

```
Phase 1 (capture validation, sema)
    ↓  (shares sema/expr.rs and the spec suite)
Phase 2 (ownership duty + capture masking)
    ↓  (shares the spec suite; independent code surface)
Phase 3 (callable check, sema/call.rs)
```

Phases are independent in behavior; the sequence keeps the shared suite file and the sema edits
touched by one phase at a time, and Phase 1's spec tasks land the first new diagnostic.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All three phases completed (22/22 tasks)
2. ✅ All verification passing (`make check` per phase; `make verify` at close-out)
3. ✅ No warnings/errors (strict builds for every covered emission)
4. ✅ No dead code — no unused parameters, functions, or modules
5. ✅ Memory hardening — host ASan probes clean for the DEF-104/DEF-109 family
6. ✅ Documentation — the two authorized registry rows and goldens; plain-language comments updated
7. ✅ Code reviewed per the repository's phase-review dispatch
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
