# Execution Plan: Memory and Value Integrity (Batch N)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-18
> **Progress**: 15/62 tasks (24%) — Phase 2 complete
> **Planning baseline**: `f6da78f` (the batch-M close-out commit); the preflight audits ran on
> `100180c` (the plan commit; compiler sources unchanged)
> **CodeOps Artifact Schema**: 1

## Overview

Close the 23 rows named by the dispatch — 9 🔴 criticals and 14 🟠 majors — in seven machinery
phases and a close-out. `DEF-425` is parked pending the user's ruling, leaving 22 active rows
(9 criticals + 13 majors). Twenty active rows are red at `f6da78f` and are implemented red →
green; two (`DEF-427`, `DEF-447`) are verified not reproducible and are closed as false positives
with green controls and probe evidence (AR-27, AR-28); `DEF-425` is recorded as parked with its
conflict evidence and ruling request (AR-22). The scope is severity-first: every critical lands in
P1–P3, and every phase follows the specification-first ordering
(spec cases → red → implement → green → impl pins → review). The new cases append to the origin
suites (AR-30); the probe-verified direct twins (the `E10030`/`E10034`/`E10026` controls, the
`if`-condition literal control, and the `String`/`Box` array controls) stay green as pinned
controls. Every fix extends existing machinery; no new diagnostic, no new IR channel, no new test
suite pair (AR-33, AR-34).

The pre-phase step runs `analyze-agents` before phase 1, because new plan documents landed; the
close-out refreshes the specialist check again (8.1.4). The `DEF-425` park record is a phase-5
ledger item, not a fix task.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Loan transfer on stored copies | 9 |
| 2 | Loan sources through runtime, temporary, and call reads | 8 |
| 3 | Raw-deref move-outs | 8 |
| 4 | Store lowering and evaluation order | 8 |
| 5 | Per-instance generic judgments | 9 |
| 6 | Literal destinations and conditional arrays | 8 |
| 7 | Control resolution (`DEF-427`, `DEF-447`) | 8 |
| 8 | Close-out | 4 |

**Total: 62 tasks across 8 phases**

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

