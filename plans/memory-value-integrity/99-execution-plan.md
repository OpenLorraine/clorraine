# Execution Plan: Memory and Value Integrity (Batch N)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-18
> **Progress**: 0/60 tasks (0%) — Planning
> **Planning baseline**: `f6da78f` (the batch-M close-out commit)
> **CodeOps Artifact Schema**: 1

## Overview

Close the 23 rows named by the dispatch — 9 🔴 criticals and 14 🟠 majors — in seven machinery
phases and a close-out. Twenty-one rows are red at `f6da78f` and are implemented red → green; two
(`DEF-427`, `DEF-447`) are verified not reproducible and are closed as false positives with green
controls and probe evidence (AR-27, AR-28). The scope is severity-first: every critical lands in
P1–P3, and every phase follows the specification-first ordering
(spec cases → red → implement → green → impl pins → review). The new cases append to the origin
suites (AR-30); the probe-verified direct twins (the `E10030`/`E10034`/`E10026` controls, the
`if`-condition literal control, and the `String`/`Box` array controls) stay green as pinned
controls. Every fix extends existing machinery; no new diagnostic, no new IR channel, no new test
suite pair (AR-33, AR-34).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Loan transfer on stored copies | 8 |
| 2 | Loan sources through runtime, temporary, and call reads | 8 |
| 3 | Raw-deref move-outs | 8 |
| 4 | Store lowering and evaluation order | 8 |
| 5 | Per-instance generic judgments | 8 |
| 6 | Literal destinations and conditional arrays | 8 |
| 7 | Control resolution (`DEF-427`, `DEF-447`) | 8 |
| 8 | Close-out | 4 |

**Total: 60 tasks across 8 phases**

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
> **Rows**: `DEF-428` (critical), `DEF-429` (critical), `DEF-431` (critical), `DEF-435` (major)
> **Lenses**: correctness + semantics + memory-safety (AR-31; no emission lens for the loan
> bookkeeping)
> **Reasoning**: high — the fixes change when and where element loans are recorded; a wrong path
> or order either misses the escape or rejects a legal copy, and the frozen `E10031`
> write-conflict reading must stay

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-loan-store-transfers.md) · [07](07-testing-strategy.md) ST-1..ST-4 ·
AR-6, AR-7, AR-9, AR-12
**Objective**: Pin the exact self-assignment, the composite element/projected copy, the deeper
projection, and the element-store nested fill, with their direct controls.

- [ ] 1.1.1 [spec-author] Write the phase cases in
      `compiler/tests/conditional_projection_residue_spec_test.rs` (selfassignloans,
      compositeelementstore, deeperprojectioncopy, elementstorenestedfill) with their direct
      controls; the red cases committed first
- [ ] 1.1.2 Run the suite and record the red phase: the four red cases fail without the fix
      (the expected `E10030` is absent), the direct controls are verified green first, and the
      batch-M loan/E10031 controls stay green (AR-3)

### Step 1.2: Implementation

- [ ] 1.2.1 Complete the element-loan transfer: snapshot before the binding effect or skip the
      exact self-assignment replacement (`own/statements.rs`, `own/place.rs:1096`);
      remove the `projections.is_empty()` gate and pass the destination path to the collector
      (`own/expr.rs:1296-1307`); add the prefix fallback to `move_element_loans`
      (`own/place.rs:1192`); route the element-store value through
      `record_copy_element_loans` (`own/expr.rs:2557-2621`)
- [ ] 1.2.2 Run the suite to green; on any failure STOP and fix the implementation (or park per
      AR-33 if a frozen code is insufficient); run `make check`

### Step 1.3: Impl Tests & Review

- [ ] 1.3.1 Write the phase implementation pins (the loan record paths, the report
      code/count/span per shape, the accepted copy emissions, and a re-emission determinism pin)
      in `conditional_projection_residue_impl_test.rs`
- [ ] 1.3.2 Phase review — dispatch the correctness, semantics, and memory-safety lenses; apply
      the corrections; run the host ASan/LSan probes. The reviewer packet includes the
      blast-radius step (every `set_binding_loans`, `move_element_loans`,
      `record_copy_element_loans`, and `emit_indexed_store` caller) and the test-integrity step
- [ ] 1.3.3 Bounded confirmation: re-run the phase probes and the focused suite after the
      corrections; record FAIL/PASS per round

### Step 1.4: Phase Record

