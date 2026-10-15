# Execution Plan: Conditional, Projection, and Update Residue II

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-15 (planning)
> **Progress**: 30/33 tasks (91%) — Phase 5 complete; close-out next
> **Planning baseline**: `a462084` (the batch-I close-out commit)
> **CodeOps Artifact Schema**: 1

## Overview

Close the 27 open batch-I follow-on rows — 1 critical, 5 majors, 21 minors, none parked — in
five phases (view owners/loans/stores; guarded/discarded/carrier roots; fixed-array and literal
emissions; loop conditions/update order/store lowering; diagnostics and marks) and a close-out.
Every fix extends the batch-I machinery; every phase follows the specification-first ordering
(spec cases → red → implement → green → impl pins → review). The new cases append to the batch-I
suite pair (AR-12); the landed partial fixes (`DEF-306`, `DEF-308`) stay green as pinned
controls.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | View owners, loans, and stores | 6 |
| 2 | Guarded, discarded, and carrier roots | 6 |
| 3 | Fixed-array and literal emissions | 6 |
| 4 | Loop conditions, update order, and store lowering | 6 |
| 5 | Diagnostics and marks | 6 |
| 6 | Close-out | 3 |

**Total: 33 tasks across 6 phases**

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

## Phase 1: View owners, loans, and stores

> **Phase baseline tree**: `50b91d9` (the phase-4 red-phase commit; the five new cases red / the pins fail there), recorded 2026-10-16
> **Lenses**: correctness + memory-safety (AR-13) — dispatched via the project brief and the
> read-only `memory-safety-auditor`
> **Reasoning**: high — ownership and lifetime judgments in the freshly reviewed machinery
> (extend the owner walk and the store judgment, never re-architect)

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-view-owners-loans-stores.md) · [07](07-testing-strategy.md)
ST-1..ST-7 · AR-3, AR-5, AR-6
**Objective**: Pin the duplicable-projection residual binding, the runtime-index element store,
the cross-frame out-slot rule (stop-gated), the module-global fresh init, the container-escape
element loans, the stale literal-carrier loan, and the all-null composite store.

- [x] 1.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (dupresidual, runtimestore, crossframe, modulefresh, containerescape, staleloan, allnull) ✅ (completed: 2026-10-15 21:05; seven tests appended (+867 lines, additions only; 88 → 95 tests) — the residual-owning duplicable projection read twins (in loops, 12 B per read), the runtime-index element store with the constant-index parity controls, the cross-frame out-slot pass-through (the param/same-frame controls; the non-nullable `borrow slot: str` chain is a dead write and excluded), the module-global fresh-owner retention (the direct `stringOf` spelling is already clean; the member-projection spelling is the red), the container escapes (element/alias/global) against the single-view twin, the stale carrier loans against the live-loan control, and the all-null composite with its mix controls; every accepted program folds a read-back into its exit code per the preflight)
- [x] 1.1.2 Run the suite and record the red phase (the probe-verified reds re-measured; the landed-twin controls green first — AR-3) ✅ (completed: 2026-10-15 21:05; plain 90 passed / 5 failed and ASan 88 passed / 7 failed — the seven new tests are the failures (ST-1/ST-4 are leak-only plain greens); the reds re-derived at the head: the duplicable-projection reads leak 66–96 B, the runtime-index fresh store leaks 11 B and the loop spelling 33 B with no `E10030` (its mutation reports already fire — green controls), the cross-frame spellings are accepted with heap-use-after-free (local/temp/str-twin/array), the module member spelling leaks 11 B, the container escapes are accepted with use-after-free (element/alias 143/219) or leak (global `E10033` missing), the stale carrier loans falsely report `E10031`, and the all-null stores falsely report `E10030`/`E10033`; parent-verified)

### Step 1.2: Implementation