## Phase 1: Loan transfer on stored copies

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-428` (critical), `DEF-429` (critical, primary), `DEF-431` (critical, verification
> branch), `DEF-435` (major, verification branch)
> **Lenses**: correctness + semantics + memory-safety (AR-31; no emission lens for the loan
> bookkeeping)
> **Reasoning**: high — the fixes change when and where element loans are recorded; a wrong path
> or order either misses the escape or rejects a legal copy, and the frozen `E10031`
> write-conflict reading must stay

### Step 1.0: Pre-flight

- [x] 1.0.1 [orchestrator] Run `analyze-agents` before phase 1 and record the specialist-check ✅ (completed: 2026-10-18; `analyze-agents` ran before phase 1 — outcome None (the four project agents cover the recurring areas); `codeops/specialist-check.json` refreshed with 39 plans; the close-out 8.1.4 refreshes it again)
      state (the `AGENTS.md` rule after new plans land); the close-out refreshes it again in 8.1.4

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-loan-store-transfers.md) · [07](07-testing-strategy.md) ST-1..ST-4 ·
AR-6, AR-7, AR-9, AR-12
**Objective**: Pin the exact self-assignment, the composite element/projected copy, the deeper
projection, and the element-store nested fill, with their direct controls.

- [x] 1.1.1 [spec-author] Write the phase cases in ✅ (completed: 2026-10-18; four spec cases added: the exact self-assignment (`s = s`, `ys = ys`) with the direct and in-scope controls; the composite element/projected/literal/runtime copy destinations with the direct conditional fill, the runtime direct/copy forms, and the constant sibling; the deeper projection copy with its direct control; the element-store constant/runtime fill copies with the constant direct read; the runtime direct read is a recorded probe (AR-35(e)), not a spec assertion)
      `compiler/tests/conditional_projection_residue_spec_test.rs` (selfassignloans,
      compositeelementstore with the runtime-destination and constant-sibling controls,
      deeperprojectioncopy, elementstorenestedfill with the constant and runtime halves) with their
      direct controls; the red cases committed first
- [x] 1.1.2 Run the suite and record the red phase: the red cases fail without the fix (the ✅ (completed: 2026-10-18; the red phase recorded: residue 190 passed / 4 failed; every red shape accepted at head with an ASan heap-use-after-free; the direct reads (13:5/15:5), the in-scope reads, the constant sibling, and the batch-M `E10031` controls green)
      expected `E10030` is absent), the direct controls are verified green first, and the
      batch-M loan/E10031 controls stay green (AR-3)

### Step 1.2: Implementation

- [x] 1.2.1 Complete the element-loan transfer in this order: (a) skip the exact self-assignment ✅ (completed: 2026-10-18; the exact self-assignment skip `own/statements.rs:805`/`own/expr.rs:1478`; the composite gate removed + the wildcard record `own/expr.rs:1302`/`own/place.rs:1123-1139`; the position readers `place.rs:927/1001/1224/1393` with `position_starts_with`/`position_covers`/`position_step_matches`/`position_rest`; `DEF-431` closed **incidentally** under the same gate (no code); `DEF-435`'s constant half closed incidentally under the gate, and its runtime half under the `Element(None)` wildcard record (not the gate; a fixed extent now expands the wildcard into its candidate elements in `own/place.rs`, so a later constant store clears its own slot); the runtime direct read now one `E10030` (the AR-35(e) intended delta)
      replacement only (`own/statements.rs`, `own/place.rs:1096`; reuse
      `is_exact_self_assign`, `own/expr.rs:1481-1493`); (b) remove the `projections.is_empty()`
      gate and pass the destination path to the collector (`own/expr.rs:1299-1306`), recording a
      wildcard `Element(None)` for the runtime destination; the non-constant branch of
      `replace_element_loans` (`own/place.rs:1119-1127`) records the wildcard path with the union
      in `element_loans` and keeps the binding union for `E10031`; make the three position readers
      (`collect_place_copy_loans`, `move_element_loans`, `place_recorded_position_owners`, and the
      shared `recorded_position_frame_owners`) match a recorded `Element(None)` against any
      `Element(_)` at the same position prefix; (c) re-run ST-3/ST-4. If the `429`
      gate greens `DEF-431`/`DEF-435`, record the incidental closure with evidence; otherwise add
      the prefix fallback to `move_element_loans` (`own/place.rs:1212`) for `DEF-431` and the
      collector call in `emit_indexed_store` (`own/expr.rs:2557-2629`) for `DEF-435`'s runtime
      half. Record the constant direct read (`d435b`, `E10030`) and the runtime direct read
      (`q435runtime_direct`) separately per AR-35(e). Do not union every binding loan onto every
      read (the `ys[1]` sibling control).
- [x] 1.2.2 Run the suite to green; on any failure STOP and fix the implementation (or park per ✅ (completed: 2026-10-18; residue 194/194 and borrow-store 30/30 plain and under `RDC_ASAN=1`; `make check` exit 0; the differential shows the 11 intended `E10030` sites and zero emitted-C delta; the constant sibling and the in-scope reads clean)
      AR-33 if a frozen code is insufficient); run `make check`

### Step 1.3: Impl Tests & Review

- [x] 1.3.1 Write the phase implementation pins (the loan record paths, the `Element(None)` wildcard ✅ (completed: 2026-10-18; seventeen impl pins: the self-assignment loan records (single/array), the conditional/literal element copies per arm/element, the runtime wildcard through the direct/copy/element-store/whole-row readers, the deeper projection, the constant/runtime element-store fills, and four declared controls (the in-scope emissions, the constant sibling, the byte-identical re-emission); the base discrimination 13/4 at `70ef134`; residue impl 188/188, plain and ASan)
      record and its three-reader match, the report code/count/span per shape, the accepted copy
      emissions, and a re-emission determinism pin) in `conditional_projection_residue_impl_test.rs`
- [x] 1.3.2 Phase review — dispatch the correctness, semantics, and memory-safety lenses; apply ✅ (completed: 2026-10-18; the three lenses dispatched: correctness FAIL (the wildcard over-report blocker), semantics PASS WITH MINORS, memory CONFIRMED (0 in-diff; `DEF-449` filed); the correction round expanded the wildcard over the fixed extent, pinned the acceptances and the intended deltas, and added the position-match unit tests)
      the corrections; run the host ASan/LSan probes. The reviewer packet includes the
      blast-radius step (every `set_binding_loans`, `replace_element_loans`,
      `record_element_view_loans`, `place_recorded_position_owners`, `move_element_loans`,
      `record_copy_element_loans`, `collect_place_copy_loans`, `recorded_position_frame_owners`, and
      the `emit_indexed_store` caller) and the test-integrity step
- [x] 1.3.3 Bounded confirmation: re-run the phase probes and the focused suite after the ✅ (completed: 2026-10-18; the bounded confirmation: correctness PASS WITH MINORS (0 blockers; N1C-1/2, N1S-2 closed; the sub-row conservatism recorded); the ASan matrix and the focused suites green)
      corrections; record FAIL/PASS per round

### Step 1.4: Phase Record

- [x] 1.4.1 Phase record: append the verdicts, fix rounds, and filed rows to `05-findings.md`; ✅ (completed: 2026-10-18; the ledger section recorded; the phase-2 note for `DEF-430`; progress 9/62)
      update the Progress block and commit per the active commit mode

**Verify**: `make check`; the residue suite plain and `RDC_ASAN=1`; the host ASan/LSan probes; the
strict builds

**Deliverables**:
- [ ] ST-1..ST-4 red then green; the direct controls stay green; the constant sibling stays clean
- [ ] The `DEF-431`/`DEF-435` verification-branch outcome is recorded (incidental or fix)
- [ ] The batch-M loan matrix and the `E10031` controls stay green

---

## Phase 2: Loan sources through runtime, temporary, and call reads

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-430` (critical), `DEF-432` (critical), `DEF-433` (critical)
> **Lenses**: correctness + semantics + memory-safety (AR-31)
> **Reasoning**: high — the runtime union is conservative, the temporary owner must release once,
> and the binding-loan fallback must not shadow the element positions (the batch-M one-report
> rule)

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-loan-sources.md) · [07](07-testing-strategy.md) ST-5..ST-7 ·
AR-8, AR-10, AR-11
**Objective**: Pin the runtime store/constant read, the temporary-derived element, and the
call-mediated view result, with their direct controls and the `k = 0` variant.