- [ ] 1.4.1 Phase record: append the verdicts, fix rounds, and filed rows to `05-findings.md`;
      update the Progress block and commit per the active commit mode

**Verify**: `make check`; the residue suite plain and `RDC_ASAN=1`; the host ASan/LSan probes; the
strict builds

**Deliverables**:
- [ ] ST-1..ST-4 red then green; the direct controls stay green
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

- [ ] 2.1.1 [spec-author] Write the phase cases (runtimestorecopy, temporaryelementcopy,
      callviewresult) with their direct controls and the `DEF-430` `k = 0` conservative variant;
      the red cases committed first
- [ ] 2.1.2 Run the suite and record the red phase; verify the direct controls green first and
      record the `DEF-430` direct-runtime-fill blast-radius outcome (accepted or rejected) per
      AR-8

### Step 2.2: Implementation

- [ ] 2.2.1 Extend the loan sources: include the runtime binding union in the constant-read
      transfer (`own/place.rs:1369`, `:1192`); record the provenance owners of a non-place
      element/fill value (`own/place.rs:1424-1432`, `:1435`); consult the binding-loan owners in
      the cross-frame fallback (`own/call_check.rs:585`)
- [ ] 2.2.2 Run the suite to green; on any failure STOP and fix (or park per AR-33); run
      `make check`

### Step 2.3: Impl Tests & Review

- [ ] 2.3.1 Write the phase implementation pins (the runtime union record, the temp-owner record,
      the binding-loan fallback, the report code/count/span, and a re-emission determinism pin)
- [ ] 2.3.2 Phase review — dispatch correctness, semantics, and memory-safety; apply the
      corrections; run the host ASan/LSan probes. The packet includes the blast-radius step
      (`collect_place_copy_loans`, `move_element_loans`, `recorded_position_frame_owners`,
      `frame_owned_view_owner`) and the test-integrity step
- [ ] 2.3.3 Bounded confirmation: re-run the phase probes and the focused suite; record the
      rounds

### Step 2.4: Phase Record

- [ ] 2.4.1 Phase record: append to `05-findings.md` (including the `DEF-430` direct-fill
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
      guardedintrinsicderef, guardedmembertake) with both selections and the batch-M
      discard/non-taking controls; the red cases committed first
- [ ] 3.1.2 Run the suite and record the red phase (the five consuming shapes and both guard
      selections); verify the controls green first

### Step 3.2: Implementation

- [ ] 3.2.1 Extend the materializer to a `Deref` anywhere in a projection chain
      (`own/lower.rs:3699-3721`, used by the store/return/field/take paths); consult
      `value_move_outs` in `arm_consumes_target` (`codegen/function.rs:2400-2418`); return a
      raw-deref read unchanged from `bind_intrinsic_argument` (`own/lower.rs:4171-4242`); record
      the pointee move-out for a `Deref`-rooted projection in `take_pointee_move_outs`
      (`own/call.rs:804-821`)
- [ ] 3.2.2 Run the suite to green; on any failure STOP and fix (or park/record per AR-33,
      AR-35(b)); run `make check`

### Step 3.3: Impl Tests & Review

- [ ] 3.3.1 Write the phase implementation pins (the single `cl_alt` call, the read/empty pointer
      identity, the `MoveOutValue` placement inside the evaluated path, the absent false-path
      call, and a re-emission determinism pin)
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
      fixedarrayselfread, derefrhistore) with the conditional-base/block controls, the `assert_traps`
      bounds case, and the duplicable controls; the red cases committed first
- [ ] 4.1.2 Run the suite and record the red phase; verify the controls green first

### Step 4.2: Implementation

- [ ] 4.2.1 Complete the store lowering: make the `Deref`-rooted place a store target and emit
      through the raw-deref path (`own/place.rs:1931`, `own/expr.rs:1003-1073`, `:2557`); pin the
      whole-element base/index unconditionally before the RHS (`own/expr.rs:2490-2564`); evaluate
      the fixed-array literal self-read before the clear and skip the identical constant-index
      `Move` (`codegen/function.rs:1556-1605`, `:659-690`); run `materialize_owning_deref` on the
      element/raw-target RHS (`own/expr.rs:2599-2613`, `:1060-1070`)
- [ ] 4.2.2 Run the suite to green; on any failure STOP and fix (or stop for a profile/new-channel
      ruling per AR-35(a)(b)); run `make check`