- [x] 1.2.1 Bind the residual-owning fresh root for duplicable projections; route the
      runtime-index element store through the copy/loan path; implement the cross-frame
      call-site rule (or stop and park per AR-6); release/adopt the module-global fresh owner;
      consult the container escape's element loans; drop the overwritten literal-carrier loans;
      accept the all-null composite store — `compiler/src/own/lower.rs`,
      `compiler/src/own/place.rs`, `compiler/src/own/statements.rs`, `compiler/src/own/expr.rs`,
      `compiler/src/own/call_check.rs`, `compiler/src/own/program.rs`,
      `compiler/src/own.rs` ✅ (completed: 2026-10-15 21:35; the actual touched files: `own/lower.rs`, `own/place.rs`, `own/statements.rs`, `own/expr.rs`, `own/call_check.rs`, `own.rs`, `codegen/program.rs`, `codegen/function.rs`; the residual-owning root binds through a lower-value tail hook and the gates accept a root that needs a fill even when the selected member is duplicable; the runtime index copies a view element and returns the prepared view for the loan record; the cross-frame rule is a contained call-site check (AR-6 outcome: no park — the accepted parameter/same-frame twins are preserved; boundary: a read-only out-slot callee with a this-frame view argument still rejects); the module-global root is retained for release with the read member emptied; the container escapes consult the frame loan owners at the return and the global store; the literal-carrier loans are per-position with a prefix replacement; the all-null merge yields an empty composite while the plain `None` sentinel keeps its rejection)
- [x] 1.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-15 21:35; the residue spec 95/95 plain and under `RDC_ASAN=1`; the batch-H, callable-release, module-initializer, and checker-conformance suites green; `make check` exit 0 with no new warnings; parent-verified 95/95 plain + ASan)

### Step 1.3: Impl Tests & Review

- [x] 1.3.1 Write the phase implementation pins (the binding gates, the loan record/replace shapes, the report counts and spans, the re-emission determinism pins) — `compiler/tests/conditional_projection_residue_impl_test.rs` ✅ (completed: 2026-10-15 21:35; seven pins added (59 tests total, additions only); every pin fails at `bf394e5` on its first assertion with the controls (`liveloan`, `nullmixlocal`) holding; 59/59 green)
- [x] 1.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses (correctness + memory-safety; the reviewer packet includes the blast-radius step and the test-integrity step) ✅ (completed: 2026-10-16 01:25; both lenses dispatched plus three targeted confirmations; the fix round (the owners-resolved cross-frame rule, the read-only boundary pin, F2/F4-deep/F5) and the correction round (C1 `Param` element loans, C2 the retain gate + array fill, C3 the two-level loan, C4 the mixed array-literal) closed every in-scope finding; the two fix-round claim gaps were caught by the confirmation and fixed; five rows filed (`DEF-311`..`DEF-315`); the frozen-revision verification: the residue spec 107/107 plain and ASan, the impl pins 63/63, `make check` exit 0 with no new warnings)

**Verify**: `make check`; the ownership cases under `RDC_ASAN=1`; the strict-build probes

**Deliverables**:
- [ ] ST-1..ST-7 red then green (the cross-frame case or its recorded park per AR-6)
- [ ] The batch-I suites and the pinned oracles stay green

---

## Phase 2: Guarded, discarded, and carrier roots

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + memory-safety (AR-13)
> **Reasoning**: high — the root binders and discard materializers batch I built; the guarded
> path must stay lazy

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-guarded-discarded-carrier-roots.md) · [07](07-testing-strategy.md)
ST-8..ST-11
**Objective**: Pin the deferred call-wrapped element root, the conditional carrier literal's
taken arm, the discarded residual-owning root, and the bare discarded literal.

- [x] 2.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (guardcallroot, carrierliteral, discardresidual, barediscard) ✅ (completed: 2026-10-16 01:55; four tests appended (+382 lines, additions only; 107 → 111 tests) — the guarded call-wrapped element root with the call-count folded into the exit code and the direct unguarded control, the conditional carrier literal with the loop-condition and guarded-literal controls, the discarded non-owning arm with the residual sibling field against the sole-field and owned-type controls, and the bare discarded literal with the conditional-discard control; two shape adjustments recorded: the ST-11 place-leaf spelling is `DEF-253` territory (the conditional spelling serves as the control) and the ST-9 loop-condition literal spelling is phase-3 emission territory (the guarded spelling serves as the control))
- [x] 2.1.2 Run the suite and record the red phase ✅ (completed: 2026-10-16 01:55; plain 109 passed / 2 failed and ASan 107 passed / 4 failed — exactly the four new tests (the two leak-only reds are plain greens); the reds: the guarded call runs on the skipped path (exit 30) and leaks 66 B/6, the taken carrier arm leaks 22 B/2, the discarded residual leaks 11 B/1, and the bare discard fails the strict build (`expected ';'`, `(void)({ __cl_tmp1 });`); the five controls green; parent-verified)