- [x] 2.1.1 [spec-author] Write the phase cases (runtimestorecopy, temporaryelementcopy, ✅ (completed: 2026-10-18; `DEF-430` re-probed and **reclassified as an incidental closure** (the phase-1 fixed-extent wildcard already reports one `E10030` for the k=0/k=1 copies and the direct runtime reads; the constant sibling accepts clean) — ST-5 is a green control; new spec cases: the temporary-derived element copy (`[mk().v; "EE"]`; red) and the call-mediated view result (`let s = pick(xs); fill(s)` and `fill(pick(xs))`; red), with the direct twins, the in-scope reads, and the caller-parameter acceptance)
      callviewresult) with their direct controls, the `DEF-430` `k = 0` conservative variant, the
      constant-store sibling acceptance, and the caller-parameter view acceptance; the red cases
      committed first
- [x] 2.1.2 Run the suite and record the red phase; verify the direct controls green first and ✅ (completed: 2026-10-18; the red phase recorded: residue 198 passed / 2 failed (the temporary element and call-result cases); the `DEF-430` control, the sibling, the in-scope reads, and the caller-parameter view green; the direct runtime reads pinned as the AR-8 intended delta)
      record the `DEF-430` direct-runtime-fill blast-radius outcome (accepted or rejected) per
      AR-8

### Step 2.2: Implementation

- [x] 2.2.1 Extend the loan sources: record the wildcard `Element(None)` runtime position and ✅ (completed: 2026-10-18; `DEF-432`: the lowered value/root-owner map threaded through `record_copy_element_loans`/`collect_copy_element_loans`/`collect_place_copy_loans` (`own/place.rs:1337/1378/1524`), with `frame_view_owner` rendering the generated owner; `DEF-433`: the reborrow-call position source (`place.rs:1498`), `value_loan_frame_owners` (`:1735`), and the cross-frame fallback (`own/call_check.rs:587`); `DEF-430` incidental (no code); a take-materialized temp element reports two `E10030`s (mirroring the pre-existing single-view twin) — recorded for the review)
      match only reads under the runtime-written path (`own/place.rs:1119-1127`, `:1369`,
      `:1192`; the non-constant branch records the wildcard and keeps the binding union for
      `E10031`, and `place_recorded_position_owners`/`recorded_position_frame_owners` share the
      same `Element(None)`-matches-`Element(_)` prefix rule); thread the lowered value/root-owner
      map (`view_owners`/`lowered_root_bindings`, `:1569`/`:1603`) from
      `check_let`/`check_assign` and record the non-place owner (`:1421-1428`); consult
      `view_loans`/`loan_owners` for the direct call form and the binding-loan owners in the
      cross-frame fallback (`own/call_check.rs:585`; `own/place.rs:1512`, `:815`)
- [x] 2.2.2 Run the suite to green; on any failure STOP and fix (or park per AR-33); run ✅ (completed: 2026-10-18; residue 200/200 and impl 193/193 plain and under `RDC_ASAN=1`; borrow-store 30/11; `make check` exit 0; the differential: 6 new reject rows (3 unique sources), zero emission deltas, zero head-accepts, no head-only strict failure)
      `make check`

### Step 2.3: Impl Tests & Review

- [x] 2.3.1 Write the phase implementation pins (the wildcard `Element(None)` record and its ✅ (completed: 2026-10-18; nine impl pins: the temporary-element owner (`__cl_owned0`/statement-temporary), the call-result binding loans (copy and direct, fallback-before-provenance), and six declared controls (the temporary emission, the caller-parameter view, the runtime-store copies/direct reads, the constant sibling, the byte-identical re-emission); the base discrimination 3/6 at `f1ab264`; residue impl 202/202)
      prefix match, the temp-owner record including `__cl_owned0`, the binding-loan fallback for
      both forms, the report code/count/span, the sibling/caller-parameter acceptances, and a
      re-emission determinism pin)
- [x] 2.3.2 Phase review — dispatch correctness, semantics, and memory-safety; apply the ✅ (completed: 2026-10-18; the three lenses dispatched: correctness PASS WITH MINORS, semantics FAIL (N2S-1; filed `DEF-451`, claim narrowed), memory CONFIRMED (0 in-diff; `DEF-450`/`451` filed); the correction round pinned the conservative boundaries, the phrase, and the assignment/nested call forms; `DEF-453` filed)
      corrections; run the host ASan/LSan probes. The packet includes the blast-radius step
      (`replace_element_loans`, `record_element_view_loans`, `collect_place_copy_loans`,
      `move_element_loans`, `place_recorded_position_owners`, `recorded_position_frame_owners`,
      `frame_owned_view_owner`) and the test-integrity step
