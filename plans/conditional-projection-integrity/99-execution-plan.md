# Execution Plan: Conditional and Projection Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-12 08:47
> **Progress**: 11/33 tasks (33%)
> **CodeOps Artifact Schema**: 1

## Overview

Close 14 defect rows (the 13-row conditional/projection cluster `DEF-176` – `DEF-188` plus the
scoping-found `DEF-256`) in six phases: conditional arm ownership, guarded receivers and loans,
view provenance and elision, statement-path discards, emission spellings, and close-out. Each
implementation phase follows the specification-first ordering (spec tests → red phase → implement →
green phase → impl tests → phase review; ASan probes and the corpus differential where the phase
touches ownership or emission) from [07-testing-strategy.md](07-testing-strategy.md).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Conditional arm ownership (DEF-176, DEF-178, DEF-179) | 6 |
| 2 | Guarded receivers and loans (DEF-180, DEF-181, DEF-182) | 6 |
| 3 | View provenance and elision (DEF-177, DEF-187) | 6 |
| 4 | Statement-path discards (DEF-185, DEF-186, DEF-256) | 6 |
| 5 | Emission spellings (DEF-183, DEF-184, DEF-188) | 6 |
| 6 | Close-out | 3 |

**Total: 33 tasks across 6 phases** (no fabricated hour estimates — scope is bounded by the
task-size criteria: each task is independently testable and reviewable).

> **⚠️ EXECUTION RULE — APPLIES TO EVERY PHASE**: follow the specification-first loop in order.
> Never implement before the phase's spec cases exist and are recorded red. On any green-run
> failure, STOP and fix the implementation — never weaken a test to pass.

---

## Phase 1: Conditional arm ownership

> **Phase baseline tree**: `0ae53eabf119f957d4b7b38083d94ce1781cbc88` (execution start: the batch H plan-creation tree plus the defect-verification ledger commit)
> **Lenses**: correctness + memory-safety
> **Reasoning**: high — two criticals plus the previously reverted whole-root attempt

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-conditional-arm-ownership.md)
**Objective**: Pin the three double-free shapes and the accepted arm controls.

- [x] 1.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_integrity_spec_test.rs` (armstore, armreturn, armtake, armdefer, armguard, armnested, armplace) ✅ (completed: 2026-10-12 01:24)
- [x] 1.1.2 Run the suite and record the red phase ✅ (completed: 2026-10-12 01:24)

### Step 1.2: Implementation

- [x] 1.2.1 Implement the per-arm move-outs and the guarded intrinsic argument — `compiler/src/own/lower.rs` (plus the checker-side projected-source marking in `own/statements.rs`, `own/expr.rs`, `own/call_check.rs`; runtime AR-15) ✅ (completed: 2026-10-12 01:33)
- [x] 1.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-12 01:33)

### Step 1.3: Impl Tests & Review

- [x] 1.3.1 Write the phase implementation pins (the move-out shapes, the guarded fill, the release counts) — `compiler/tests/conditional_projection_integrity_impl_test.rs` ✅ (completed: 2026-10-12 01:35)
- [x] 1.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the two lenses ✅ (completed: 2026-10-12 02:03; correctness + memory-safety found RV-001/RV-002/SA-001, the scoped re-review found RV-101/RV-102/RV-103/SA-102 and closed SA-001; all fixed and the targeted confirmation returned PASS; DEF-257–DEF-260 filed)

**Verify**: `make -C docker exec CMD="cargo test --test conditional_projection_integrity_spec_test --test conditional_projection_integrity_impl_test"`; `make check`

## Phase 2: Guarded receivers and loans

> **Phase baseline tree**: `e6f8848c8561c161dcb39c317b3a4bef804ec973` (Phase 1's final commit tree at `576b424`)
> **Lenses**: correctness + memory-safety + semantics
> **Reasoning**: medium — three routing extensions in the batch E binder machinery

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-guarded-receivers-and-loans.md)
**Objective**: Pin the struct-literal receiver leak, the place-arm loan, and the dot-`len` receiver leak.

- [x] 2.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_integrity_spec_test.rs` (guardliteral, placearmloan, dotlenreceiver, guardplace) ✅ (completed: 2026-10-12 08:39)
- [x] 2.1.2 Run the suite and record the red phase ✅ (completed: 2026-10-12 08:39; plain 37 passed / 2 failed on the two conditional loan cases; ASan 32 passed / 7 failed with 10-byte guarded-literal and 11-byte dot-`len` leaks)

### Step 2.2: Implementation

- [x] 2.2.1 Route the struct-literal receiver root, record the conditional place arm's shared loan, and extend the conditional gate to the dot-`len` receiver — `compiler/src/own/lower.rs`, `compiler/src/own/place.rs` (the `own/call.rs` receiver-extraction caller already routes through the guarded binder, so it needed no change) ✅ (completed: 2026-10-12 08:46; runtime AR-16 corrected the phase-1 armplace control to the named spelling's binding-granular loan)
- [x] 2.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-12 08:46; 39/39 plain and under ASan; `make check` exit 0)

### Step 2.3: Impl Tests & Review

- [x] 2.3.1 Write the phase implementation pins (the guarded fill shape, the loan report, the byte-identity controls) — `compiler/tests/conditional_projection_integrity_impl_test.rs` ✅ (completed: 2026-10-12 08:47; four new pins: the guarded literal fill and release, the conditional dot-`len` per-arm roots, the one-report loan span, and the `len(cond.v)` byte-identity)
- [ ] 2.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the three lenses

**Verify**: `make -C docker exec CMD="cargo test --test conditional_projection_integrity_spec_test --test conditional_projection_integrity_impl_test"`; `make check`

## Phase 3: View provenance and elision

