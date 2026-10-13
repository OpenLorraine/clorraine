# Execution Plan: Conditional and Projection Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-13 20:26
> **Progress**: 33/33 tasks (100%)
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
- [x] 2.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the three lenses ✅ (completed: 2026-10-12 09:11; correctness + memory-safety + semantics found SA-201/RV-201/RV-202/SA-202/SA-203; SA-201 and RV-201 fixed in the follow-up commit and closed by the scoped re-review (correctness PASS, SA-201 closed); SA-301 filed as DEF-264 with DEF-261–263; semantics reported no findings)

**Verify**: `make -C docker exec CMD="cargo test --test conditional_projection_integrity_spec_test --test conditional_projection_integrity_impl_test"`; `make check`

## Phase 3: View provenance and elision

> **Phase baseline tree**: `8de8e082855c075053a70cd82dab4af59f2d08e7` (Phase 2's final commit tree at `01c2481`)
> **Lenses**: correctness + memory-safety + semantics
> **Reasoning**: high — the third critical and the frozen elision reading

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-view-provenance-and-elision.md)
**Objective**: Pin the `str[N]` judgments, the derived-candidate rejection, and the accepted controls.

- [x] 3.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_integrity_spec_test.rs` (viewarrayloan, viewarraytemp, viewarraycontrol, elisionderive, elisiontemp) ✅ (completed: 2026-10-13 11:14; five new cases: the loan parity and its single-view twin, the adopted temporary backing with the loop spelling, the static and parameter controls, the derivation judgment with its twin and control, the caller-temporary declaration rejection)
- [x] 3.1.2 Run the suite and record the red phase ✅ (completed: 2026-10-13 11:14; plain 42/45 — the three rejection cases red with the filed symptoms (E10025 instead of E10022; the accepted `pick`; the accepted caller temporary); ASan 41/45 — plus the loop-backed temporary binding red with 22 B leaked in 2 allocations)

### Step 3.2: Implementation

- [x] 3.2.1 Record the binding loan, judge array element derivations, and tie the elision judgment to the actual derivation — `compiler/src/own/place.rs`, `compiler/src/own/statements.rs`, `compiler/src/own/lower.rs` ✅ (completed: 2026-10-13 11:25; the view-element binder sweeps (`view_loans`, `collect_view_owners`, the lowered-root collection), the binding-side gates (`is_view_like` for the effect and the view binder), the in-place element lowering (`view_array_literal_depth` across the eval, the store preparation, and the lowered lambda boundary; `own/expr.rs` and `own.rs` carry the flag), and the elision derivation comparison (`elision_candidate`/`return_derivation_mismatch`); the derivation check flags a named non-candidate parameter only, so the intrinsic accessor chains of the standard library keep their acceptance)
- [x] 3.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-13 11:25; spec 45/45 plain and under `RDC_ASAN=1`; impl 11/11; `make check` exit 0 — 64 binaries, 1854 tests)

### Step 3.3: Impl Tests & Review

- [x] 3.3.1 Write the phase implementation pins (the report codes/spans/counts, the byte-identity controls) — `compiler/tests/conditional_projection_integrity_impl_test.rs` ✅ (completed: 2026-10-13 11:27; four new pins: the adopted scoped owner with the element view and its release, the one-report loan at the move span, the declaration-span derivation report, and the byte-identical parameter control)
- [x] 3.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the three lenses ✅ (completed: 2026-10-13 12:09; correctness + memory-safety + semantics found RV-301/SA-302 (both critical, fixed), the scoped re-review found RV-302 = SR-002/SA-303/SR-301 (fixed in the second round with three new pins), the targeted confirmation returned PASS (RV-303 fixed directly, RV-304 accepted); DEF-265 – DEF-269 filed; spec 50/50 plain and ASan, impl 15/15, `make check` exit 0)

**Verify**: `make -C docker exec CMD="cargo test --test conditional_projection_integrity_spec_test --test conditional_projection_integrity_impl_test"`; `make check`

## Phase 4: Statement-path discards

> **Phase baseline tree**: `66026bfdbe2ddc72b95135d261927e3a4c56a5ca` (Phase 3's final commit tree at `a51c508`)
> **Lenses**: correctness + memory-safety + emission-integrity
> **Reasoning**: medium — three changes in the discard lowering

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-statement-path-discards.md)
**Objective**: Pin the closure-call and conditional discards and the intrinsic dot-call emission.

- [x] 4.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_integrity_spec_test.rs` (closurecalldiscard, conddiscard, dotlendiscard, discardcontrol) ✅ (completed: 2026-10-13 14:30; four new cases: the owning and duplicable closure-call instances, the conditional loop exercise of both arms, the two dot-call statement spellings with the value-position control, and the untouched discard-shape control)
- [x] 4.1.2 Run the suite and record the red phase ✅ (completed: 2026-10-13 14:30; plain 53/54 — the dot-call case red with the implicit `len` declaration; ASan 51/54 — plus the closure-call leak 11 B and the conditional leak 33 B in 3 allocations; the control case green as designed)