- [x] 2.3.3 Bounded confirmation: re-run the phase probes and the focused suite; record the ✅ (completed: 2026-10-18; the bounded confirmation CLEAN: N2S-1 scoped/filed, the minors closed, the mutation check, the suites, and `make check` green)
      rounds

### Step 2.4: Phase Record

- [x] 2.4.1 Phase record: append to `05-findings.md` (including the `DEF-430` direct-fill ✅ (completed: 2026-10-18; the ledger section recorded; progress 15/62)
      outcome); update Progress; commit per the active commit mode

**Verify**: `make check`; the residue suite plain and `RDC_ASAN=1`; the ASan/LSan probes; the
strict builds

**Deliverables**:
- [ ] ST-5..ST-7 red then green; the direct controls stay green
- [ ] The `DEF-430` direct-runtime-fill outcome is recorded (intended delta or filed residual)

---

## Phase 3: Raw-deref move-outs

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-437` (critical), `DEF-439` (major), `DEF-440` (critical), `DEF-441` (critical)
> **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-31; the channel is
> IR/emission)
> **Reasoning**: high — the materializer and the `value_move_outs` channel decide when a pointee
> is read and emptied; the guard paths must not evaluate or free on the skipped path

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-deref-move-outs.md) · [07](07-testing-strategy.md) ST-8..ST-11 ·
AR-13, AR-14, AR-15, AR-16
**Objective**: Pin the projected raw pointee, the guarded aliasing take, the borrowing intrinsic,
and the guarded member take, with both guard selections and the false-path controls.

- [ ] 3.1.1 [spec-author] Write the phase cases (rawderefprojection, guardedtargetalias,
      guardedintrinsicderef, guardedmembertake) with the isolated store/return/field forms, both
      selections, the distinct-length false path (`v = "vv"`, `y = "yyyy"`), the non-aliasing
      control, the two-owned-member sibling read, and the batch-M discard/non-taking controls; the
      red cases committed first
- [ ] 3.1.2 Run the suite and record the red phase (the isolated store/return/field forms, the
      five consuming shapes, and both guard selections); verify the controls green first

### Step 3.2: Implementation

- [ ] 3.2.1 Extend the materializer to a `Deref` anywhere in a projection chain
      (`own/lower.rs:3699-3721`, used by the store/return/field/take paths); extend the three
      emission gates to the chain — `moves_pointee` (`codegen/function.rs:1382-1384`),
      `return_source_needs_emptying` (`codegen/cleanup.rs:545-567`), and
      `struct_literal_needs_stores`/its field loop (`codegen/function.rs:2071-2094`,
      `:1421-1432`); apply the per-arm runtime alias check for the consumed save
      (`codegen/function.rs:2400-2444`); return the `Deref`-rooted chain unchanged from
      `bind_intrinsic_argument` and the non-take rule (`own/lower.rs:4171-4234`, `:5393-5401`);
      record the projected member (not the root) for a `Deref`-rooted take in
      `take_pointee_move_outs` (`own/call.rs:804-821`) and extend the take-path pin
      (`own/lower.rs:5239-5262`) to the chain
- [ ] 3.2.2 Run the suite to green; on any failure STOP and fix (or park/record per AR-33,
      AR-35(b)); run `make check`

### Step 3.3: Impl Tests & Review

- [ ] 3.3.1 Write the phase implementation pins (the single call per pointer expression, the
      read/empty pointer identity, the projected-member `MoveOutValue` placement inside the
      evaluated path, the emitted alias condition, the false arm's store of `y` under distinct
      lengths, the absent false-path call, and a re-emission determinism pin)
- [ ] 3.3.2 Phase review — dispatch all four lenses; apply the corrections; run the ASan/LSan
      probes and the base/head emission differential over the P3 probe set and the corpus. The
      packet includes the blast-radius step (every `materialize_owning_deref` call,
      `take_pointee_move_outs` callee kind, `arm_consumes_target` shape) and the test-integrity
      step
- [ ] 3.3.3 Bounded confirmation: re-run the phase probes and the focused suite; record the
      rounds

### Step 3.4: Phase Record

- [ ] 3.4.1 Phase record: append to `05-findings.md`; update Progress; commit per the active
      commit mode

**Verify**: `make check`; the residue suite plain and `RDC_ASAN=1`; the ASan/LSan probes; the
emission differential

**Deliverables**:
- [ ] ST-8..ST-11 red then green; both guard selections clean
- [ ] The batch-M `m375_*`/`m376_*` controls stay green

---

## Phase 4: Store lowering and evaluation order

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-421` (major), `DEF-423` (major), `DEF-424` (major), `DEF-436` (major)
> **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-31; semantics owns
> the place rule and the order)
> **Reasoning**: high — the store target, the pin, the clear order, and the materializer decide
> what is written and which owner is freed; a broad `place_target` change reaches every consumer

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-store-lowering-order.md) · [07](07-testing-strategy.md)
ST-12..ST-15 · AR-17, AR-18, AR-19, AR-20
**Objective**: Pin the `Deref`-rooted store forms, the whole-element target order and bounds trap,
the fixed-array self-read/self-copy, and the element/raw-target RHS materialization.

