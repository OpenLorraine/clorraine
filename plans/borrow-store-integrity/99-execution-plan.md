# Execution Plan: Borrow-Store Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-10 00:34
> **Progress**: 0/22 tasks (0%)
> **CodeOps Artifact Schema**: 1

## Overview

Three phases close the five batch defects ([AR-12](00-ambiguity-register.md)): the self-assign
rules (DEF-100, DEF-102), the out-slot return (DEF-101), and the view-store guards (DEF-105,
DEF-106). Each phase follows the specification-first ordering and cites its owning
[03-XX](00-index.md) document and the ST-cases in
[07-testing-strategy.md](07-testing-strategy.md).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Self-assign rules (DEF-100, DEF-102) | 8 |
| 2 | Out-slot return (DEF-101) | 6 |
| 3 | View-store guards (DEF-105, DEF-106) | 8 |

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

## Phase 1: Self-assign rules (DEF-100, DEF-102)

> **Phase baseline tree**: (recorded by the exec-plan skill at phase start)
> **Lenses**: correctness reviewer + memory-safety auditor + semantics reviewer (ownership and
> self-assign language rules; AGENTS.md dispatch matrix)
> **Reasoning**: high — ownership semantics on the lend-store path with memory-safety
> consequences

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-self-assign-rules.md) §Proposed Changes · [AR-2, AR-4, AR-7, AR-12](00-ambiguity-register.md)
**Objective**: Pin the corrected self-assign judgments and the preserved accepted shapes before
any implementation.

- [ ] 1.1.1 [spec-author] Write specification tests from ST-1 – ST-7 and the extension cases ST-23 – ST-27 — `compiler/tests/borrow_store_integrity_spec_test.rs`
- [ ] 1.1.2 Run the spec suite and record the red phase (expected: ST-1, ST-3, and ST-27 fail; ST-2, ST-4, ST-5, ST-6, ST-7 and the ST-23 – ST-26 pins pass as green pins)

**Deliverables**:
- Specification suite with ST-1 – ST-7 and ST-23 – ST-27
- Red-phase evidence with per-case justification for the passing pins

**Verify**: `make -C docker exec CMD="cargo test --test borrow_store_integrity_spec_test"`

---

### Step 1.2: Implementation

**Reference**: [03-01](03-01-self-assign-rules.md) §Proposed Changes
**Objective**: Record the parameter-typed lend self-assign site, audit it per instance, and
document the conservative runtime-index comparison (DEF-102, no semantic change).

- [ ] 1.2.1 [DEF-100] Add the `RequiresSelfAssign` record, its re-export, the monomorphization transfer (`require_self_assign`, mirroring `require_storable_slot`), and the recorder at the lend-store site — `compiler/src/ir/mono.rs`, `compiler/src/ir.rs`, `compiler/src/own.rs`, `compiler/src/own/expr.rs`
- [ ] 1.2.2 [DEF-100] Audit the duty per instance with nullability checked before the duplicable/view/unknown skips (`E10034` / `E10020` mapping), one report per site (the first offending instance in instantiation order) — `compiler/src/own/instance_check.rs`, `compiler/src/own.rs`
- [ ] 1.2.3 [DEF-102] State the conservative runtime-index comparison in the rule's documentation — `compiler/src/own/expr.rs`
- [ ] 1.2.4 Run the spec suite to green (ST-1 – ST-7 and ST-23 – ST-27) and the ASan probes for ST-2 and ST-6; on any failure STOP and fix the implementation, never the test

**Deliverables**:
- The source changes
- Green spec suite and clean ASan probes

**Verify**: `make -C docker exec CMD="cargo test --test borrow_store_integrity_spec_test"`; for the memory cases, the per-case host probes are the oracle (emit, `cc -std=c99 -g -fsanitize=address -no-pie`, run), with `make -C docker exec CMD="RDC_ASAN=1 cargo test --test borrow_store_integrity_spec_test"` as a non-gating convenience run (timing-flaky)

---

### Step 1.3: Implementation Tests & Hardening

**Reference**: [03-01](03-01-self-assign-rules.md) §Testing Requirements
**Objective**: Pin the internal shapes and run the phase verification.

- [ ] 1.3.1 Write implementation tests (the per-instance report shape — one report per site; the duty naming; byte-identical re-emission) — `compiler/tests/borrow_store_integrity_impl_test.rs`
- [ ] 1.3.2 Full phase verification — `make check`

**Deliverables**:
- Implementation suite for P1
- `make check` green

**Verify**: `make check`

---

## Phase 2: Out-slot return (DEF-101)

> **Phase baseline tree**: (recorded by the exec-plan skill at phase start)
> **Lenses**: correctness reviewer + memory-safety auditor + emission-integrity auditor (return
> emission change; AGENTS.md dispatch matrix)
> **Reasoning**: high — ownership transfer through an emitted return path

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-out-slot-return.md) §Proposed Changes · [AR-3, AR-8, AR-12](00-ambiguity-register.md)
**Objective**: Pin the move-out semantics and the unchanged guard rejections before any
implementation.

- [ ] 2.1.1 [spec-author] Write specification tests from ST-8 – ST-12 — `compiler/tests/borrow_store_integrity_spec_test.rs`
- [ ] 2.1.2 Run the spec suite and record the red phase (expected: ST-8 and ST-9 fail; ST-10 – ST-12 pass as green pins)

**Deliverables**:
- Specification clauses for P2
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test borrow_store_integrity_spec_test"`

---

