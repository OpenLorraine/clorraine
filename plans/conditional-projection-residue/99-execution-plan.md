# Execution Plan: Conditional, Projection, and Update Residue

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-15 06:52
> **Progress**: 7/39 tasks (18%)
> **CodeOps Artifact Schema**: 1

## Overview

Handle the 23 batch-H residue rows — 21 implemented closes and two incidental closures (`DEF-259`
at the planning baseline, `DEF-268` by the batch-H emission work; AR-23, AR-25) — in six phases
(arm roots and per-instance duty; view derivation and owners; escaping stores; guarded and
discarded literal positions; fixed-array arms; update order completion) and a close-out. Every fix
extends the batch-H machinery; every phase follows the specification-first ordering (spec tests →
red → implement → green → impl pins → review).

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

> **Phase baseline tree**: `9833e73917a43e1bd7750c1fcbf040f277025270` (at phase start, 2026-10-13)
> **Lenses**: correctness + memory-safety
> **Reasoning**: high — per-arm ownership in the freshly reviewed machinery (extend, never
> re-architect)

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-arm-roots-and-duty.md) · [07](07-testing-strategy.md) ST-1..ST-6 · AR-3, AR-4, AR-23
**Objective**: Pin the per-instance duty, the fresh-root sibling binding, the guarded take
carrier, and the struct-literal field root; keep the incidentally closed literal arm root (ST-3)
as the green control.

- [x] 1.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (armduty, armfresh, armliteral, guardtake, fieldroot, controls) ✅ (completed: 2026-10-13 23:29; eight cases authored on the new suite; the two packet errors the author found were corrected by the parent and re-verified — the fieldroot call now doubles its true arm, and the control program uses legal ownership; the literal-arm case is green at the head, so `DEF-259`'s probed spellings appear incidentally fixed — recorded for the review)
- [x] 1.1.2 Run the suite and record the red phase ✅ (completed: 2026-10-13 23:29; plain and ASan: armduty RED (heap-use-after-free), armfresh RED under ASan (11 B leak), guardtake and its loop RED (double-free), fieldroot RED under ASan (11 B leak) and its loop RED under ASan (22 B / 33 B serial), armliteral and the controls GREEN as designed)

### Step 1.2: Implementation

- [x] 1.2.1 Extend the per-instance arm duty, the fresh-root binding, the literal arm root, and the guarded field/argument roots — `compiler/src/own/lower.rs`, `compiler/src/own/expr.rs`, `compiler/src/own/call_check.rs` ✅ (completed: 2026-10-13 23:48; the generic arm projection routes through the per-arm move-out with the instance's type judged at emission and the checker's projected-source duty recorded (`own/statements.rs`, `own/expr.rs`, `own/call_check.rs`); a conditional-of-projections fresh root binds only when it still owns an un-taken field; the guarded take fills one shared carrier on the evaluated path with the post-call move-out emptying it; a struct-literal field's ownerless projection root binds in the deferred/guarded path, with the guard form skipping the reassigned drop for an adopting carrier)
- [x] 1.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-13 23:48; the eight phase cases green plain and under `RDC_ASAN=1`; `make check` exit 0 — 65 suites, no failures; the batch-H suites stay green)

### Step 1.3: Impl Tests & Review

- [x] 1.3.1 Write the phase implementation pins (the carrier/binding shapes, the release counts, the re-emission determinism pins) — `compiler/tests/conditional_projection_residue_impl_test.rs` ✅ (completed: 2026-10-15 06:36; seven pins on seven tests — the per-instance arm move, the residual-owning fresh root, the literal projection arm (a green control: base-identical), the guarded take carrier fill/empty, the receiver-field root inside the guard (the adopting-carrier drop skip), the direct spellings (a green control), and the re-emission determinism pin; 7/7 green in the container and against the full `make check`; the author measured four of the seven failing at the pre-fix compiler `744674b` on exactly the fix-only markers — the phase review re-checks two of them)
- [ ] 1.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses

**Verify**: `make check`; the ownership cases under `RDC_ASAN=1`

**Deliverables**:
- [ ] ST-1, ST-2, ST-4, ST-5 red then green; ST-3 and ST-6 green controls (the incidental closure — AR-23 — and the direct spellings)
- [ ] All verification passing

---

## Phase 2: View derivation and owners

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + semantics + memory-safety
> **Reasoning**: high — the derivation and owner rules are frozen; the fixes extend their walk

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-view-derivation-and-owners.md) · [07](07-testing-strategy.md) ST-7..ST-11, ST-28, ST-29 · AR-19, AR-25, AR-26
**Objective**: Pin the call-wrapped owner (every arm and the chained callee), the element
positions, the `str[N]` return, the one-report mixed return (ST-11), and the reversed-order green
control (ST-10).