- [ ] 4.1.1 [spec-author] Write the phase cases (derefrootedstore, elementtargetorder,
      fixedarrayselfread, derefrhistore) with the indexed `bump` counter, the conditional-base and
      structural-`.len` controls, the `assert_traps` bounds case, the `x = *(&x)`/nested/two-element
      self-reads with the distinct-binding/place controls and a following distinct-value store
      through each raw-self-alias place (so a lost store is visible), the `at1` residual, and the
      duplicable controls; the red cases committed first
- [ ] 4.1.2 Run the suite and record the red phase; verify the controls green first

### Step 4.2: Implementation

- [ ] 4.2.1 Complete the store lowering: route the `Deref`-rooted `Member`/`Index` target through
      the raw-deref store path in `check_assign` (`own/expr.rs:1003-1073`), and do **not** add a
      `Deref` arm to `place_target`; pin the `Deref`-rooted target's base/index before the RHS with
      `pin_store_target` (`own/expr.rs:2393-2421`) and make the whole-element base/index pin
      unconditional (`:2490-2566`); evaluate the fixed-array literal self-read before the clear,
      treat the identical constant-index `Move` and the raw self-alias as no-ops
      (`codegen/function.rs:1557-1592`, `:659-690`), and implement the raw self-alias with the
      **`is_raw_self_alias` recognizer** (new beside `is_exact_self_assign`,
      `own/expr.rs:1481-1493`): unwrap `Deref(AddressOf(place))` and compare the inner place, or
      compare the rendered addresses; keep `place_target` `Deref`-free and pin the two self-alias
      forms in ST-14 with the distinct-binding/place controls and a following distinct-value store,
      so a lost store is visible; run `materialize_owning_deref` on the element/raw-target
      RHS (`own/expr.rs:2603-2622`, `:1055-1076`). Keep the structural `.len` `E10221` branch
      (`own/expr.rs:1448-1461`). File the `at1` use-after-move row at the phase record; do not fix
      it here.
- [ ] 4.2.2 Run the suite to green; on any failure STOP and fix (or stop for a profile/new-channel
      ruling per AR-35(a)(b)); run `make check`

### Step 4.3: Impl Tests & Review

- [ ] 4.3.1 Write the phase implementation pins (the target projection, the pinned base/index, the
      store/RHS order, the self-copy/`is_raw_self_alias` skip including a distinct-place refusal,
      the per-pointer-expression call count, and a re-emission determinism pin)
- [ ] 4.3.2 Phase review — dispatch all four lenses; apply the corrections; run the ASan/LSan
      probes, the `assert_traps` cases, the strict builds, and the base/head differential. The
      packet includes the blast-radius step (every `place_target`, `indexed_target`,
      `emit_indexed_store`, fixed-array assign, and `Statement::Move` consumer) and the
      test-integrity step
- [ ] 4.3.3 Bounded confirmation: re-run the phase probes and the focused suite; record the
      rounds

### Step 4.4: Phase Record

- [ ] 4.4.1 Phase record: append to `05-findings.md`; update Progress; commit per the active
      commit mode

**Verify**: `make check`; the residue suite plain and `RDC_ASAN=1`; the strict builds; the
`assert_traps` cases; the hosted-amiga compile leg; the freestanding compile-only leg for the
eligible scalar forms with the per-case `E11200` reasons; the emission differential

**Deliverables**:
- [ ] ST-12..ST-15 red then green; the member-route and conditional-base controls stay green
- [ ] The `E11211` bounds trap and the `E10221` conditional-base rejection are unchanged
- [ ] The `at1` source read is filed as a residual row (no fix)

---