### Step 2.2: Implementation

- [x] 2.2.1 Extend the deferred branch to the call-wrapped element position; bind the carrier
      literal's taken-arm fresh root for the local's frame; judge the discarded arm by the
      root's residual ownership; materialize the bare discarded literal's fresh leaves —
      `compiler/src/own/lower.rs`, `compiler/src/codegen/guards.rs`,
      `compiler/src/codegen/function.rs` ✅ (completed: 2026-10-16 02:30; the actual touched files: `own/lower.rs`, `own/statements.rs`, `codegen/function.rs` (`codegen/guards.rs` needed no change); the lend path binds a projection over a fresh root under `defer_move_outs > 0` through the deferred fill; the carrier literal's taken-arm fresh roots bind via the aggregate view-position descent; the discarded arm is judged by the root's residual ownership (narrowed to keep the batch-I dot-len pins); the bare discarded literal materializes its fresh leaves with the guards-only discard emission)
- [x] 2.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-16 02:30; the residue spec 111/111 plain and under `RDC_ASAN=1`; the batch-H, callable-release, and batch-I suites green; `make check` exit 0 with no new warnings; the differential then showed eight intended emit-diffs — **superseded by the correction rounds: the re-measured count at the phase's final tree is 16 emit-diff rows + 3 rejection rows, all intended (the correctness confirmation J2C-1)**; parent-verified 111/111 plain + ASan at the time)

### Step 2.3: Impl Tests & Review

- [x] 2.3.1 Write the phase implementation pins (the deferred fill shapes, the arm bindings, the release counts, the re-emission determinism pins) — `compiler/tests/conditional_projection_residue_impl_test.rs` ✅ (completed: 2026-10-16 02:30; five pins added (68 tests total, additions only); the deferred fill inside the guard, the taken-arm binding with its release, the residual-root bind, the bare-discard leaf materialization + determinism; 68/68 green)
- [x] 2.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses ✅ (completed: 2026-10-16 04:15; both lenses dispatched plus three targeted confirmations; the correction rounds: the discard materialization (struct fields + every impure leaf), the out-of-frame carrier-store rejection, the gate narrowed to non-owning arms, the kept-view scope-owner bind, the mixed-discard root bind, and the element-conditional `Conditional` arm with the arm-fill acceptance; the two fix-round claim gaps were caught by the confirmations and fixed; six rows filed (`DEF-316`..`DEF-321`, `DEF-288` extended); the closing verification: the residue spec 116/116 plain and ASan, the impl pins 68/68, `make check` exit 0, the six repros clean on the parent's re-run)

**Verify**: `make check`; the ownership cases under `RDC_ASAN=1`

**Deliverables**:
- [ ] ST-8..ST-11 red then green
- [ ] The batch-I discard/guard pins stay green

---

## Phase 3: Fixed-array and literal emissions

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + emission-integrity + memory-safety (AR-13) — the read-only
> `emission-integrity-auditor` joins the base/head differential
> **Reasoning**: high — the hidden out-parameter ABI, the per-arm destination stores, and the
> critical's return shape

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-fixed-array-literal-emissions.md) · [07](07-testing-strategy.md)
ST-12..ST-19 · AR-3, AR-5
**Objective**: Pin the field-initializer routes, the guarded view-array field, the conditional
of call-filled literals, the `String[2]` conditional-root return (critical), the
guarded/deferred fallback, the array-literal element gate, and the nested-arm read base with its
landed controls.