### Step 4.2: Implementation

- [x] 4.2.1 Fix the closure-call predicate, bind the conditional arm's result, and route the discarded intrinsic dot-call — `compiler/src/own/statements.rs`, `compiler/src/own/lower.rs` ✅ (completed: 2026-10-13 14:36; the shared `discard_binds_owner` predicate (`!unknown && !fixed-array && owns_drop || maybe_owns`) drives the closure-call binding; the conditional arm guard-materializes each fresh owning result into a statement temporary via `defer_discarded_arm` + `discard_conditional_depth` (place arms stay reads; the deferred take-arm spelling is untouched); the discarded intrinsic dot-call routes through `lower_intrinsic_call` and discards only when the value is effectful; the lifted-lambda boundary resets the new depth)
- [x] 4.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-13 14:36; the first attempt bound the whole conditional and moved a place arm (`t = NULL`), caught by the batch-D impl pin `should_release_a_discarded_transfer_temporary_after_the_statement` and redesigned to per-arm guard materialization; spec 54/54 plain and under `RDC_ASAN=1`, impl 15/15, `call_value_integrity_impl` 61/61, `make check` exit 0 — 64 binaries, 1867 tests)

### Step 4.3: Impl Tests & Review

- [x] 4.3.1 Write the phase implementation pins (the statement temporaries, the call spelling, the byte-identity controls) — `compiler/tests/conditional_projection_integrity_impl_test.rs` ✅ (completed: 2026-10-13 14:39; six new pins: the owning and duplicable closure instances, the guard-materialized conditional arms with their releases, the place arm staying a read, the folded dot-call with its bound fresh receiver and release, the value-position byte-identity control, and the untouched-shape byte-identity control; an `after` helper mirrored from the sibling suite)
- [x] 4.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the three lenses ✅ (completed: 2026-10-13 15:20; correctness + memory-safety + emission-integrity found RV-401/SA-401 (aliasing struct-literal arm, critical, fixed by the gate), the scoped re-review found RV-402/EA-401 (the nested spelling, critical, fixed by the recursive predicate), the first confirmation found RV-403 (the conditional spelling, critical, fixed and pinned), the final confirmation returned PASS on all five items with no findings; RV-404 accepted, RV-405/RV-406 fixed; DEF-270 – DEF-272 filed; spec 55/55 plain and ASan, impl 21/21, `make check` exit 0)

**Verify**: `make -C docker exec CMD="cargo test --test conditional_projection_integrity_spec_test --test conditional_projection_integrity_impl_test"`; `make check`

## Phase 5: Emission spellings

> **Phase baseline tree**: `4cfe13ebf04c44ee28a7eeffa78a729b18d41ac3` (Phase 4's final commit tree at `926b3ad`)
> **Lenses**: correctness + emission-integrity + semantics
> **Reasoning**: medium — three emitter/lowering spellings

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-emission-spellings.md)
**Objective**: Pin the conditional return, the postfix pin, and the fixed-array address.