## Phase 5: Per-instance generic judgments

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-410` (major), `DEF-426` (major); `DEF-425` (major) is parked with its evidence
> and ruling request
> **Lenses**: all four (AR-31; the rejection is memory/ownership; the guard is emission)
> **Reasoning**: high — the per-instance audit must report only for an owning substitution and
> must not report at the definition; the division guard must match the concrete twin exactly

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-per-instance-generic.md) · [07](07-testing-strategy.md)
ST-16..ST-17 · AR-21, AR-22, AR-23
**Objective**: Pin the generic conditional store (`E10034` per owning instance), the separate
duplicable generic whole move, the concrete twin, and the generic division guard; record the
`DEF-425` park evidence.

- [ ] 5.1.1 [spec-author] Write the phase cases in
      `borrow_generic_integrity_spec_test.rs` (genericconditionalstore) and
      `conditional_projection_residue_spec_test.rs` (genericdivisionguard), with the owning, the
      duplicable-generic-whole-move at `T = P` (`q410dup`), and concrete controls; the red cases
      committed first
- [ ] 5.1.2 Run the suites and record the red phase (the generic body accepted; the concrete twin
      rejecting; the generic guard absent); verify the controls green first

### Step 5.2: Implementation

- [ ] 5.2.1 Record the conditional type-parameter arm as a maybe-owning source in
      `prepare_binding_effect` (`own/statements.rs:816-887`; `RequiresNotLend`, the per-instance
      audit `check_not_lend_requirements`), not by calling `move_place` at the definition
      (`conditional_move_sources`, `own/expr.rs:175-205`); resolve the division guard's unknown
      projection type with `resolved_place_ty`/`resolved_value_ty` (`codegen/expr.rs:321`, `:397`,
      both already at head) in `division_guard_condition` (`codegen/guards.rs:1444-1460`). The P5
      scope is `DEF-410`/`DEF-426` only; `DEF-425` is parked. The parked route, if the user rules
      for a fix, is `RequiresNotLend` + `project_place_path` + `check_not_lend_requirements` — not
      the `E10026` duty.
- [ ] 5.2.2 Run the suites to green; on any failure STOP and fix (or record per AR-35(f)); run
      `make check`

### Step 5.3: Impl Tests & Review

- [ ] 5.3.1 Write the phase implementation pins (the `E10034` record/count/span, the duplicable
      generic whole-move acceptance, the no-report-at-the-definition pin, the guard count/text in
      the generic and concrete emissions through the resolved projection type, and a re-emission
      determinism pin)
- [ ] 5.3.2 Phase review — dispatch all four lenses; apply the corrections; run the ASan probes
      and the `assert_traps` division cases; run the generic/concrete emission differential. The
      packet includes the blast-radius step (`conditional_move_sources` arms,
      `mark_moved_if_owning_at`/`note_borrowed_element_move` callers, the not-lend/element duty
      consumers, every division guard site) and the test-integrity step
- [ ] 5.3.3 Bounded confirmation: re-run the phase probes and the focused suites; record the
      rounds

### Step 5.4: Phase Record

- [ ] 5.4.1 Phase record: append to `05-findings.md`; update Progress; commit per the active
      commit mode
- [ ] 5.4.2 [orchestrator] Record the `DEF-425` park: the frozen BR-5 conflict, the pinned
      batch-M spec/impl case paths, the `d425a`/`d425b`/`d425d` probes, and the explicit ruling
      request; no code/spec/test edit. Carry the park to the close-out (8.1.2)

**Verify**: `make check`; the borrow-generic and residue suites plain and `RDC_ASAN=1`; the
`assert_traps` `E11210` cases; the emission differential

**Deliverables**:
- [ ] ST-16..ST-17 red then green; the concrete/take twins keep their reports
- [ ] The generic and concrete division emissions agree
- [ ] The `DEF-425` park and its ruling request are recorded; no code change

---

## Phase 6: Literal destinations and conditional arrays

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-445` (major), `DEF-446` (major), `DEF-448` (major)
> **Lenses**: all four (AR-31; the literal store form is emission and ordering)
> **Reasoning**: high — the predicates and the guarded/conditional store form decide whether the
> hidden destination is bound and which arm evaluates; the unchosen/trailing owners must release
> once

### Step 6.1: Specification Tests

**Reference**: [03-06](03-06-literal-destinations.md) · [07](07-testing-strategy.md)
ST-18..ST-20 · AR-24, AR-25, AR-26
**Objective**: Pin the conditional-of-literals read base (and nested field/element), the
ternary-arm take literal, and the conditional array literal of places, with their controls.

- [ ] 6.1.1 [spec-author] Write the phase cases (conditionalliteralread, ternarytakeliteral,
      conditionalarraytake) with the mixed place/literal read base, the brace-list, `if`-condition,
      `String[2]`/`Box[2]`, and unconditional controls; the red cases committed first
- [ ] 6.1.2 Run the suite and record the red phase (the strict C failures); verify the controls
      green first

### Step 6.2: Implementation

- [ ] 6.2.1 Recurse into conditional arms in the literal read/store predicates with the OR rule
      (`own/lower.rs:4839`; only `literal_read_needs_stores` returns false for a `Conditional` at
      its own level) and name `struct_literal_needs_stores` in the codegen fix
      (`codegen/function.rs:2071`); route the ternary-arm literal through the guarded literal store
      (`own/lower.rs:5238-5270`, `:1726`); route the conditional array literal of places through
      the element-wise store form (`codegen/function.rs:1723`, the conditional value lowering)
- [ ] 6.2.2 Run the suite to green; on any failure STOP and fix (or park per AR-33); run
      `make check`

### Step 6.3: Impl Tests & Review

- [ ] 6.3.1 Write the phase implementation pins (the bound destination array, the call-with-
      destination form, the per-arm store, the call counter, the trailing-owner release, and a
      re-emission determinism pin)
