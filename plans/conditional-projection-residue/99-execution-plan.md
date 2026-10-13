# Execution Plan: Conditional, Projection, and Update Residue

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-13 20:40
> **Progress**: 0/39 tasks (0%)
> **CodeOps Artifact Schema**: 1

## Overview

Close the 23 batch-H residue rows in six phases (arm roots and per-instance duty; view derivation
and owners; escaping stores; guarded and discarded literal positions; fixed-array arms; update
order completion) and a close-out. Every fix extends the batch-H machinery; every phase follows
the specification-first ordering (spec tests → red → implement → green → impl pins → review).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Arm roots and per-instance duty | 6 |
| 2 | View derivation and owners | 6 |
| 3 | Escaping stores | 6 |
| 4 | Guarded and discarded literal positions | 6 |
| 5 | Fixed-array conditional arms and projections | 6 |
| 6 | Update order completion | 6 |
| 7 | Close-out | 3 |

**Total: 39 tasks across 7 phases**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp.
> 2. **On verify pass:** promote it to `[x]` with a timestamp.
> 3. **Update the Progress header** and the Last Updated stamp after EVERY task.
> 4. **Resume** by scanning top-to-bottom: the first `[~]` first, else the first `[ ]`.
> 5. **On blocker:** mark `[!]` with a short reason.

---

## Phase 1: Arm roots and per-instance duty

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + memory-safety
> **Reasoning**: high — per-arm ownership in the freshly reviewed machinery (extend, never
> re-architect)

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-arm-roots-and-duty.md) · [07](07-testing-strategy.md) ST-1..ST-6 · AR-3, AR-4
**Objective**: Pin the per-instance duty, the fresh-root sibling binding, the literal arm root,
the guarded take carrier, and the struct-literal field root.

- [ ] 1.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (armduty, armfresh, armliteral, guardtake, fieldroot, controls)
- [ ] 1.1.2 Run the suite and record the red phase

### Step 1.2: Implementation

- [ ] 1.2.1 Extend the per-instance arm duty, the fresh-root binding, the literal arm root, and the guarded field/argument roots — `compiler/src/own/lower.rs`, `compiler/src/own/expr.rs`, `compiler/src/own/call_check.rs`
- [ ] 1.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 1.3: Impl Tests & Review

- [ ] 1.3.1 Write the phase implementation pins (the carrier/binding shapes, the release counts, the byte-identity controls) — `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 1.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses

**Verify**: `make check`

**Deliverables**:
- [ ] ST-1..ST-6 red then green
- [ ] All verification passing

---

## Phase 2: View derivation and owners

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + semantics + memory-safety
> **Reasoning**: high — the derivation and owner rules are frozen; the fixes extend their walk

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-view-derivation-and-owners.md) · [07](07-testing-strategy.md) ST-7..ST-11 · AR-7, AR-8
**Objective**: Pin the call-wrapped owner, the element positions, the `str[N]` return, the
disagreeing return, and the single escape report.

- [ ] 2.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (callwrappedowner, elementpositions, arrayreturn, condreturnjudge, escapereport)
- [ ] 2.1.2 Run the suite and record the red phase

### Step 2.2: Implementation

- [ ] 2.2.1 Extend the owner walk through a view-returning call, the element positions, the return's view-element lowering, the disagreeing-return judgment, and the report dedupe — `compiler/src/own/place.rs`, `compiler/src/own/call_check.rs`, `compiler/src/own/statements.rs`, `compiler/src/own/liveness.rs`
- [ ] 2.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 2.3: Impl Tests & Review

- [ ] 2.3.1 Write the phase implementation pins (the owner shapes, the element bindings, the report counts and spans) — `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 2.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses

**Verify**: `make check`

**Deliverables**:
- [ ] ST-7..ST-11 red then green
- [ ] The batch-H suites and the `callable_release_integrity` oracle stay green

---

## Phase 3: Escaping stores

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + semantics + memory-safety
> **Reasoning**: high — the escape rule is frozen; the over-rejection also changes acceptance

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-escaping-stores.md) · [07](07-testing-strategy.md) ST-12..ST-15 · AR-5, AR-6
**Objective**: Pin the escaping-store judgment (globals, fields, out-slots), the store and capture
spellings, and the local element acceptance.

- [ ] 3.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (globalstore, outslotstore, fieldstore, localelement)
- [ ] 3.1.2 Run the suite and record the red phase

### Step 3.2: Implementation

- [ ] 3.2.1 Judge the stored view's provenance against the frame it escapes; lower the field store and the capture; scope the element-store rule to escaping storage — `compiler/src/own/statements.rs`, `compiler/src/own/expr.rs`, `compiler/src/own/place.rs`, `compiler/src/codegen/*`
- [ ] 3.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 3.3: Impl Tests & Review

