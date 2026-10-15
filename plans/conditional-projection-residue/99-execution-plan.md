# Execution Plan: Conditional, Projection, and Update Residue

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-15 09:52
> **Progress**: 30/39 tasks (77%)
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
- [x] 1.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses ✅ (completed: 2026-10-15 07:52; correctness and memory-safety dispatched on the phase diff plus three targeted confirmation rounds; six in-diff findings resolved in the round — the adopting-carrier hoist the deferred-field route made reachable, the fixed-array exclusion and gate parity across the new carrying branches, the per-instance take/return move-out consistency, and the literal-source moves including their nested recursion — and the pre-existing spelling families completed rather than refiled; `DEF-281` filed; the final re-probe confirmed clean; the full record is in `05-findings.md`)

**Verify**: `make check`; the ownership cases under `RDC_ASAN=1`

**Deliverables**:
- [x] ST-1, ST-2, ST-4, ST-5 red then green (plus the review-round regression cases recorded in the findings ledger); ST-3 and ST-6 green controls (the incidental closure — AR-23 — and the direct spellings)
- [x] All verification passing: the ownership cases under `RDC_ASAN=1`; the batch-H suites (61/61, 27/27), `residual_defect_hardening` (50/0, 26/0), the callable-release oracle (52/0), and `./spec/check.sh` (27 files, 110 compile-fail tests) green; `make check` at the round's end fails only the four declared phase-2 reds (Step 2.1)

---

## Phase 2: View derivation and owners