- [ ] 6.3.2 Phase review — dispatch all four lenses; apply the corrections; run the ASan/LSan
      probes, the strict builds, the hosted-amiga compile-only leg, and the base/head
      differential. The packet includes the blast-radius step (every literal predicate caller, the
      unguarded codegen read path, the take argument carrier, the conditional fixed-array assign)
      and the test-integrity step
- [ ] 6.3.3 Bounded confirmation: re-run the phase probes and the focused suite; record the
      rounds

### Step 6.4: Phase Record

- [ ] 6.4.1 Phase record: append to `05-findings.md`; update Progress; commit per the active
      commit mode

**Verify**: `make check`; the residue suite plain and `RDC_ASAN=1`; the strict builds; the
hosted-amiga compile leg; the freestanding compile-only leg for the eligible scalar forms with the
per-case `E11200` reasons; the emission differential

**Deliverables**:
- [ ] ST-18..ST-20 red then green; the brace-list and `if`-condition controls stay green
- [ ] The phase-7 and batch-M literal controls stay green

---

## Phase 7: Control resolution (`DEF-427`, `DEF-447`)

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-427` (major, false positive), `DEF-447` (major, false positive)
> **Lenses**: correctness + memory-safety (AR-31; no code change)
> **Reasoning**: medium — the closure is evidence-based; a wrong closure hides a defect, and a
> regression of the controls must reopen the row

### Step 7.1: Controls and Evidence

**Reference**: [03-07](03-07-control-resolution.md) · [07](07-testing-strategy.md)
ST-21..ST-22 · AR-27, AR-28
**Objective**: Pin the view-length matrix and the inline read-base literal as green controls, and
record the false-positive probe evidence.

- [ ] 7.1.1 [spec-author] Write the controls: `viewlengthcontrol` in
      `array_value_integrity_spec_test.rs` (the frozen view-length matrix; the filed byte probe as
      an accepted, sanitizer-clean run, without asserting its exit value) and `inlineownedliteral`
      in `conditional_projection_residue_spec_test.rs` (the filed shape and its variants); both
      controls are green at the head. Add the array-value suite's `run_memory` helper so the ASan
      leg is real
- [ ] 7.1.2 Run the controls and record the green baseline; re-run the adversarial probes
      (`d427p`, the `DEF-447` variants) and confirm the false-positive evidence at the execution
      head

### Step 7.2: Closure Ruling

- [ ] 7.2.1 Record the control-resolution ruling: both rows close as false positives with the
      probe evidence; no compiler, spec, or registry edit; update the roadmap status
      **provisionally** until the close-out accounting (8.1.2) recomputes the batch
- [ ] 7.2.2 Run `make check` and the focused suites to confirm the controls and no regression

### Step 7.3: Impl Pins & Review

- [ ] 7.3.1 Write the control implementation pins: the view matrix's emitted `.len` and the
      **scalar-`len` fallback** (`len(u8) == 0`, with the filed probe's exact `254` run) as a
      current-output record, plus the body-scope literal binding / no leak for `DEF-447`, in the
      two `_impl_test.rs` files; note that a future scalar-`len` fix needs a user ruling
- [ ] 7.3.2 Phase review — dispatch correctness and memory-safety over the closure evidence;
      confirm the probe record and the control expectations; reclassify to a fix if a red shape
      appears (AR-35(d))
- [ ] 7.3.3 Bounded confirmation: re-run the controls and the closure probes after the review;
      record the rounds

### Step 7.4: Phase Record

- [ ] 7.4.1 Phase record: append the closure evidence and the filed-row outcome to
      `05-findings.md`; update Progress; commit per the active commit mode

**Verify**: `make check`; the array-value and residue suites plain and `RDC_ASAN=1`; the strict
builds

**Deliverables**:
- [ ] ST-21..ST-22 green; the false-positive evidence recorded; the filed byte probe accepted and
      sanitizer-clean (exact output in the impl pin)
- [ ] The scalar-`len` fold filed as a residual row (no code); a future scalar-`len` fix needs a
      user ruling
- [ ] No code change; a red execution-head shape reclassifies the row instead of closing it

---

## Phase 8: Close-out

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: none new (per the `AGENTS.md` dispatch matrix)
> **Reasoning**: low — verification and bookkeeping

### Step 8.1: Verification and Sync

- [ ] 8.1.1 Run the full verification: `make check`, `make verify`, the strict-build sweep, the
      focused ASan legs, the per-phase emission differentials, and the freestanding compile-only
      leg for the eligible scalar P4/P6 forms with the per-case `E11200` ineligibility reasons;
      record the evidence
- [ ] 8.1.2 Update the roadmap rows: the 20 red rows move to Done with per-row evidence; the two
      false-positive controls (`DEF-427`, `DEF-447`) move to Done with their control/probe
      evidence; `DEF-425` stays **parked** with its conflict evidence and ruling request; the two
      other parked majors (`DEF-167`, `DEF-171`) stay open. Add/check every row filed during
      execution (including the `DEF-435` runtime direct read, the `at1` source read, and the
      scalar-`len` fold) with its reproduction and evidence; recompute the next-batch count and
      state the accounting explicitly (AR-36)
- [ ] 8.1.3 Record the close-out section in the findings ledger and mark the plan Complete in
      `00-index.md`
- [ ] 8.1.4 Refresh the specialist check (`analyze-agents`) and record its state; update the
      `AGENTS.md` test map only if the batch wording needs it

**Verify**: `make check`; `make verify`

**Deliverables**:
- [ ] The 22 active rows accounted for: 20 Done with fix evidence plus 2 controls Done with
      false-positive evidence
- [ ] `DEF-425` carried as parked with its ruling request; the two other parked majors open
- [ ] Every in-execution filed row checked (incidental vs open); the `at1` and scalar-`len`
      residuals filed; the next-batch count recomputed
- [ ] The plan marked Complete; the specialist check recorded

---

## Dependencies

```
Phase 1 (loan transfer on stored copies) ──→ Phase 2 (loan sources)     [own/place.rs]
Phase 3 (raw-deref move-outs) ──→ Phase 4 (store lowering and order)    [Deref store/materializer]
Phase 6 (literal destinations)                                          [independent]
Phase 7 (control resolution)                                            [independent; no code]
All → Phase 8 (close-out)
```

Arrow legend: P1→P2 (the shared `own/place.rs` loan bookkeeping), P3→P4 (the raw-deref store and
materializer surfaces); P4 and P5 are independent — `DEF-426` uses the `resolved_place_ty`/
`resolved_value_ty` resolver that already exists at head (`codegen/expr.rs:321`, `:397`), not a P4
output; P6 and P7 are independent. The execution order stays
**1 → 2 → 3 → 4 → 5 → 6 → 7 → 8** (severity-first: P1–P3 carry all nine criticals; AR-4).

---

## Verification Commands

| Command | Purpose |
|---------|---------|
| `make check` | Per-phase gate: `./spec/check.sh` plus `cargo test --workspace` in the pinned container |
| `make -C docker exec CMD="cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test"` | The P1–P4/P6/`DEF-426`/`DEF-447` focused suite |
| `make -C docker exec CMD="RDC_ASAN=1 cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test"` | The P1–P4/P6/`DEF-447` memory leg |
| `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test --test borrow_generic_integrity_impl_test"` | The P5 `DEF-410` focused suite |
| `make -C docker exec CMD="RDC_ASAN=1 cargo test --test borrow_generic_integrity_spec_test --test borrow_generic_integrity_impl_test"` | The P5 memory leg |
| `make -C docker exec CMD="cargo test --test array_value_integrity_spec_test --test array_value_integrity_impl_test"` | The P7 `DEF-427` focused suite (after adding its `run_memory` helper) |
| `make -C docker exec CMD="RDC_ASAN=1 cargo test --test array_value_integrity_spec_test --test array_value_integrity_impl_test"` | The P7 `DEF-427` memory leg (real only after the helper lands) |
| `make -C docker exec CMD="target/debug/clc --profile host -o out.c prog.clo && cc -std=c99 -O0 -g -Wall -Wextra -Werror -o out.bin out.c"` | The strict-build probes (the pinned container) |
| `cc -std=c99 -O0 -g -fsanitize=address -no-pie` + `ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30` | The host ASan/LSan probes |
| `make -C docker exec CMD="target/debug/clc --profile hosted-amiga -o out.c prog.clo"` + the pinned `m68k-amigaos-gcc` | The hosted-amiga compile leg for the P4 store forms and the P6 literal forms |
| Per-phase clc+cc over the named probe sets and the corpus | The base/head emission differential (AR-32) |
| `make verify` | Close-out: every hosted-amiga leg plus the runtime-fault programs |