- [x] 3.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (condfield, condrootfield, guardviewfield, condcallliteral, condrowreturn, guardfallback, arrayelement, nestedread) ✅ (completed: 2026-10-16 04:45; eight tests appended (+792 lines, additions only; 116 → 124 tests) with twelve controls; the reds re-derived: the conditional fixed-array field (plain `lvalue required` + guarded `int-conversion`), the conditional root into a literal field, the guarded view-array field (`missing-field-initializers`), the conditional of call-filled literals (`too few arguments`), the `String[2]` conditional-root return (segfault 139), the guarded fixed-array consume (`take`+owning; the skipped path runs an arm), the conditional element of an array element, and the nested-arm element read; `DEF-305`'s filed direct shape is green at the head (closed incidentally by the phase-2 commit) and stands as a control)
- [x] 3.1.2 Run the suite and record the red phase (the `DEF-306` store/iterable controls ✅ (completed: 2026-10-16 04:45, corrected 2026-10-16 06:10 per J3R-3: the red ref is `47306d2` + the two-line `place.rs` visibility hunk (the committed tree alone does not compile); plain 116 passed / 8 failed and ASan 116 passed / 8 failed — exactly the eight new tests; the twelve controls green; parent-verified)
      verified green first — AR-3)

### Step 3.2: Implementation

- [x] 3.2.1 Route the conditional field values and conditional-root field projections through
      the per-arm destination store; use the array-elements initializer for the guarded
      view-array field; fill the conditional-of-literals per arm; bind each arm in the
      fixed-array return emitter (the critical); route the guarded expression fallback through
      the per-arm store; extend `fixed_array_field_needs_stores` to `ArrayLit` elements;
      distribute the nested-arm read base — `compiler/src/codegen/function.rs`,
      `compiler/src/codegen/guards.rs`, `compiler/src/codegen/cleanup.rs`,
      `compiler/src/own/lower.rs`, `compiler/src/own/statements.rs` ✅ (completed: 2026-10-16 05:20; the field branch routes a fixed-array conditional/projection field through the per-arm root binding and the store twin follows; a guarded literal field without an initializer form materializes; the guarded view element binds through the deferred fill with the widened array-literal element spelling; the fixed-array field stores through the row core so a call field passes the hidden destination; the fixed-array return copies Temp/Materialize arms per arm through the store core (the critical); the expression-path fallback uses the row core; an element conditional of store-needing literals stores per arm with the gate descending array-literal elements; the read base distributes nested conditional arms; the guarded direct-field spellings are covered by the same mechanism; an `eager_conditional_arm` flag keeps eager conditional-arm literals on the previous path so no pin was invalidated)
- [x] 3.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-16 05:20; the residue spec 124/124 plain and under `RDC_ASAN=1`; the batch-H, callable-release, and related suites green; `make check` exit 0; the differential shows 27 intended emit-diffs and no accept/reject change; parent-verified 124/124 plain + ASan)

### Step 3.3: Impl Tests & Review

- [x] 3.3.1 Write the phase implementation pins (the per-arm destination shapes, the
      hidden-destination pass, the copy shapes, the arm rebuild, the re-emission determinism
      pins) — `compiler/tests/conditional_projection_residue_impl_test.rs` ✅ (completed: 2026-10-16 05:45; six pins added (74 tests total, additions only) — the per-arm conditional fixed-array field store, the per-arm String[2] return copy (the critical), the guarded view-array field element fill, the hidden-destination call fields, the nested element read base, and the re-emission determinism pin (with a repaired-shape fragment so it discriminates); each fails at `47306d2` for the intended reason; 74/74 green and `make check` exit 0)
- [x] 3.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses plus the base/head emission differential ✅ (completed: 2026-10-16 06:55; both lenses dispatched plus one emission re-confirmation; the correction round fixed the kept view binding's statement-scoped root (the scope-owner path) and the guarded element-conditional double free (the per-arm move), plus the marker/doc cleanups and the baseline/red-ref corrections; rows filed: `DEF-322`..`DEF-324`; the phase-2 build gap (`cfb4c1a`/`47306d2` missing the two-line `place.rs` widening) is recorded in the ledger; the closing verification: the residue spec 126/126 plain and ASan, the impl pins 74/74, `make check` exit 0, the repros clean on the parent's re-run)

**Verify**: `make check`; the emission differential; the ownership cases under `RDC_ASAN=1`

**Deliverables**:
- [ ] ST-12..ST-19 red then green (the critical `DEF-303` included)
- [ ] The emission differential shows only intended deltas; the direct spellings byte-identical