> **Phase baseline tree**: to be recorded at Phase 2's close (= Phase 2's final commit tree)
> **Lenses**: correctness + memory-safety + semantics
> **Reasoning**: high — the third critical and the frozen elision reading

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-view-provenance-and-elision.md)
**Objective**: Pin the `str[N]` judgments, the derived-candidate rejection, and the accepted controls.

- [ ] 3.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_integrity_spec_test.rs` (viewarrayloan, viewarraytemp, viewarraycontrol, elisionderive, elisiontemp)
- [ ] 3.1.2 Run the suite and record the red phase

### Step 3.2: Implementation

- [ ] 3.2.1 Record the binding loan, judge array element derivations, and tie the elision judgment to the actual derivation — `compiler/src/own/place.rs`, `compiler/src/own/statements.rs`, `compiler/src/own/lower.rs`
- [ ] 3.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 3.3: Impl Tests & Review

- [ ] 3.3.1 Write the phase implementation pins (the report codes/spans/counts, the byte-identity controls) — `compiler/tests/conditional_projection_integrity_impl_test.rs`
- [ ] 3.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the three lenses

**Verify**: `make -C docker exec CMD="cargo test --test conditional_projection_integrity_spec_test --test conditional_projection_integrity_impl_test"`; `make check`

## Phase 4: Statement-path discards

> **Phase baseline tree**: to be recorded at Phase 3's close (= Phase 3's final commit tree)
> **Lenses**: correctness + memory-safety + emission-integrity
> **Reasoning**: medium — three changes in the discard lowering

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-statement-path-discards.md)
**Objective**: Pin the closure-call and conditional discards and the intrinsic dot-call emission.

- [ ] 4.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_integrity_spec_test.rs` (closurecalldiscard, conddiscard, dotlendiscard, discardcontrol)
- [ ] 4.1.2 Run the suite and record the red phase

### Step 4.2: Implementation

- [ ] 4.2.1 Fix the closure-call predicate, bind the conditional arm's result, and route the discarded intrinsic dot-call — `compiler/src/own/statements.rs`, `compiler/src/own/lower.rs`
- [ ] 4.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 4.3: Impl Tests & Review

- [ ] 4.3.1 Write the phase implementation pins (the statement temporaries, the call spelling, the byte-identity controls) — `compiler/tests/conditional_projection_integrity_impl_test.rs`
- [ ] 4.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the three lenses

**Verify**: `make -C docker exec CMD="cargo test --test conditional_projection_integrity_spec_test --test conditional_projection_integrity_impl_test"`; `make check`

## Phase 5: Emission spellings

> **Phase baseline tree**: to be recorded at Phase 4's close (= Phase 4's final commit tree)
> **Lenses**: correctness + emission-integrity + semantics
> **Reasoning**: medium — three emitter/lowering spellings

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-emission-spellings.md)
**Objective**: Pin the conditional return, the postfix pin, and the fixed-array address.

- [ ] 5.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_integrity_spec_test.rs` (condreturn, postfixseq, fixedarrayaddr, emissioncontrol)
- [ ] 5.1.2 Run the suite and record the red phase

### Step 5.2: Implementation

- [ ] 5.2.1 Unify the conditional arm spelling, pin the postfix update, and bind the call root before the fixed-array address — `compiler/src/codegen/expr.rs`, `compiler/src/own/lower.rs`
- [ ] 5.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 5.3: Impl Tests & Review

- [ ] 5.3.1 Write the phase implementation pins (the unified spelling, the update temporary, the bound root, the byte-identity controls) — `compiler/tests/conditional_projection_integrity_impl_test.rs`
- [ ] 5.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the three lenses

**Verify**: `make -C docker exec CMD="cargo test --test conditional_projection_integrity_spec_test --test conditional_projection_integrity_impl_test"`; `make check`

## Phase 6: Close-out

> **Phase baseline tree**: to be recorded at Phase 5's close (= Phase 5's final commit tree)
> **Lenses**: none new — the close-out verifies the reviewed phases

### Step 6.1: Final Verification & Documentation

**Reference**: [01-requirements.md](01-requirements.md) §Acceptance Criteria
**Objective**: Run the full acceptance, sync the docs and the roadmap, and close the plan.

- [ ] 6.1.1 Run the full verification: `make check`, `make verify`, the strict-build sweep, and the ASan suite; record the evidence
- [ ] 6.1.2 Update `AGENTS.md` (test naming map with the new suites) and sync the roadmap rows to Done with per-row evidence
- [ ] 6.1.3 Record the close-out section in the findings ledger and mark the plan Complete in `00-index.md`

**Verify**: `make check`; `make verify`

---

## Dependencies

```
Phase 1 (conditional arm ownership)   independent
Phase 2 (receivers + loans)           independent; shares own/lower.rs with Phase 1 — run after
Phase 3 (view provenance + elision)   independent; shares own/place.rs with Phase 2's loan work — run after
Phase 4 (statement-path discards)     independent; shares own/statements.rs with Phase 3's provenance work — run after
Phase 5 (emission spellings)          uses Phase 1's per-arm shape for DEF-183 — run after Phase 1
Phase 6 (close-out)                   after every phase
```

---

## Success Criteria

**Feature is complete when:**

1. ☐ All phases completed
2. ☐ All verification passing (`make check` per phase; `make verify` at close-out)
3. ☐ No warnings/errors; no spec or registry change
4. ☐ No dead code — no unused parameters, functions, modules
5. ☐ Memory safety proven — every filed double-free/leak/UAF shape is ASan-clean and every created
   owner releases exactly once
6. ☐ Documentation updated (`AGENTS.md` test map, the roadmap rows, the findings ledger)
7. ☐ Code reviewed per the quality profile (per-phase lenses, no open critical/major)
