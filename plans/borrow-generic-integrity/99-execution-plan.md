# Execution Plan: Borrow/Generic Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-09 20:23
> **Progress**: 0/29 tasks (0%)
> **CodeOps Artifact Schema**: 1

## Overview

Four phases close the six batch defects ([AR-11](00-ambiguity-register.md)): the borrow-store
rules (DEF-78, DEF-82, DEF-83), the instance keys (DEF-79), the nullable view comparison
(DEF-81), and the may-lend callee identity (DEF-77). Each phase follows the specification-first
ordering and cites its owning [03-XX](00-index.md) document and the ST-cases in
[07-testing-strategy.md](07-testing-strategy.md).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Borrow-store rules (DEF-78, DEF-82, DEF-83) | 9 |
| 2 | Instance keys (DEF-79) | 7 |
| 3 | Nullable view comparison (DEF-81) | 6 |
| 4 | May-lend callee identity (DEF-77) | 7 |

**Total: 29 tasks across 4 phases** (no fabricated hour estimates — scope is bounded by the
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

## Phase 1: Borrow-store rules (DEF-78, DEF-82, DEF-83)

> **Phase baseline tree**: _(recorded by the exec-plan skill from a temporary-index snapshot of
> committed, staged, unstaged, and untracked phase-start state)_
> **Lenses**: correctness reviewer + memory-safety auditor + semantics reviewer (ownership/lend
> store rules; AGENTS.md dispatch matrix)
> **Reasoning**: high — ownership and memory-safety semantics on an assignment path

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-borrow-store-rules.md) §Proposed Changes · [AR-2, AR-5, AR-8, AR-9, AR-13](00-ambiguity-register.md)
**Objective**: Pin the corrected store rules and the preserved legal shapes before any
implementation.

- [ ] 1.1.1 [spec-author] Write specification tests from ST-1 – ST-8 — `compiler/tests/borrow_generic_integrity_spec_test.rs`
- [ ] 1.1.2 Run the spec suite and record the red phase (expected: ST-1 – ST-4, ST-7, ST-8 fail; ST-5, ST-6 pass as legal-shape regression pins)

**Deliverables**:
- Specification suite with ST-1 – ST-8
- Red-phase evidence with per-case justification for the passing pins

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 1.2: Implementation

**Reference**: [03-01](03-01-borrow-store-rules.md) §Proposed Changes
**Objective**: Implement the target-side store rule, the duty audit, the out-slot self-assign
rejection, and the warning exemption.

- [ ] 1.2.1 [DEF-78] Record `RequiresStorableSlot` and reject the concrete owning store — `compiler/src/ir/mono.rs`, `compiler/src/own/expr.rs`
- [ ] 1.2.2 [DEF-78] Audit the duty per instance — `compiler/src/own/instance_check.rs`
- [ ] 1.2.3 [DEF-82] Reject the whole-binding out-slot self-assignment — `compiler/src/own/expr.rs`
- [ ] 1.2.4 [DEF-83] Exempt nullable borrow parameters from `W10001` — `compiler/src/own/call_check.rs`
- [ ] 1.2.5 Run the spec suite to green (ST-1 – ST-8) and the host-ASan probes for ST-6; on any failure STOP and fix the implementation, never the test

**Deliverables**:
- The four source changes
- Green spec suite and clean ASan probes

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 1.3: Implementation Tests & Hardening

**Reference**: [03-01](03-01-borrow-store-rules.md) §Testing Requirements
**Objective**: Pin the internal shapes and run the phase verification.

- [ ] 1.3.1 Write implementation tests (emitted out-slot store shapes, one-report counts, duty dedupe) — `compiler/tests/borrow_generic_integrity_impl_test.rs`
- [ ] 1.3.2 Full phase verification — `make check`

**Deliverables**:
- Implementation suite for P1
- `make check` green

**Verify**: `make check`

---

## Phase 2: Instance keys (DEF-79)

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: correctness reviewer + memory-safety auditor + emission-integrity auditor
> (symbol-name and emission change; AGENTS.md dispatch matrix)
> **Reasoning**: high — C symbol/ABI spellings across the corpus

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-instance-keys.md) §Proposed Changes · [AR-4, AR-6](00-ambiguity-register.md)
**Objective**: Pin the distinct-instance behavior, the unchanged function-instance spellings, and
determinism.

- [ ] 2.1.1 [spec-author] Write specification tests from ST-9 – ST-12 — `compiler/tests/borrow_generic_integrity_spec_test.rs`
- [ ] 2.1.2 Run the spec suite and record the red phase (expected: ST-9, ST-11 fail on the redefinition; ST-10, ST-12 pass as pins)

**Deliverables**:
- Specification cases ST-9 – ST-12 added
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 2.2: Implementation

**Reference**: [03-02](03-02-instance-keys.md) §Proposed Changes
**Objective**: Fold nullability into the key and remove the wrapper.

- [ ] 2.2.1 Fold nullability into `type_key`; preserve the text-conversion spelling — `compiler/src/types/keys.rs`, `compiler/src/codegen/expr.rs`
- [ ] 2.2.2 Remove `instance_arg_key`; point its call sites at `type_key` — `compiler/src/codegen/types.rs`, `compiler/src/codegen.rs`, `compiler/src/codegen/expr.rs`
- [ ] 2.2.3 Run the spec suite to green (ST-9 – ST-12); on any failure STOP and fix the implementation

**Deliverables**:
- The key change and the wrapper removal
- Green spec suite

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 2.3: Implementation Tests & Hardening