---

## Phase 4: Loop conditions, update order, and store lowering

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + emission-integrity + semantics + memory-safety (AR-13)
> **Reasoning**: high — the loop fill, the update pin, the raw-deref lowering, and a
> loop-condition ownership fix; the `DEF-294` outcome is recorded either way (AR-10)

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-loop-conditions-update-order.md) · [07](07-testing-strategy.md)
ST-20..ST-24 · AR-3, AR-10
**Objective**: Pin the mixed-arm loop receiver (dual-toolchain), the loop-condition
projection, the guarded update spellings with their landed loop-condition control, the fill
evaluation, and the bare raw-deref store.

- [x] 4.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (loopreceiver, loopmember, guardupdate, arrayfill, rawderefstore) ✅ (completed: 2026-10-16 07:20; five tests appended (+308 lines, additions only; 126 → 131 tests) — the mixed-arm receiver in a loop condition (the dual-toolchain case per AR-10: the container leg green, the host leg `-Werror=sequence-point`), the loop-condition member projection as a green control (DEF-295 is incidentally fixed by the phase-2 residual gate — the re-measurement confirmed), the guarded spread operand (the sequence-point reds), the array fill evaluated once, and the bare raw-deref store; the eager top-level receiver read's 11 B leak was observed and excluded (recorded for assessment))
- [x] 4.1.2 Run the suite and record the red phase (the plain loop-condition and fill-free ✅ (completed: 2026-10-16 07:20; plain 128 passed / 3 failed and ASan 128 passed / 3 failed — exactly the three reds; the two new green cases and all 126 committed cases pass; parent-verified)
      controls verified green first; the `DEF-294` dual-toolchain measurement recorded — AR-10)

### Step 4.2: Implementation

- [x] 4.2.1 Settle/pin the mixed-arm loop receiver; bind the loop-condition projection's root
      once; pin the guarded operand's updating parts inside the guard path; pin the array fill
      before the element loop; lower the bare raw-deref store target with the update pin —
      `compiler/src/own/lower.rs`, `compiler/src/own/call.rs`, `compiler/src/own/expr.rs`,
      `compiler/src/own/loops.rs`, `compiler/src/codegen/guards.rs`,
      `compiler/src/codegen/control.rs`, `compiler/src/own/statements.rs`,
      `compiler/src/ir.rs` ✅ (completed: 2026-10-16 07:50; a func-field receiver that is not a place/temporary/bound value settles into a deferred temporary at argument position zero (`own/call.rs:153-168`), so both toolchains accept (AR-10 closure: the pinned emission changes to the sequential shape) and the eager top-level receiver read's 11 B leak closes; the guarded operand's updating parts settle through the guard fill (`own/lower.rs:1694`); an update-bearing array fill pins once (`:497-510`); the bare raw-deref store lowers (`own/expr.rs:920-985`, `:808`, `:2369`) with the update pin and the `unsafe` gate intact; the red-phase case's arithmetic was corrected per the parent's ruling (the per-path pass value; the exit-0 invariant preserved and discriminated — a wrong order exits 11))
- [x] 4.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-16 07:50; the residue spec 131/131 plain and under `RDC_ASAN=1`; the batch-H and related suites green; `make check` exit 0; the differential shows only the intended emit-diffs and no accept/reject change; parent-verified 131/131 plain + ASan)

### Step 4.3: Impl Tests & Review

- [x] 4.3.1 Write the phase implementation pins (the receiver pin shape, the loop root binding,
      the guarded fill positions, the fill pin, the deref store statement, the re-emission
      determinism pins) — `compiler/tests/conditional_projection_residue_impl_test.rs` ✅ (completed: 2026-10-16 08:10; five pins added (79 tests total, additions only) — the settled receiver at argument position zero (loop and eager), the settled guarded operand parts, the single array-fill pin, the lowered raw-deref store with its update pin, and a discriminating re-emission determinism pin; all five fail at `50b91d9` for the intended reason; 79/79 green and `make check` exit 0)