- [ ] 3.3.1 Write the phase implementation pins (the report spans, the store/capture shapes, the byte-identity controls) — `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 3.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses

**Verify**: `make check`

**Deliverables**:
- [ ] ST-12..ST-15 red then green
- [ ] No accepted safe program newly rejected without a frozen-rule citation

---

## Phase 4: Guarded and discarded literal positions

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + memory-safety
> **Reasoning**: high — the discarded/guarded paths are the phase-4 fix territory of batch H

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-guarded-literal-positions.md) · [07](07-testing-strategy.md) ST-16..ST-18 · AR-12
**Objective**: Pin the loop-condition conditional literal, the discarded mixed-field literal, and
the discarded fresh projection arm.

- [ ] 4.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (loopcondliteral, discardmixed, discardprojection)
- [ ] 4.1.2 Run the suite and record the red phase

### Step 4.2: Implementation

- [ ] 4.2.1 Reuse the direct-literal per-arm binding for the loop-condition walk; make the discard decision per field; bind the discarded arm's projection root — `compiler/src/own/lower.rs`
- [ ] 4.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 4.3: Impl Tests & Review

- [ ] 4.3.1 Write the phase implementation pins (the per-field shapes, the root binding, the release counts, the byte-identity controls) — `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 4.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses

**Verify**: `make check`

**Deliverables**:
- [ ] ST-16..ST-18 red then green
- [ ] The batch-H discard pins stay green

---

## Phase 5: Fixed-array conditional arms and projections

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + emission-integrity + memory-safety
> **Reasoning**: high — the hidden out-parameter ABI and the address shapes are emission-critical

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-fixed-array-arms.md) · [07](07-testing-strategy.md) ST-19..ST-22 · AR-11
**Objective**: Pin the fixed-array conditional arm, the mixed literal field, and the
conditional-root/`for`-iterable projections.

- [ ] 5.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (condarmfixed, mixedliteral, condroot, foriterable)
- [ ] 5.1.2 Run the suite and record the red phase

### Step 5.2: Implementation

- [ ] 5.2.1 Route each fixed-array arm through the destination-store shape; bind the conditional root and the `for` iterable through the projection preparation — `compiler/src/own/lower.rs`, `compiler/src/codegen/*`
- [ ] 5.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 5.3: Impl Tests & Review

- [ ] 5.3.1 Write the phase implementation pins (the destination shapes, the bound roots, the byte-identity controls) — `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 5.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses

**Verify**: `make check`

**Deliverables**:
- [ ] ST-19..ST-22 red then green
- [ ] The emission differential shows only intended deltas

---

## Phase 6: Update order completion

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + emission-integrity + semantics
> **Reasoning**: high — the left-to-right rule's remaining constructs (spec/04 §3)

### Step 6.1: Specification Tests

**Reference**: [03-06](03-06-update-order.md) · [07](07-testing-strategy.md) ST-23..ST-27 · AR-9, AR-10
**Objective**: Pin the assignment-target, literal, dot-receiver, loop-condition, and
owning-argument order cases.

- [ ] 6.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (assigntarget, literalorder, dotreceiver, loopcondupdate, owningarg)
- [ ] 6.1.2 Run the suite and record the red phase

### Step 6.2: Implementation

- [ ] 6.2.1 Extend the pin to the assignment targets, the literal fields/elements, the dot receiver, the loop condition, and the owning argument — `compiler/src/own/lower.rs`
- [ ] 6.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 6.3: Impl Tests & Review

- [ ] 6.3.1 Write the phase implementation pins (the pin positions, the owning-argument shape, the byte-identity controls) — `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 6.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses

**Verify**: `make check`

**Deliverables**:
- [ ] ST-23..ST-27 red then green
- [ ] The batch-H update pins stay green

---

## Phase 7: Close-out

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: none new (per the `AGENTS.md` dispatch matrix)
> **Reasoning**: low — verification and bookkeeping

### Step 7.1: Verification and Sync

- [ ] 7.1.1 Run the full verification: `make check`, `make verify`, the strict-build sweep, and the ASan suite; record the evidence
- [ ] 7.1.2 Update `AGENTS.md` (test naming map with the new suites) and sync the roadmap rows to Done with per-row evidence
- [ ] 7.1.3 Record the close-out section in the findings ledger and mark the plan Complete in `00-index.md`

**Verify**: `make check`; `make verify`

**Deliverables**:
- [ ] All 23 rows Done with evidence; the six parked rows stay open with their rulings listed
- [ ] The plan marked Complete

---

## Dependencies

```
Phase 1 (arm roots) → Phase 4 (guarded/discarded literals)
        ↓
Phase 2 (view derivation/owners) → Phase 3 (escaping stores)
        ↓
Phase 5 (fixed arrays) → Phase 6 (update order)
        ↓
Phase 7 (close-out)
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check`; `make verify` at close-out)
3. ✅ No warnings/errors
4. ✅ No dead code — no unused parameters, functions, classes, or modules
5. ✅ Security hardened — not applicable beyond the compiler's own input validation (unchanged)
6. ✅ Documentation updated (the `AGENTS.md` test map and the roadmap rows)
7. ✅ Code reviewed (every phase's declared lenses)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