**Reference**: [03-02](03-02-instance-keys.md) §Testing Requirements
**Objective**: Pin the symbol spellings and byte stability; hand the emission sweep to the phase
review.

- [ ] 2.3.1 Write implementation tests (typedef and symbol names, function-instance byte pins, re-emission) — `compiler/tests/borrow_generic_integrity_impl_test.rs`
- [ ] 2.3.2 Full phase verification — `make check`

**Deliverables**:
- Implementation suite for P2
- `make check` green
- The emission sweep runs in the phase review (base↔head, expected diffs only)

**Verify**: `make check`

---

## Phase 3: Nullable view comparison (DEF-81)

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: correctness reviewer + memory-safety auditor + semantics reviewer (operator rule;
> AGENTS.md dispatch matrix)
> **Reasoning**: medium — one sema rule with an existing emitter path; bounded

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-nullable-view-comparison.md) §Proposed Changes · [AR-7](00-ambiguity-register.md)
**Objective**: Pin the acceptance, the narrowing run, and the preserved rejections.

- [ ] 3.1.1 [spec-author] Write specification tests from ST-13 – ST-16 — `compiler/tests/borrow_generic_integrity_spec_test.rs`
- [ ] 3.1.2 Run the spec suite and record the red phase (expected: ST-13, ST-14, ST-16 fail; ST-15 passes as the preserved-rejection pin)

**Deliverables**:
- Specification cases ST-13 – ST-16 added
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 3.2: Implementation

**Reference**: [03-03](03-03-nullable-view-comparison.md) §Proposed Changes
**Objective**: Accept the view-versus-`null` pairing in the equality arm.

- [ ] 3.2.1 Implement the pairing rule — `compiler/src/sema/ops.rs`
- [ ] 3.2.2 Run the spec suite to green (ST-13 – ST-16); on any failure STOP and fix the implementation

**Deliverables**:
- The sema rule change
- Green spec suite

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 3.3: Implementation Tests & Hardening

**Reference**: [03-03](03-03-nullable-view-comparison.md) §Testing Requirements
**Objective**: Pin the emitted `.ptr` comparison and the preserved diagnostic.

- [ ] 3.3.1 Write implementation tests (emitted comparison shape, E10222 for view-versus-view) — `compiler/tests/borrow_generic_integrity_impl_test.rs`
- [ ] 3.3.2 Full phase verification — `make check`

**Deliverables**:
- Implementation suite for P3
- `make check` green

**Verify**: `make check`

---

## Phase 4: May-lend callee identity (DEF-77)

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: correctness reviewer + memory-safety auditor + semantics reviewer (borrow gate and
> emitted names; AGENTS.md dispatch matrix)
> **Reasoning**: high — ownership binding semantics; the leak oracle is ASan

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-may-lend-callee-identity.md) §Proposed Changes · [AR-3](00-ambiguity-register.md)
**Objective**: Pin the leak repro and the unchanged sibling behaviors.

- [ ] 4.1.1 [spec-author] Write specification tests from ST-17 – ST-20 — `compiler/tests/borrow_generic_integrity_spec_test.rs`
- [ ] 4.1.2 Run the spec suite and record the red phase (expected: ST-17 fails under ASan; ST-18 – ST-20 pass as pins)

**Deliverables**:
- Specification cases ST-17 – ST-20 added
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 4.2: Implementation

**Reference**: [03-04](03-04-may-lend-callee-identity.md) §Proposed Changes
**Objective**: Extend the callee-name masking guard to every binding kind.

- [ ] 4.2.1 Extend the mask condition in `unique_ir_name` — `compiler/src/own.rs`
- [ ] 4.2.2 Run the spec suite to green (ST-17 – ST-20) and the ST-17 ASan probe; on any failure STOP and fix the implementation

**Deliverables**:
- The guard change
- Green spec suite and clean ASan probe

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 4.3: Implementation Tests & Hardening

**Reference**: [03-04](03-04-may-lend-callee-identity.md) §Testing Requirements
**Objective**: Pin the suffixed names, close the phase, and run the final acceptance.

- [ ] 4.3.1 Write implementation tests (suffixed IR/emitted names for colliding bindings, unchanged module-call emission) — `compiler/tests/borrow_generic_integrity_impl_test.rs`
- [ ] 4.3.2 Full phase verification — `make check`
- [ ] 4.3.3 Final acceptance — `make verify` (all legs)

**Deliverables**:
- Implementation suite for P4
- `make check` and `make verify` green

**Verify**: `make verify`

---

## Dependencies

```
Phase 1 (borrow-store rules)
    ↓
Phase 2 (instance keys)
    ↓
Phase 3 (nullable view comparison)
    ↓
Phase 4 (may-lend callee identity)
```

The phases are independent in code; the order runs from the memory-safety-heavy store rules to
the isolated name/operator fixes. Within Phase 1, tasks 1.2.1 → 1.2.2 are sequential; 1.2.3 and
1.2.4 are independent of each other and of 1.2.1/1.2.2.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check` per phase; `make verify` final)
3. ✅ The six verified repros of [02-current-state.md](02-current-state.md) behave per R1 – R9
4. ✅ No warnings/errors; no change under `spec/`
5. ✅ Host-ASan probes clean for the P1 and P4 shapes
6. ✅ No dead code — `instance_arg_key` is removed rather than left unused
7. ✅ Code reviewed per the repo's strict quality profile (independent phase reviews per AGENTS.md)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