- [x] 4.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; record the `DEF-294` closure (fixed or control) in the ledger \u2705 (completed: 2026-10-16 11:00; all three lenses dispatched plus three targeted confirmations; five correction rounds closed J4R-1..4, F1..F3, and FINAL-1 (the borrowed receiver arm, the raw-store pointee drop, the compound store, the nullable check, the named-destination order, the single pointer bind, the moved deref source); FINAL-2 ruled the AR-18 unsafe boundary; the red-phase case arithmetic corrected per the ruling; six rows filed (`DEF-325`..`DEF-330`); `DEF-294` closed per AR-10 and `DEF-295` closed incidentally; the closing verification: the residue spec 138/138 plain and ASan, the impl pins 79/79, `make check` exit 0)

**Verify**: `make check`; the strict builds on both toolchains; the batch-H update pins

**Deliverables**:
- [ ] ST-20..ST-24 red then green (or the `DEF-294` control outcome with its evidence, AR-10)
- [ ] The batch-H update pins stay green; the controls byte-identical

---

## Phase 5: Diagnostics and marks

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + semantics (AR-13)
> **Reasoning**: medium — report counts/phrases and a judgment mark; the pinned oracles gate

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-diagnostics-and-marks.md) · [07](07-testing-strategy.md)
ST-25..ST-27 · AR-7, AR-9
**Objective**: Pin the one-report dedupe at a nested mixed return and its cross-site instances,
the generated-owner phrase, and the conditional-iterable move mark.