### Step 2.2: Implementation

**Reference**: [03-02](03-02-out-slot-return.md) §Proposed Changes
**Objective**: The direct-return branch empties a caller-storage source through the existing
zeroing step.

- [ ] 2.2.1 [DEF-101] Detect the caller-storage source (`pointer_params`) in the direct-return branch and emit the block-local plus source-empty form — `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/function.rs`
- [ ] 2.2.2 Run the spec suite to green (ST-8 – ST-12) and the ASan probes for ST-8 and ST-9; on any failure STOP and fix the implementation, never the test

**Deliverables**:
- The return emission change
- Green spec suite and clean ASan probe

**Verify**: `make -C docker exec CMD="cargo test --test borrow_store_integrity_spec_test"`; the ST-8 and ST-9 host ASan probes are the memory oracle (the `RDC_ASAN=1` container run is a non-gating convenience)

---

### Step 2.3: Implementation Tests & Hardening

**Reference**: [03-02](03-02-out-slot-return.md) §Testing Requirements
**Objective**: Pin the emitted return shape and run the phase verification.

- [ ] 2.3.1 Write implementation tests (the `__cl_return` block-local, `(*v) = NULL;`, the unchanged direct emission for a view slot, byte-identical re-emission) — `compiler/tests/borrow_store_integrity_impl_test.rs`
- [ ] 2.3.2 Full phase verification — `make check`

**Deliverables**:
- Implementation suite for P2
- `make check` green

**Verify**: `make check`

---

## Phase 3: View-store guards (DEF-105, DEF-106)

> **Phase baseline tree**: (recorded by the exec-plan skill at phase start)
> **Lenses**: correctness reviewer + memory-safety auditor + emission-integrity auditor (checker
> rule plus IR statement and emission change; AGENTS.md dispatch matrix)
> **Reasoning**: high — language-rule change on stores plus a new IR statement and emission shape

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-view-store-guards.md) §Proposed Changes · [AR-5, AR-6, AR-11](00-ambiguity-register.md)
**Objective**: Pin the store null requirement, the bounds trap, and the unchanged acceptances
before any implementation.

- [ ] 3.1.1 [spec-author] Write specification tests from ST-13 – ST-22 and the extension cases ST-28 – ST-30 — `compiler/tests/borrow_store_integrity_spec_test.rs`
- [ ] 3.1.2 Run the spec suite and record the red phase (expected: ST-13, ST-14, ST-18, and ST-29 fail; the remaining cases pass as green pins)

**Deliverables**:
- Specification clauses for P3
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test borrow_store_integrity_spec_test"`

---

### Step 3.2: Implementation

**Reference**: [03-03](03-03-view-store-guards.md) §Proposed Changes
**Objective**: Add the store-target null requirement and the constant-index bounds trap.

- [ ] 3.2.1 [DEF-105] Walk every assignment and compound-assignment write target for the read path's nullable-base check (the update form is already covered by the operand's read walk; ST-28 pins it) — `compiler/src/own/expr.rs`
- [ ] 3.2.2 [DEF-106] Add the `Guards` statement (constructed only when a level's base has a dynamic bound), lower the store target's element selection into it for every constant-index store form (plain, compound, and update; row stores via the same assignment path), and emit the guard pass — `compiler/src/ir.rs`, `compiler/src/own/expr.rs`, `compiler/src/own/generics.rs`, `compiler/src/codegen/function.rs`
- [ ] 3.2.3 Run the spec suite to green (ST-13 – ST-22 and ST-28 – ST-30) and the ASan probes for ST-15, ST-18, and ST-20; on any failure STOP and fix the implementation, never the test

**Deliverables**:
- The checker walk and the `Guards` statement
- Green spec suite and clean ASan probes

**Verify**: `make -C docker exec CMD="cargo test --test borrow_store_integrity_spec_test"`; the per-case host ASan probes are the memory oracle (the `RDC_ASAN=1` container run is a non-gating convenience)

---

### Step 3.3: Implementation Tests & Hardening

**Reference**: [03-03](03-03-view-store-guards.md) §Testing Requirements
**Objective**: Pin the internal shapes, run the phase verification, and close the plan with the
full acceptance run.

- [ ] 3.3.1 Write implementation tests (the trap line spelled before a slice element store in the plain, compound, and update forms; no `Guards` statement for fixed-array-only targets; the unchanged fixed-array emission; the `E10010` store-site shape with one report for the update form; byte-identical re-emission) — `compiler/tests/borrow_store_integrity_impl_test.rs`
- [ ] 3.3.2 Full phase verification — `make check`
- [ ] 3.3.3 Full acceptance — `make verify`

**Deliverables**:
- Implementation suite for P3
- `make check` and `make verify` green

**Verify**: `make verify`

---

## Dependencies

```
Phase 1 (self-assign rules)
    ↓
Phase 2 (out-slot return)
    ↓
Phase 3 (view-store guards)
```

Phase 1 is independent of the others; Phases 2 and 3 are ordered for review focus (each phase's
review closes before the next opens).

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed (22/22 tasks)
2. ✅ All verification passing (`make check`, `make verify`)
3. ✅ No warnings/errors in the emitted C under the strict host flags
4. ✅ No dead code — no unused parameters, functions, or statements
5. ✅ Security hardened — memory safety probed under ASan for every accepted owning shape
6. ✅ Documentation updated (the conservative comparison note; plan close-out notes)
7. ✅ Code reviewed (per-phase dispatch, AR-13)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