## Reviewer Dispatch Conventions

- Correctness and semantics via the project briefs (`.opencode/agents/correctness-reviewer.md`,
  `.opencode/agents/semantics-reviewer.md`); memory-safety and emission-integrity via the
  dedicated read-only subagents (`memory-safety-auditor`, `emission-integrity-auditor`).
- Every reviewer packet includes the **blast-radius step** (every call site, exhaustive match,
  and parallel path of the changed construct) and the **test-integrity step** (an assertion must
  fail without the fix); the read-only auditors stay read-only.
- Phase dispatch follows the change surface (AR-31) and the `AGENTS.md` matrix: P1/P2 exclude the
  emission auditor (loan bookkeeping only); P3/P4/P5/P6 include all four; P7 is correctness +
  memory, fix-free.
- Spec cases are never re-pinned; only `_impl_test.rs` byte-identity pins may be, with the
  differential evidence (AR-30).

## Success Criteria

**Feature is complete when:**

1. [ ] All phases completed
2. [ ] All verification passing (`make check`; `make verify` at close-out)
3. [ ] No warnings/errors; no dead code
4. [ ] Security hardened — not applicable beyond the compiler's own input validation (unchanged)
5. [ ] Documentation updated (the roadmap rows and the findings ledger)
6. [ ] Code reviewed (every phase's declared lenses)