> **Phase baseline tree**: `c148fc5` (the phase's fix-tree base; recorded 2026-10-15)
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

- [x] 2.2.1 Extend the owner walk through a view-returning call (every arm and the chained callee), the element positions, and the return's view-element lowering; suppress the local-owner `E10035` at a mixed return whose other arm's lowered view reads a compiler-generated root or a statement temporary (the one-report rule) — `compiler/src/own/place.rs`, `compiler/src/own/call_check.rs`, `compiler/src/own/statements.rs`, `compiler/src/own/lower.rs` ✅ (completed: 2026-10-15 08:13; the call path now descends into the view-returning call's single reborrowed source through the new `call_view_source`/`ViewCallSource`, so every arm contributes owners and the chained callee inherits them; the view-element type condition keeps the element sources in place in every position with the argument and struct-field positions adopting the backing for the destination's frame and a destination guard for returns; the return's array literal runs the view-element treatment; the local-owner `E10035` is withheld exactly where the lowered return reads a compiler-generated root or statement temporary (`temp_loans` empty signal; the controls keep one `E10035`, the parameter variant keeps `E10036` + `E10030`); deviations: `own/expr.rs` and `own/call.rs` were touched for the element guard and the argument adoption, and `own/call_check.rs::argument_owner` needed no change (no pinned case routes through it))
- [x] 2.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-15 08:13; residue spec 25/25 plain and under `RDC_ASAN=1`; `make check` exit 0 — 66 suites, no failures; the batch-H, callable-release, stage, compile-fail, and unit suites stay green; a flagged residual (a non-view struct carrying adopted views returned by value) is under review with its exact reproduction)

### Step 2.3: Impl Tests & Review

- [x] 2.3.1 Write the phase implementation pins (the owner shapes, the element bindings, the report counts and spans) — `compiler/tests/conditional_projection_residue_impl_test.rs` ✅ (completed: 2026-10-15 08:26; five pins added (22 tests total, additions only) for the call-wrapped owner walk and its two-arm/chained spellings (one `E10031` each, plus the accepted safe twin's inline shape), the element adoption (`ClBox __cl_owned0` in the argument and field positions), the `str[N]` return (the accepted emission, no false consume), the one-report rule (one `E10030` where withheld, one `E10035` for the controls), and re-emission determinism; each fails at `c148fc5` for the intended reason (recorded in the report); 22/22 green and `make check` exit 0)
- [x] 2.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses ✅ (completed: 2026-10-15 10:05; correctness, semantics, and memory-safety dispatched on the phase diff plus two correction rounds and the final confirmation — the carrier-return escape (a flow-sensitive carried-owner set with the return walk; one `E10030` for a generated/temp backing and one `E10035` for a local-only view), the enclosing-block store adoption (the owner binds at the function body scope), the deferred guard contract (both guarded argument paths materialize on the taken path and run nothing on the skipped path), the aggregate argument adoption, the reborrow report alignment, and the nested guard-store recursion (E-1, found by the final confirmation in the guards emitter); two pre-existing diagnostics rows filed (`DEF-282`, `DEF-283`) and one pre-existing leak row filed (`DEF-284`); the full record is in `05-findings.md`; the closing verification: the residue spec 36/36 plain and under ASan, the impl pins 22/22, the batch-H and callable-release oracles green, `make check` exit 0, and the 236-probe corpus differential with no base-clean program defective)

**Verify**: `make check`; the ownership cases under `RDC_ASAN=1`

**Deliverables**:
- [x] ST-7, ST-8, ST-9, ST-11, ST-28, ST-29 red then green (plus the review-round regression cases); ST-10 a green control (the reversed order — AR-25)
- [x] The batch-H suites and the `callable_release_integrity` oracle stay green

---

## Phase 3: Escaping stores

> **Phase baseline tree**: `dd2ca01` (the phase's fix-tree base; recorded 2026-10-15)
> **Lenses**: correctness + semantics + memory-safety + emission-integrity
> **Reasoning**: high — the escape rule is frozen; the over-rejection also changes acceptance
> and the store/capture shapes are emission-critical

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-escaping-stores.md) · [07](07-testing-strategy.md) ST-12..ST-15, ST-30..ST-33 · AR-5, AR-6, AR-20, AR-21
**Objective**: Pin the per-destination store judgment (globals/fields vs out-slots), the store and
capture spellings, and the scoped local element acceptance.

- [x] 3.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (globalstore, outslotstore, fieldstore, localelement, widenedstore, loopstore, staticelement, viewcapture) ✅ (completed: 2026-10-15 09:52; eight tests appended (+572 lines, additions only); two design-text inaccuracies corrected from measurement — the parameter-backed global store is accepted at the head (a red, not a control) and the non-nullable `borrow slot: str` out-slot spelling copies by value so no use-after-free reproduces there)
- [x] 3.1.2 Run the suite and record the red phase ✅ (completed: 2026-10-15 09:52; plain 36 passed / 8 failed and ASan 36 passed / 8 failed — exactly the eight new tests; the reds: the accepted global stores, the accepted out-slot fresh/local stores, the accepted widened stores, the accepted loop/nested-scope element stores, the field-store strict-build failure, the false E10033 rejections of the local element spellings, the static field's strict-build failure, and the capture's build failure; parent-verified)

### Step 3.2: Implementation

- [x] 3.2.1 Judge the stored view's provenance per destination kind (globals/fields `E10033`, out-slots `E10030`); lower the field store and the capture; scope the element-store loan to backings whose scope covers the array's — `compiler/src/own/statements.rs`, `compiler/src/own/expr.rs`, `compiler/src/own/place.rs`, `compiler/src/codegen/*` ✅ (completed: 2026-10-15 10:2x; the per-destination judgment (a new `check_escaping_view_store`: global/field destinations accept static/module-global owners only; caller slots accept static/parameter/global and report `E10030` otherwise), the element-wise judgment for the array spellings, the local element loan record with the per-scope rejection, and the store/capture spellings (the global field's projected type routes the fixed-array element-wise store; the environment literal initializes a fixed-array capture element-wise); deviations: `own.rs` and `codegen/expr.rs` were the actual sites; two in-round regressions were fixed (a `for (borrow e in xs)` alias is frame storage, and `slot = null` is skipped); five accepted programs changed emission (the previously invalid C) and no diagnostics changed beyond the intended ones)
- [x] 3.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-15 10:2x; residue spec 44/44 plain and under `RDC_ASAN=1`; the batch-H, callable-release, borrow-generic, and impl suites green; `make check` exit 0 with no warnings; the 40-probe corpus shows the intended rejections/acceptances and the five emission deltas)

### Step 3.3: Impl Tests & Review

- [x] 3.3.1 Write the phase implementation pins (the report spans, the store/capture shapes, the re-emission determinism pins) — `compiler/tests/conditional_projection_residue_impl_test.rs` ✅ (completed: 2026-10-15 11:45; seven pins added (35 tests total, additions only) — the rejection shapes with store-line spans, the element-wise global-field store, the capture initializer, the accepted local element store, and re-emission determinism; red-phase wording corrected per the confirmation: eight of the nine new spec cases and three of the six new pins are red at `acee2cb`, the remainder are proven by single-mechanism mutation trees, and the alias-shape pin is an identity control (its emission is byte-identical base-to-head); 35/35 green and `make check` exit 0)
- [x] 3.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses ✅ (completed: 2026-10-15 12:0x; four lenses dispatched (correctness, semantics, memory-safety, emission-integrity) plus four targeted confirmations; the review round's one merged class (the over-rejection), the take class, the `for`-alias class, the literal parity, the one-report boundary, the stale marks/loans, and the hygiene items all fixed; the post-confirmation PC-1/SC-1 follow-up accepts the parameter-backed carrier literals (`LiteralViews`); seven rows filed (`DEF-285`..`DEF-291`, `DEF-282` extended); the full record is in `05-findings.md`; the closing verification: the residue spec 55/55 plain and ASan, the impl pins 35/35, the batch-H/callable-release suites green, `make check` exit 0 with no warnings)

**Verify**: `make check`; the ownership cases under `RDC_ASAN=1`

**Deliverables**:
- [x] ST-12..ST-15, ST-30..ST-33 red then green (plus the review-round regression cases)
- [x] No accepted safe program newly rejected without a frozen-rule citation (the confirmation's over-rejection families were fixed; the remaining rejections carry the frozen-text citations)

---

## Phase 4: Guarded and discarded literal positions

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + memory-safety
> **Reasoning**: high — the discarded/guarded paths are the phase-4 fix territory of batch H

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-guarded-literal-positions.md) · [07](07-testing-strategy.md) ST-16..ST-18, ST-34 · AR-12, AR-22
**Objective**: Pin the loop-condition conditional literal (both literal forms), the `?:`/`||`
positions, the discarded mixed-field literal, and the discarded fresh projection arm.

- [x] 4.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (loopcondliteral, literalpositions, discardmixed, discardprojection) ✅ (completed: 2026-10-15 12:20; four tests appended (+265 lines, additions only) — the loop-condition conditional literal (single-shot and counted, both selections, plus the array-literal element variant), the receiver positions (`?:` and `||`, plus the loop-condition spelling of the `||` receiver), the discarded mixed-field literal with its direct control, and the discarded projection arm; the field/element spellings were chosen to keep the DEF-262 double-free reproducible; two design-uncovered probes were recorded — the conditional carrier literal's taken-arm leak (filed as `DEF-292`) and the guarded fixed-array field expression (added to the phase-5 candidates in the ledger))
- [x] 4.1.2 Run the suite and record the red phase ✅ (completed: 2026-10-15 12:20; plain 56 passed / 3 failed and ASan 55 passed / 4 failed — the double-free in the loop-condition literal and the `||` receiver's loop spelling, the discarded mixed-field literal's false `E10020` moves, and the discarded projection arm's 22 B leak; the standalone receiver-position spellings are pre-closed (green); the 55 pre-existing cases stay green; parent-verified)

### Step 4.2: Implementation

- [x] 4.2.1 Reuse the direct-literal per-arm binding for the loop-condition walk; make the discard decision per field (the place fields stay reads); bind the discarded arm's projection root ✅ (completed: 2026-10-15 12:45; the loop-condition walk's `Materialize` arm ignores a wholesale operand (`materialize_stores_wholesale`), so a literal/call/array/conditional arm stores straight into the carrier without a second owner; the discarded-conditional flag makes a literal field/element a read while the fresh fields materialize for release (`defer_discarded_arm` + `materialize_discarded_field`); a projection arm binds its ownerless root via `bind_guarded_projection_root`; an extra acceptance (`litfieldproj`) was fixed in the same pass; one base-equal candidate (a non-owning selected type over an owning root) was filed as `DEF-293`) — `compiler/src/own/lower.rs`
- [x] 4.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-15 12:45; residue spec 59/59 plain and under `RDC_ASAN=1`; exactly four emission and two diagnostic deltas base-to-head, all intended; batch-H, callable-release, and `make check` green with no warnings; parent-verified 59/59 plain + ASan)

### Step 4.3: Impl Tests & Review

- [x] 4.3.1 Write the phase implementation pins (the per-field shapes, the root binding, the release counts, the re-emission determinism pins) — `compiler/tests/conditional_projection_residue_impl_test.rs` ✅ (completed: 2026-10-15 13:40; four pins added (39 tests total, additions only) — the loop-condition walk's wholesale store, the discarded mixed-field per-fresh-owner materialization, the discarded projection root binding, and re-emission determinism; each fails at `f853ef9` for the intended reason (the per-field temp copied into the carrier and released twice; the `E10020` moves; the unbound root); 39/39 green and `make check` exit 0)
- [x] 4.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses ✅ (completed: 2026-10-15 13:40; correctness and memory-safety dispatched plus three targeted confirmations; the corrections: the discarded aggregate-field recursion (P4R-1/F-1), the discarded-array-literal-arm flag scoping (P4R-2), and the discarded-field projection read redirect (MEM-1, closing `DEF-296`); rows filed: `DEF-294`..`DEF-297` plus the phase-5 candidate note; the closing verification: the residue spec 63/63 plain and ASan, the impl pins 39/39, the batch-H and callable-release suites green, `make check` exit 0 with no warnings)

**Verify**: `make check`; the ownership cases under `RDC_ASAN=1`

**Deliverables**:
- [x] ST-16..ST-18, ST-34 red then green (plus the review-round regression cases)
- [x] The batch-H discard pins stay green

---

## Phase 5: Fixed-array conditional arms and projections

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + emission-integrity + memory-safety
> **Reasoning**: high — the hidden out-parameter ABI and the address shapes are emission-critical

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-fixed-array-arms.md) · [07](07-testing-strategy.md) ST-19..ST-22, ST-35 · AR-11
**Objective**: Pin the fixed-array conditional arm, the mixed literal field, and the
conditional-root (a move-only root included)/`for`-iterable projections.

- [x] 5.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (condarmfixed, mixedliteral, condroot, foriterable, moveonlyroot) ✅ (completed: 2026-10-15 14:20; six tests appended (+356 lines, additions only) — the fixed-array conditional's discard and consuming store with per-arm counters, the mixed fixed-array/String literal (the call-filled field and the literal-arm spelling), the conditional root's store and return projections, the `for` iterable, the move-only root, and the direct controls; three candidate shapes were filed (`DEF-298`..`DEF-300`), two more (the discarded all-fresh fixed-array literal and the direct call-filled field) are handled with the implementation, and the design's static-field claim was noted as not reproducing)
- [x] 5.1.2 Run the suite and record the red phase ✅ (completed: 2026-10-15 14:20; plain 64 passed / 5 failed and the same under `RDC_ASAN=1` (all strict-build failures) — the discarded conditional's call arms emit `too few arguments`, the mixed literal's call field likewise, and the conditional-root store/return plus the `for` iterable emit `lvalue required as unary '&'`; the store spelling, the literal-arm mixed literal, and the three direct controls are green; parent-verified)

### Step 5.2: Implementation

- [x] 5.2.1 Route each fixed-array arm through the per-arm destination store; bind each arm's root and render the conditional root as the per-arm address form; route the `for` iterable through the projection preparation ✅ (completed: 2026-10-15 15:10; the discarded fixed-array conditional and the call-filled literal field take a `__cl_discard` destination filled and released per arm (`codegen/function.rs`); the conditional root's projection distributes per arm in the store/return gates (`distribute_conditional_projection`, `roots_in_conditional`); the `for` iterable binds an ownerless projection root; the guards emitter fallback is replaced by the per-arm destination store; deviation: the design's sketch `(c ? &b1.xs : &b2.xs)` is realized as the existing per-arm store core (an if/else with one copy per arm — same semantics, no new C construct); the two folded shapes (x4, q5) are fixed with cases; three base-equal residuals were filed as `DEF-301`..`DEF-303`) — `compiler/src/own/lower.rs`, `compiler/src/own/loops.rs`, `compiler/src/codegen/guards.rs`, `compiler/src/codegen/function.rs`
- [x] 5.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-15 15:10; residue spec 71/71 plain and under `RDC_ASAN=1`; the batch-H, callable-release, and hardening suites green; `make check` exit 0; 18 of 43 probe programs changed emission, all in the intended classes, the direct controls byte-identical; parent-verified 71/71 plain + ASan)

### Step 5.3: Impl Tests & Review

- [x] 5.3.1 Write the phase implementation pins (the per-arm destination shapes, the per-arm address form, the bound roots, the re-emission determinism pins) — `compiler/tests/conditional_projection_residue_impl_test.rs` ✅ (completed: 2026-10-15 15:35; seven pins added (46 tests total, additions only) — the discarded conditional's `__cl_discard` destination with its per-arm fills, the owning-element destination's releases, the call-filled literal field's per-field build, the conditional-root store and return copies (no root value copy), the `for`-iterable root binding, and re-emission determinism (an identity control, as in phase 3); six of seven fail at `91decc0` on their first assertion for the intended reasons; 46/46 green and `make check` exit 0)
- [x] 5.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the lenses ✅ (completed: 2026-10-15 16:35; three lenses dispatched (correctness, memory-safety, emission-integrity) plus four targeted confirmations; the corrections: the in-place conditional-root iterable (no materialization/move), the ownerless arm roots binding per arm (closing `DEF-301`), and the nested-arm distribution recursion with lazily-guarded pins (closing `DEF-306`'s store/return/iterable positions); rows filed: `DEF-301`..`DEF-307`, `DEF-281` extended, `DEF-298`..`DEF-300` stay filed; the closing verification: the residue spec 77/77 plain and ASan, the impl pins 46/46, the batch-H and callable-release suites green, `make check` exit 0 with no warnings)

**Verify**: `make check`; the ownership cases under `RDC_ASAN=1`

**Deliverables**:
- [x] ST-19..ST-22, ST-35 red then green (plus the review-round regression cases)
- [x] The emission differential shows only intended deltas (4559 programs: 39 deltas, all intended classes; no head-only strict failure; the deviation verified)

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