- [x] 5.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_integrity_spec_test.rs` (condreturn, postfixseq, fixedarrayaddr, emissioncontrol) ✅ (completed: 2026-10-13 19:43; four new cases: the view-typed conditional in the return and store spellings, the shared postfix update program (both operand orders, two updates, the call-argument spellings, the standalone initializer, and the single-access form), the fixed-array projection in the return, `let`, and assignment spellings with the owned-field and call-argument controls, and the untouched-spelling control; the author's arithmetic check caught two packet subtrahend errors, corrected by the parent — the strict left-to-right sums are 48 and 10)
- [x] 5.1.2 Run the suite and record the red phase ✅ (completed: 2026-10-13 19:43; plain 56/59 — the three filed strict-build failures (the conditional type mismatch, the sequence-point update, and the fixed-array address of a non-lvalue); ASan 56/59 — the same three; the control case green as designed)

### Step 5.2: Implementation

- [x] 5.2.1 Unify the conditional arm spelling, pin the postfix update, and bind the call root before the fixed-array address — `compiler/src/codegen/expr.rs`, `compiler/src/own/lower.rs` ✅ (completed: 2026-10-13 19:50; the arm widening is the shared `widen_text_view` helper (`codegen/function.rs`, `codegen/expr.rs`) so the return and store spellings take one C type; the update order pins through `contains_update` + `pin_update_value` in the `Binary` arm and in the call-argument loop (a `take` argument and a constant keep their spelling, and a guarded or loop-condition operand keeps its existing deferred form); the fixed-array address binds the ownerless root through `bind_fixed_array_root` in `prepare_store` and `check_return`, with the owning root keeping its scope-owner spelling)
- [x] 5.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-13 19:50; spec 59/59 plain and under `RDC_ASAN=1`; impl 21/21; `make check` exit 0 — 64 binaries, 1878 tests; the emitted shapes verified by hand: `return (c ? cl_string_view(b.v) : a);`, `__cl_tmp0 = (i)++; __cl_tmp1 = i;`, `__cl_tmp0 = mk(); memcpy(&(a), &(__cl_tmp0.xs), sizeof(a));`)

### Step 5.3: Impl Tests & Review

- [x] 5.3.1 Write the phase implementation pins (the unified spelling, the update temporary, the bound root, the byte-identity controls) — `compiler/tests/conditional_projection_integrity_impl_test.rs` ✅ (completed: 2026-10-13 19:50; six new pins: the widened arm in the return and store spellings with the same-typed control, the same-typed conditional byte-identity, the update temporaries in both operand orders, call arguments, and a single-access wider expression (with no inline update beside another access to its place), the bound fixed-array root in the return and store spellings, the owning root's byte-identical scope-owner spelling and release, and the untouched standalone update and update initializer byte-identity)
- [x] 5.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the three lenses ✅ (completed: 2026-10-13 20:09; the three lenses found RV-001/EIA-5.1/EIA-5.2 (the post-lowering pin inverted left-to-right for a nested update in a later operand — critical, fixed by pinning at each operand's and argument's own lowering position) and a newly reachable dangling conditional return (critical, fixed by the differing-arms provenance rule and pinned by the two local-escape rejects); EIA-5.3 accepted record-only with the `03-05` wording corrected; `DEF-273` – `DEF-276` filed; the scoped re-review then verified the criticals fixed with only report-only minors (`DEF-277` – `DEF-280` filed); spec 61/61 plain and under `RDC_ASAN=1`, impl 27/27, `make check` exit 0 — 64 binaries, 1886 tests)

**Verify**: `make -C docker exec CMD="cargo test --test conditional_projection_integrity_spec_test --test conditional_projection_integrity_impl_test"`; `make check`

## Phase 6: Close-out

> **Phase baseline tree**: to be recorded at Phase 5's close (= Phase 5's final commit tree)
> **Lenses**: none new — the close-out verifies the reviewed phases

### Step 6.1: Final Verification & Documentation

**Reference**: [01-requirements.md](01-requirements.md) §Acceptance Criteria
**Objective**: Run the full acceptance, sync the docs and the roadmap, and close the plan.

- [x] 6.1.1 Run the full verification: `make check`, `make verify`, the strict-build sweep, and the ASan suite; record the evidence ✅ (completed: 2026-10-13 20:26; `make check` exit 0 — 64 binaries, 1886 tests, spec check OK (27 files, 110 compile-fail); `make verify` exit 0 — all legs passed (the hosted-amiga corpus matches, the FS-UAE-only programs match, the runtime-fault programs run); the focused suites: spec 61/61 plain and under `RDC_ASAN=1`, impl 27/27; the strict-build sweep: every head-accepted suite and corpus unit compiles under `-Werror` on host, hosted-amiga, and freestanding)
- [x] 6.1.2 Update `AGENTS.md` (test naming map with the new suites) and sync the roadmap rows to Done with per-row evidence ✅ (completed: 2026-10-13 20:26; the test naming map lists the two suites with their coverage; the 14 batch-H rows are Done with per-row evidence — six earlier-phase rows (`DEF-176`, `DEF-178` – `DEF-182`) were still `Executing` and are corrected; the review-filed rows `DEF-257` – `DEF-280` stay in the backlog)
- [x] 6.1.3 Record the close-out section in the findings ledger and mark the plan Complete in `00-index.md` ✅ (completed: 2026-10-13 20:26; the close-out records the final verification, the roadmap sync, and the plan close; `00-index.md` marks the plan Complete)

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