### Step 4.3: Impl Tests & Review

- [ ] 4.3.1 Write the phase implementation pins (the target projection, the index pin, the
      store/RHS order, the self-copy skip, the single `cl_alt` call, and a re-emission determinism
      pin)
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
`assert_traps` cases; the hosted-amiga compile leg; the emission differential

**Deliverables**:
- [ ] ST-12..ST-15 red then green; the member-route and conditional-base controls stay green
- [ ] The `E11211` bounds trap and the `E10221` conditional-base rejection are unchanged

---

## Phase 5: Per-instance generic judgments

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-410` (major), `DEF-425` (major), `DEF-426` (major)
> **Lenses**: all four (AR-31; the rejections are memory/ownership; the guard is emission)
> **Reasoning**: high — the per-instance audit must report only for an owning substitution and
> must not report at the definition; the division guard must match the concrete twin exactly

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-per-instance-generic.md) · [07](07-testing-strategy.md)
ST-16..ST-18 · AR-21, AR-22, AR-23
**Objective**: Pin the generic conditional store and borrow element move (`E10034` per owning
instance), the duplicable acceptances, the concrete/take twins, and the generic division guard.

- [ ] 5.1.1 [spec-author] Write the phase cases in
      `borrow_generic_integrity_spec_test.rs` (genericconditionalstore, genericelementborrow) and
      `conditional_projection_residue_spec_test.rs` (genericdivisionguard), with the owning,
      duplicable, concrete, and take controls; the red cases committed first
- [ ] 5.1.2 Run the suites and record the red phase (the generic bodies accepted; the concrete
      twins rejecting; the generic guard absent); verify the controls green first

### Step 5.2: Implementation

- [ ] 5.2.1 Record the conditional type-parameter arm as a maybe-owning source
      (`own/expr.rs:175-205`); extend the element-move duty to a borrow parameter with its
      projection path and judge it per instance (`own/place.rs:629-666`, `:541-590`,
      `own/instance_check.rs:210-260`); feed the resolved per-instance operand type into
      `division_guard_condition` (`codegen/guards.rs:1444-1460`)
- [ ] 5.2.2 Run the suites to green; on any failure STOP and fix (or record per AR-35(f)); run
      `make check`

### Step 5.3: Impl Tests & Review

- [ ] 5.3.1 Write the phase implementation pins (the `E10034` record/count/span, the duplicable
      acceptances, the guard count/text in the generic and concrete emissions, and a re-emission
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

**Verify**: `make check`; the borrow-generic and residue suites plain and `RDC_ASAN=1`; the
`assert_traps` `E11210` cases; the emission differential

**Deliverables**:
- [ ] ST-16..ST-18 red then green; the concrete/take twins keep their reports
- [ ] The generic and concrete division emissions agree

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
ST-19..ST-21 · AR-24, AR-25, AR-26
**Objective**: Pin the conditional-of-literals read base (and nested field/element), the
ternary-arm take literal, and the conditional array literal of places, with their controls.

- [ ] 6.1.1 [spec-author] Write the phase cases (conditionalliteralread, ternarytakeliteral,
      conditionalarraytake) with the brace-list, `if`-condition, `String[2]`/`Box[2]`, and
      unconditional controls; the red cases committed first
- [ ] 6.1.2 Run the suite and record the red phase (the strict C failures); verify the controls
      green first

### Step 6.2: Implementation

- [ ] 6.2.1 Recurse into conditional arms in the literal read/store predicates
      (`own/lower.rs:4839/4849/4864`); route the ternary-arm literal through the guarded literal
      store (`own/lower.rs:5238-5270`, `:1726`); route the conditional array literal of places
      through the element-wise store form (`codegen/function.rs:1723`, the conditional value
      lowering)
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
hosted-amiga compile leg; the emission differential

**Deliverables**:
- [ ] ST-19..ST-21 red then green; the brace-list and `if`-condition controls stay green
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
ST-22..ST-23 · AR-27, AR-28
**Objective**: Pin the view-length matrix and the inline read-base literal as green controls, and
record the false-positive probe evidence.

- [ ] 7.1.1 [spec-author] Write the controls: `viewlengthcontrol` in
      `array_value_integrity_spec_test.rs` (the `len` matrix and the byte-index variant) and
      `inlineownedliteral` in `conditional_projection_residue_spec_test.rs` (the filed shape and
      its variants); both are green at the head
- [ ] 7.1.2 Run the controls and record the green baseline; re-run the adversarial probes
      (`d427p`, the `DEF-447` variants) and confirm the false-positive evidence at the execution
      head

### Step 7.2: Closure Ruling

- [ ] 7.2.1 Record the control-resolution ruling: both rows close as false positives with the
      probe evidence; no compiler, spec, or registry edit; update the roadmap status
- [ ] 7.2.2 Run `make check` and the focused suites to confirm the controls and no regression

### Step 7.3: Impl Pins & Review

- [ ] 7.3.1 Write the control implementation pins (the emitted `.len` for the view matrix and the
      body-scope literal binding / no leak for `DEF-447`) in the two `_impl_test.rs` files
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
- [ ] ST-22..ST-23 green; the false-positive evidence recorded
- [ ] No code change; a red execution-head shape reclassifies the row instead of closing it

---

## Phase 8: Close-out

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: none new (per the `AGENTS.md` dispatch matrix)
> **Reasoning**: low — verification and bookkeeping

### Step 8.1: Verification and Sync

- [ ] 8.1.1 Run the full verification: `make check`, `make verify`, the strict-build sweep, the
      focused ASan legs, and the per-phase emission differentials; record the evidence
- [ ] 8.1.2 Update the roadmap rows: the 21 red rows move to Done with per-row evidence; the two
      false-positive controls (`DEF-427`, `DEF-447`) move to Done with their control/probe
      evidence; the two parked majors (`DEF-167`, `DEF-171`) stay open. Add/check every row filed
      during execution with its reproduction and evidence; recompute the next-batch count and
      state the accounting explicitly (AR-36)
- [ ] 8.1.3 Record the close-out section in the findings ledger and mark the plan Complete in
      `00-index.md`
- [ ] 8.1.4 Refresh the specialist check (`analyze-agents`) and record its state; update the
      `AGENTS.md` test map only if the batch wording needs it

**Verify**: `make check`; `make verify`

**Deliverables**:
- [ ] The 23 rows accounted for: 21 Done with fix evidence plus 2 controls Done with
      false-positive evidence
- [ ] Every in-execution filed row checked (incidental vs open); the next-batch count recomputed
- [ ] The plan marked Complete; the specialist check recorded

---

## Dependencies

```
Phase 1 (loan transfer on stored copies)      [critical-heavy; independent]
Phase 2 (loan sources)                        [independent; shares own/place.rs]
Phase 3 (raw-deref move-outs)                 [independent]
Phase 4 (store lowering and order)            [independent; owns the store shapes]
Phase 5 (per-instance generic judgments)      [independent; reuses the P4 store type]
Phase 6 (literal destinations)                [independent]
Phase 7 (control resolution)                  [independent; no code]
All → Phase 8 (close-out)
```

Arrow legend: none of the phases has a hard code dependency; the execution order is
**severity-first** (P1–P3 carry all nine criticals) and then the majors (P4–P6), with the
fix-free control phase last. P1 and P2 share `own/place.rs` and run in sequence to keep the loan
review coherent; P3 and P4 share the materializer and run in sequence; P5 reuses the store's
resolved type from P4. The phases run 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 in order (AR-4).

---

## Verification Commands

| Command | Purpose |
|---------|---------|
| `make check` | Per-phase gate: `./spec/check.sh` plus `cargo test --workspace` in the pinned container |
| `make -C docker exec CMD="cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test"` | The P1–P4/P6/`DEF-426`/`DEF-447` focused suite |
| `make -C docker exec CMD="RDC_ASAN=1 cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test"` | The P1–P4/P6/`DEF-447` memory leg |
| `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test --test borrow_generic_integrity_impl_test"` | The P5 `DEF-410`/`DEF-425` focused suite |
| `make -C docker exec CMD="RDC_ASAN=1 cargo test --test borrow_generic_integrity_spec_test --test borrow_generic_integrity_impl_test"` | The P5 memory leg |
| `make -C docker exec CMD="cargo test --test array_value_integrity_spec_test --test array_value_integrity_impl_test"` | The P7 `DEF-427` focused suite |
| `make -C docker exec CMD="RDC_ASAN=1 cargo test --test array_value_integrity_spec_test --test array_value_integrity_impl_test"` | The P7 `DEF-427` memory leg |
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