- [x] 5.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (onereportnested, deferphrase, iterableark) ✅ (completed: 2026-10-16 11:40; three tests appended (+372 lines, additions only; 138 → 141 tests) plus the one-report helper — the nested mixed view return reporting twice at one span (the file, the global-field and the two-owner pair spellings) with the local-only and local+param controls, the deferred generated-owner phrase (`__cl_owned1`) with the direct-phrase control, and the whole-array conditional iterable's stale per-arm move marks with the `u8[4]` control; the reds re-derived: the iterable case reports two marks (one per arm) and the mixed-pair probe was re-authored from the surviving evidence (disclosed))
- [x] 5.1.2 Run the suite and record the red phase (the controls' counts re-measured first) ✅ (completed: 2026-10-16 11:40; plain 138 passed / 3 failed and ASan 138 passed / 3 failed — exactly the three new tests; the controls and the pinned oracles (`callable_release_integrity`, `checker_conformance`) green; parent-verified)

### Step 5.2: Implementation

- [x] 5.2.1 Dedupe the reports at one span and withhold the consequence report per AR-7; route
      the deferred owner display through the generated-owner phrase; align
      `mark_read_conditional_move` with the in-place conditional iterable —
      `compiler/src/own/liveness.rs`, `compiler/src/own/statements.rs`,
      `compiler/src/own/place.rs`, `compiler/src/own/expr.rs` ✅ (completed: 2026-10-16 12:05; the escape report loop keeps a per-span set (a nested mixed view return reports once — the file, the pair and the global-field spellings) and the destination consequence report is withheld where the callee own judgment stands; the deferred generated owner renders with the same phrase as the direct spelling (the reserved binding spelling recognized); the stale move mark on a materialized whole-array conditional iterable is removed, keeping the in-place loop emission; two reported residuals left for filing (the one-candidate mismatch caller case and the older quoted-spelling phrase fallback))
- [x] 5.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-16 12:05; the residue spec 141/141 plain and under `RDC_ASAN=1`; the pinned oracles green; `make check` exit 0; the differential shows exactly the three diagnostic dedupes and the one intended acceptance; parent-verified 141/141 plain + ASan)

### Step 5.3: Impl Tests & Review

- [x] 5.3.1 Write the phase implementation pins (the counts and spans, the displayed phrase,
      the mark shape, the re-emission determinism pins) —
      `compiler/tests/conditional_projection_residue_impl_test.rs` ✅ (completed: 2026-10-16 12:30; six pins added (85 tests total, additions only) plus a one-report helper — the dedupe at the nested and pair returns (the pair owner unpinned), the withheld global-field consequence, the deferred phrase, the in-place iterable on owned and duplicable elements, the element/projection regression guard, and a discriminating re-emission pin; all six fail at `d0f9558` for the intended reason; 85/85 green and `make check` exit 0)
- [x] 5.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; run the pinned oracles (`callable_release_integrity`,
      `checker_conformance`) ✅ (completed: 2026-10-16 14:00; both lenses dispatched plus one correction round: J5R-1/J5S-2 (the suppressed callee arm now reports the frame-backed component — the `Composite` arm in the return judgment) and J5S-1 (the withholding resolves the callee through the call precedence, so a callable value keeps its own report) fixed; J5R-2 (the shared subject helper) and J5S-3 (the generated-id tracking replacing the spelling heuristic) fixed; J5R-3 closed as a boundary per the semantics review; four spec cases added (145 total); rows filed: `DEF-331` (closed boundary), `DEF-332`, `DEF-333`; the closing verification: the residue spec 145/145 plain and ASan, the impl pins 85/85, `make check` exit 0, no head-only acceptance, no emission diff)

**Verify**: `make check`; the oracle suites

**Deliverables**:
- [ ] ST-25..ST-27 red then green
- [ ] `callable_release_integrity`'s exactly-one `E10030` pin and the `checker_conformance`
      phrase test stay green

---

## Phase 6: Close-out

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: none new (per the `AGENTS.md` dispatch matrix)
> **Reasoning**: low — verification and bookkeeping

### Step 6.1: Verification and Sync

- [ ] 6.1.1 Run the full verification: `make check`, `make verify`, the strict-build sweep, and
      the ASan suites; record the evidence
- [ ] 6.1.2 Update the roadmap rows: the 27 rows move to Done with per-row evidence; the six
      older parked rows stay open (AR-2); the batch-I rows already closed stay untouched
- [ ] 6.1.3 Record the close-out section in the findings ledger and mark the plan Complete in
      `00-index.md`

**Verify**: `make check`; `make verify`

**Deliverables**:
- [ ] The 27 rows accounted for: Done with evidence (or the registered park/control outcomes)
- [ ] The six older parked rows' rulings collected (AR-2)
- [ ] The plan marked Complete

---

## Dependencies

```
Phase 1 (owners/loans/stores) → Phase 2 (guarded/discarded/carrier roots)
        ↓
Phase 3 (fixed-array/literal emissions)
        ↓
Phase 4 (loop/update/store lowering)
        ↓
Phase 5 (diagnostics/marks) → Phase 6 (close-out)
```

Arrow legend: `A → B` marks a machinery dependency (B extends the binders, owner walk, or
preparation A touches); the vertical order is the executable sequence — the phases run
1 → 2 → 3 → 4 → 5 → 6 in order.

---

## Verification Commands

| Command | Purpose |
|---------|---------|
| `make check` | Per-phase gate: `./spec/check.sh` plus `cargo test --workspace` in the pinned container |
| `make -C docker exec CMD="cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test"` | The focused residue suites |
| The suite's `RDC_ASAN=1` switch | The memory runs (the suite helper) |
| `make -C docker exec CMD="target/debug/clc --profile host -o out.c prog.clo"` + `cc -std=c99 -O0 -g -Wall -Wextra -Werror` | The strict-build probes (the pinned container; the host toolchain for `DEF-294`, AR-10) |
| `make verify` | Close-out: every hosted-amiga leg plus the runtime-fault programs |

## Reviewer Dispatch Conventions

- Correctness and semantics via the project briefs (`.opencode/agents/correctness-reviewer.md`,
  `.opencode/agents/semantics-reviewer.md`); memory-safety and emission-integrity via the
  dedicated read-only subagents (`memory-safety-auditor`, `emission-integrity-auditor`).
- Every reviewer packet includes the **blast-radius step** (every call site, exhaustive match,
  and parallel path of the changed construct) and the **test-integrity step** (an assertion must
  fail without the fix); the read-only auditors stay read-only.
- Phase dispatch follows the change surface (AR-13) and the `AGENTS.md` matrix.

## Success Criteria

**Feature is complete when:**

1. [ ] All phases completed
2. [ ] All verification passing (`make check`; `make verify` at close-out)
3. [ ] No warnings/errors; no dead code
4. [ ] Security hardened — not applicable beyond the compiler's own input validation (unchanged)
5. [ ] Documentation updated (the roadmap rows and the findings ledger)
6. [ ] Code reviewed (every phase's declared lenses)