- [x] 2.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (callwrappedowner, elementpositions, arrayreturn, onereport, reverseorder, ownertwins) ✅ (completed: 2026-10-15 06:48; six tests appended, additions-only — the call-wrapped owner (first/second/twin), the element positions (argument/field), the `str[N]` return (parameter/member-twin/member), the one-report case (mixed/local-parameter/local-only), the reversed-order green control, and the owner twins (chained/two-arm/direct); the committed phase-1 oracle and helpers are untouched; the author measured two design-word inaccuracies (recorded: `[h.v]` reports `E10034` + `E10036`; the two-arm walk records no arm's owner), and both design texts were corrected)
- [x] 2.1.2 Run the suite and record the red phase ✅ (completed: 2026-10-15 06:52; plain 10 passed / 4 failed and ASan 9 passed / 5 failed — callwrappedowner and the owner twins accepted-then-freed, the one-report case reporting two errors (`E10035` + `E10030`), the `str[N]` parameter case falsely rejected with `E10034`, and the element positions leaking 11 B per element under ASan; the reversed-order control and all ten program-level controls green — `E10031`/`E10036`/`E10035` counts as designed; parent-verified)

### Step 2.2: Implementation

- [ ] 2.2.1 Extend the owner walk through a view-returning call (every arm and the chained callee), the element positions, and the return's view-element lowering; suppress the local-owner `E10035` at a mixed return whose other arm's lowered view reads a compiler-generated root or a statement temporary (the one-report rule) — `compiler/src/own/place.rs`, `compiler/src/own/call_check.rs`, `compiler/src/own/statements.rs`, `compiler/src/own/lower.rs`
- [ ] 2.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 2.3: Impl Tests & Review

- [ ] 2.3.1 Write the phase implementation pins (the owner shapes, the element bindings, the report counts and spans) — `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 2.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses

**Verify**: `make check`; the ownership cases under `RDC_ASAN=1`

**Deliverables**:
- [ ] ST-7, ST-8, ST-9, ST-11, ST-28, ST-29 red then green; ST-10 a green control (the reversed order — AR-25)
- [ ] The batch-H suites and the `callable_release_integrity` oracle stay green

---

## Phase 3: Escaping stores

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + semantics + memory-safety + emission-integrity
> **Reasoning**: high — the escape rule is frozen; the over-rejection also changes acceptance
> and the store/capture shapes are emission-critical

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-escaping-stores.md) · [07](07-testing-strategy.md) ST-12..ST-15, ST-30..ST-33 · AR-5, AR-6, AR-20, AR-21
**Objective**: Pin the per-destination store judgment (globals/fields vs out-slots), the store and
capture spellings, and the scoped local element acceptance.

- [ ] 3.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (globalstore, outslotstore, fieldstore, localelement, widenedstore, loopstore, staticelement, viewcapture)
- [ ] 3.1.2 Run the suite and record the red phase

### Step 3.2: Implementation

- [ ] 3.2.1 Judge the stored view's provenance per destination kind (globals/fields `E10033`, out-slots `E10030`); lower the field store and the capture; scope the element-store loan to backings whose scope covers the array's — `compiler/src/own/statements.rs`, `compiler/src/own/expr.rs`, `compiler/src/own/place.rs`, `compiler/src/codegen/*`
- [ ] 3.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 3.3: Impl Tests & Review

- [ ] 3.3.1 Write the phase implementation pins (the report spans, the store/capture shapes, the re-emission determinism pins) — `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 3.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses

**Verify**: `make check`; the ownership cases under `RDC_ASAN=1`

**Deliverables**:
- [ ] ST-12..ST-15, ST-30..ST-33 red then green
- [ ] No accepted safe program newly rejected without a frozen-rule citation

---

## Phase 4: Guarded and discarded literal positions

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + memory-safety
> **Reasoning**: high — the discarded/guarded paths are the phase-4 fix territory of batch H

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-guarded-literal-positions.md) · [07](07-testing-strategy.md) ST-16..ST-18, ST-34 · AR-12, AR-22
**Objective**: Pin the loop-condition conditional literal (both literal forms), the `?:`/`||`
positions, the discarded mixed-field literal, and the discarded fresh projection arm.

- [ ] 4.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (loopcondliteral, literalpositions, discardmixed, discardprojection)
- [ ] 4.1.2 Run the suite and record the red phase

### Step 4.2: Implementation

- [ ] 4.2.1 Reuse the direct-literal per-arm binding for the loop-condition walk; make the discard decision per field (the place fields stay reads); bind the discarded arm's projection root — `compiler/src/own/lower.rs`
- [ ] 4.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 4.3: Impl Tests & Review

- [ ] 4.3.1 Write the phase implementation pins (the per-field shapes, the root binding, the release counts, the re-emission determinism pins) — `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 4.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses

**Verify**: `make check`; the ownership cases under `RDC_ASAN=1`

**Deliverables**:
- [ ] ST-16..ST-18, ST-34 red then green
- [ ] The batch-H discard pins stay green

---

## Phase 5: Fixed-array conditional arms and projections

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + emission-integrity + memory-safety
> **Reasoning**: high — the hidden out-parameter ABI and the address shapes are emission-critical

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-fixed-array-arms.md) · [07](07-testing-strategy.md) ST-19..ST-22, ST-35 · AR-11
**Objective**: Pin the fixed-array conditional arm, the mixed literal field, and the
conditional-root (a move-only root included)/`for`-iterable projections.

- [ ] 5.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (condarmfixed, mixedliteral, condroot, foriterable, moveonlyroot)
- [ ] 5.1.2 Run the suite and record the red phase

### Step 5.2: Implementation

- [ ] 5.2.1 Route each fixed-array arm through the per-arm destination store; bind each arm's root and render the conditional root as the per-arm address form; route the `for` iterable through the projection preparation — `compiler/src/own/lower.rs`, `compiler/src/own/loops.rs`, `compiler/src/codegen/guards.rs`, `compiler/src/codegen/function.rs`
- [ ] 5.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 5.3: Impl Tests & Review

- [ ] 5.3.1 Write the phase implementation pins (the per-arm destination shapes, the per-arm address form, the bound roots, the re-emission determinism pins) — `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 5.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses

**Verify**: `make check`; the ownership cases under `RDC_ASAN=1`

**Deliverables**:
- [ ] ST-19..ST-22, ST-35 red then green
- [ ] The emission differential shows only intended deltas

---

## Phase 6: Update order completion

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + emission-integrity + semantics
> **Reasoning**: high — the left-to-right rule's remaining constructs (spec/04 §3)

### Step 6.1: Specification Tests

**Reference**: [03-06](03-06-update-order.md) · [07](07-testing-strategy.md) ST-23..ST-27 · AR-9, AR-10, AR-24
**Objective**: Pin the assignment-target (the update-containing right-hand side included),
literal, dot-receiver, loop-condition, and owning-argument order cases.

- [ ] 6.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (assigntarget, literalorder, dotreceiver, loopcondupdate, owningarg)
- [ ] 6.1.2 Run the suite and record the red phase (the strict-build failures on the pinned container, the site/emission pins, and the order-observing case — AR-24)

### Step 6.2: Implementation

- [ ] 6.2.1 Extend the pin to the assignment targets (the update-containing right-hand side included), the literal fields/elements, the dot receiver, the loop condition (a per-iteration `Materialize` fill), and the owning argument — `compiler/src/own/lower.rs`, `compiler/src/own/call.rs`, `compiler/src/own/expr.rs`, `compiler/src/own/loops.rs`, `compiler/src/codegen/control.rs`
- [ ] 6.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 6.3: Impl Tests & Review

- [ ] 6.3.1 Write the phase implementation pins (the pin positions, the owning-argument shape, the re-emission determinism pins) — `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 6.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses

**Verify**: `make check`

**Deliverables**:
- [ ] ST-23..ST-27 red (the site/emission pins and the order-observing case — AR-24) then green
- [ ] The batch-H update pins stay green

---

## Phase 7: Close-out

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: none new (per the `AGENTS.md` dispatch matrix)
> **Reasoning**: low — verification and bookkeeping

### Step 7.1: Verification and Sync

- [ ] 7.1.1 Run the full verification: `make check`, `make verify`, the strict-build sweep, and the ASan suite; record the evidence
- [ ] 7.1.2 Update `AGENTS.md` (test naming map with the new suites) and sync the roadmap rows: 21 Done with per-row evidence and `DEF-259`/`DEF-268` closed with their incidental notes; the six older parked rows stay open (AR-2)
- [ ] 7.1.3 Record the close-out section in the findings ledger and mark the plan Complete in `00-index.md`

**Verify**: `make check`; `make verify`

**Deliverables**:
- [ ] The 23 handled rows accounted for: 21 Done with evidence; `DEF-259` and `DEF-268` closed with their incidental notes; the six older parked rows stay open with their rulings listed (AR-2)
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

Arrow legend: `A → B` marks a machinery dependency (B extends the binders, owner walk, or
preparation A touches); the vertical order is the executable sequence — the phases run
1 → 2 → 3 → 4 → 5 → 6 → 7 in order.

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
