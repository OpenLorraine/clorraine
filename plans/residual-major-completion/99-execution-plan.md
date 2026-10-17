# Execution Plan: Residual Major Completion (Batch M)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-17
> **Progress**: 26/60 tasks (43%) — Phase 4 red
> **Planning baseline**: `86d6f51` (the batch-L follow-up close-out commit)
> **CodeOps Artifact Schema**: 1

## Overview

Close the last 20 un-parked rows — the critical `DEF-367` and 19 majors, with `DEF-361` and
`DEF-374` verified green and carried as pinned controls — in seven implementation phases by
machinery (call identity and written arguments; substitution edges I; substitution edges II;
dropped store targets; element/store loans; deref/take channels; literal destinations and guards)
and a close-out. The scope is severity-first (the critical is planned first); the **execution is
dependency-ordered** (P1 → P2, P1 → P3, P2 → P3 and P2 → P5, P6 → P7; P4 is independent and runs
fourth — AR-36). Every fix extends existing machinery; every phase follows the specification-first
ordering
(spec cases → red → implement → green → impl pins → review). The new cases append to the origin
suites (AR-26); the probe-verified controls (the six `DEF-361` fixtures, the `DEF-374` two-report
pair, the nullable-handle instance, the constant-index store, the qualified func-typed global, the
read/ordinary store positions) stay green as pinned controls.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Call identity and written-argument checks | 8 |
| 2 | Substitution edges I: conditional and element keys | 8 |
| 3 | Substitution edges II: context typing | 8 |
| 4 | Dropped store targets | 8 |
| 5 | Element and store loans | 8 |
| 6 | Deref and take channels | 8 |
| 7 | Literal destinations and guards | 8 |
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

## Phase 1: Call identity and written-argument checks

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-360`, `DEF-361` (control), `DEF-363`, `DEF-390`
> **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-27; the memory
> lens covers the symbol/IR change)
> **Reasoning**: high — the dot-call route infers the callee's type arguments at the call site,
> the collision spans every declaration kind, and the qualified identity reaches the mode/
> parameter/return lookups

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-call-identity-written-arguments.md) · [07](07-testing-strategy.md)
ST-1..ST-4 · AR-8, AR-9, AR-24, AR-31
**Objective**: Pin the dot-call written argument (including the valid generic caller), the
declaration/import collision kind matrix, the qualified callee identity, and the six direct-name
controls.

- [x] 1.1.1 [spec-author] Write the phase cases —
      `compiler/tests/checker_conformance_spec_test.rs` (dotwrittenarg, directshadowcontrol,
      importdeclcollision) and `compiler/tests/callable_release_integrity_spec_test.rs`
      (qualifiedcallidentity) with their controls; the red cases committed first ✅ (completed: 2026-10-17; four spec cases added: the dot-call written argument (`checker_conformance`), the declaration/import collision with the 10-kind matrix (`checker_conformance`), the six-fixture direct-name control (`checker_conformance`), and the qualified function identity (`callable_release_integrity`))
- [x] 1.1.2 Run the suites and record the red phase (the matching dot calls, the renamed import,
      the qualified mismatch, the func-typed global, and the six direct-name fixtures verified
      green first; the `c: Box` generic caller is part of the red expectation — it is falsely
      rejected at the head; AR-5) ✅ (completed: 2026-10-17; the red phase recorded: checker 164 passed / 2 failed (the dot-written-argument and collision cases), callable 67 passed / 1 failed (the qualified identity); the six shadow fixtures and all other controls green, plain and under `RDC_ASAN=1`)

### Step 1.2: Implementation

- [x] 1.2.1 Infer the callee's type arguments at the dot-call site and run the written-argument
      conversion check there; extend the import duplicate test to every declaration kind; thread
      the module-qualified callee identity through the symbol and the mode/parameter/return
      lookups — `compiler/src/sema/call.rs`, `compiler/src/sema/expr.rs`,
      `compiler/src/load/mod.rs`, `compiler/src/own/call.rs`,
      `compiler/src/own/call_check.rs`, `compiler/src/codegen.rs`,
      `compiler/src/codegen/calls.rs`, `compiler/src/codegen/expr.rs`,
      `compiler/src/codegen/function.rs`. **`DEF-361` is a control: no code change for it**
      (AR-31) ✅ (completed: 2026-10-17; the dot-call written-argument check with call-site type-argument inference (`sema/call.rs:488/619/576/600/1186`); the declaration/import collision for every kind (`load/mod.rs:391/420/177`); the qualified callee identity threaded through the own stage, the emitter, and the initializer path (`own/call_check.rs:297`, `own/call.rs:738/611`, `codegen.rs:705`, `codegen/program.rs:2688/3338`, `own/place.rs:1455/1512/1617`); the direct-name control unchanged; one origin case re-targeted per AR-39)
- [x] 1.2.2 Run the suites to green; on any failure STOP and fix the implementation (or park per
      AR-29 if a frozen code is insufficient) ✅ (completed: 2026-10-17; checker 166/166 and callable 68/68 plain and under `RDC_ASAN=1`; `make check` exit 0 (2,382); the differential 17 intended deltas and no change in the spec/tests, examples, or stdlib emissions; no head-only strict failure; the boundary probes green)

### Step 1.3: Impl Tests & Review

- [x] 1.3.1 Write the phase implementation pins (the dot-call report code/span, the `E11103`
      site and kind matrix, the qualified symbol and lookups, the absent dead instance for the
      six shadow controls, a re-emission determinism pin) in the two `_impl_test.rs` files ✅ (completed: 2026-10-17; seven impl pins: the dot-call report record, the generic-caller acceptance, the collision record, the shadow controls, the qualified symbol, the qualified signature/mismatch, and the func-global control; the base discrimination: 3 + 2 pins fail at `749c453`, the shadow and func-global pins are declared controls; checker impl 80/80, callable impl 36/36, plain and ASan)
- [x] 1.3.2 Phase review — dispatch the lenses (correctness + semantics + memory-safety +
      emission-integrity); apply the corrections; run the base/head emission differential over
      `m360_*`, `m361_*`, `m363_*`, `m390_*` and the corpus (AR-37). The reviewer packet includes
      the blast-radius step (every dot-call route, every declaration kind, every qualified callee
      lookup) and the test-integrity step ✅ (completed: 2026-10-17; the four lenses dispatched: correctness FAIL (2 blockers), semantics PASS WITH MINORS, memory FINDINGS (2), emission CONFIRMED (0); the correction round `dec7695` applied all six items)
- [x] 1.3.3 Bounded confirmation: re-run the phase probes and the focused suites after the
      corrections; record FAIL/PASS per round
### Step 1.4: Phase Record ✅ (completed: 2026-10-17; the bounded confirmations: memory PASS (0 blockers; the ffi findings fixed, the coverage clean), correctness PASS WITH MINORS (0 blockers; L1C-1/L1C-2 closed, the semantics minors closed, the regression matrix clean))

- [x] 1.4.1 Phase record: append the verdicts, fix rounds, and filed rows to `05-findings.md`;
      update the Progress block and commit per the active commit mode ✅ (completed: 2026-10-17; the ledger section recorded; rows `DEF-395`–`DEF-407` filed, `DEF-403` closed incidentally; progress 8/60)

**Verify**: `make check`; the checker-conformance and callable suites; the strict builds; the
emission differential

**Deliverables**:
- [ ] ST-1, ST-3, ST-4 red then green; ST-2 is the green control and the six direct-name fixtures stay green
- [ ] The origin suites and the batch-L identity oracles stay green

---

## Phase 2: Substitution edges I: conditional and element keys

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-364`, `DEF-365`, `DEF-368`
> **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-27)
> **Reasoning**: high — the conditional's per-instance and direct-context judgments reach
> ownership and emission; the keyed-symbol and concrete-twin parity are the proof

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-substitution-conditional-element.md) · [07](07-testing-strategy.md)
ST-5..ST-8 · AR-10, AR-11, AR-12
**Objective**: Pin the conditional's per-instance and four direct-context judgments, the nested
element key, and the substituted element member widening.

- [x] 2.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs`
      (opaqueconditional, nullconditionalarm, nestedelementkey, elementmemberview) with their
      controls; the red cases committed first ✅ (completed: 2026-10-17; four spec cases added: the opaque conditional at its instance with the four direct contexts and the controls, the nested element key with the `id(v.len)` control and the hosted-amiga emission, the substituted element member widening with the concrete twin, and the legal-nullable direct control; a marked freestanding placeholder defers to the impl pins)
- [x] 2.1.2 Run the suite and record the red phase (the identical-arm, non-null-sibling, direct
      context controls, `v.len`, and concrete twin verified green first; AR-5) ✅ (completed: 2026-10-17; the red phase recorded: checker 172 passed / 3 failed (the opaque conditional, the nested key, the member widening); the controls (the concrete twin, `id(v.len)`, the concrete widening, the legal nullable) green at head)

### Step 2.2: Implementation

- [x] 2.2.1 Judge the substituted conditional arms per instance and pass the expected type into
      `check_conditional` and **both arm checks** for the four direct contexts; bind the nested
      element level from the substituted element; apply the substituted member's view conversion —
      `compiler/src/sema/ops.rs`, `compiler/src/sema/expr.rs`, `compiler/src/sema/call.rs`,
      `compiler/src/own/instance_check.rs`, `compiler/src/types/mod.rs`,
      `compiler/src/codegen/expr.rs`, `compiler/src/codegen/function.rs` ✅ (completed: 2026-10-17; the destination into `check_conditional` and both arms (`sema/expr.rs:71`, `sema/ops.rs:328/408/439/465`); the site record and the per-instance audit (`sema/call.rs:343/750`, `sema/expr.rs:476`, `sema/mod.rs:223/254/502`, `own/instance_check.rs:63/1264/1369`); the index-surface level chain (`types/mod.rs:712/749`); the substitution-aware member store (`codegen/function.rs:2479`); the span correction per AR-40)
- [x] 2.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-17; checker 175/175 and impl 84/84 plain and under `RDC_ASAN=1`; `make check` exit 0; the hosted-amiga keyed symbol and widening text; the freestanding `DEF-365` build and the `m364_control_fs` profiles; the differential intended deltas only, no head-only strict failure)

### Step 2.3: Impl Tests & Review

- [x] 2.3.1 Write the phase implementation pins (the conditional report code/count/span per
      instance and per direct context, the keyed `cl_id__u8` and the absent `#`, the member
      widening text and its concrete twin, a re-emission determinism pin) in
      `compiler/tests/checker_conformance_impl_test.rs` ✅ (completed: 2026-10-17; eight impl pins: the opaque conditional at its else arm, the null arm, the four direct contexts, the keyed `cl_id__u8`, the `id(v.len)` control, the member widening with the concrete twin, and the non-view control; the base discrimination 5/3 at `ce1141e`; checker impl 92/92, plain and ASan)
- [x] 2.3.2 Phase review — dispatch the lenses (all four); apply the corrections; the memory lens
      probes the arm values and the member read; run the base/head differential over `m364_*`,
      `m365_*`, `m368_*` (AR-37) ✅ (completed: 2026-10-17; the four lenses dispatched: correctness FAIL (2 blockers), semantics FAIL (3), memory FINDINGS (1 critical), emission FINDINGS (1 major); two correction rounds `b50cc2f`/`731cbc7` applied every item; the dotted root, the FFI crossing, and the mixed arm closed in the follow-up round)
- [x] 2.3.3 Bounded confirmation: re-run the phase probes, the `RDC_ASAN=1` suite leg, and the
      focused suites; record the round results
### Step 2.4: Phase Record ✅ (completed: 2026-10-17; the bounded confirmations: memory PASS WITH MINORS (0 blockers), correctness PASS WITH MINORS (0 blockers), and a fresh final confirmation CLEAN (0 findings); `DEF-408`/`DEF-409` closed incidentally, `DEF-415` filed)

- [x] 2.4.1 Phase record: append the verdicts, fix rounds, and filed rows to `05-findings.md`;
      update the Progress block and commit per the active commit mode ✅ (completed: 2026-10-17; the ledger section recorded; progress 16/60)

**Verify**: `make check`; the checker-conformance suite plain and `RDC_ASAN=1`; the strict builds;
the hosted-amiga emission for `DEF-365`/`368`; the freestanding compile for `DEF-365`; the
no-import `m364_control_fs` on hosted-amiga and freestanding

**Deliverables**:
- [ ] ST-5..ST-8 red then green
- [ ] The origin conversion/element pins and the batch-L `DEF-345`/`351` oracles stay green

---

## Phase 3: Substitution edges II: context typing

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-371`, `DEF-384`, `DEF-385`, `DEF-386`
> **Lenses**: correctness + semantics + emission-integrity (AR-27; the nullable-handle control
> runs ASan in the verification)
> **Reasoning**: high — the literal/context and TY-4 readings are language rules; the qualified
> type path changes the resolved type identity; the folded negated fit is subtle

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-substitution-context-typing.md) · [07](07-testing-strategy.md)
ST-9..ST-12 · AR-13, AR-14, AR-15, AR-16
**Objective**: Pin the plain-call literal, the nested nullable type argument (with the duplicate
count), the qualified type path, and the folded negated literal matrix.

- [x] 3.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs`
      (genericliteralarg, nullabletypearg, qualifiedtypepath, negatedliteral) with their
      controls; the red cases committed first ✅ (completed: 2026-10-17; four spec cases added: the generic concrete-parameter literal with the out-of-range `E10202`, the nullable type argument with the annotation/call-argument/nested cases and the five-site count, the qualified type path with the `ClArray__i32` symbol and the unqualified byte identity, and the eight-position folded negated-literal matrix with the fit controls)
- [x] 3.1.2 Run the suite and record the red phase (the concrete `setS`, the `Array<String?>` and
      nullable-capable instances, the unqualified `Array<i32>`, the positive literals, and the
      fit controls verified green first; AR-5) ✅ (completed: 2026-10-17; the red phase recorded: checker 188 passed / 4 failed; the controls (the concrete `setS`, the nullable-capable `Array<String?>`, the unqualified `Array<i32>`, the positive literals, the `f32`/`bool` rejections) green at head)

### Step 3.2: Implementation

- [x] 3.2.1 Context-type the literal at a concrete generic parameter (plain-call route); apply
      TY-4 at nested type-argument positions **and the explicit call type argument** (keep
      `check_nullable_decl` and `report_illegal_nullable`); resolve the multi-segment named type;
      for a negated integer literal fold with `int_literal_value`, pass the expected type to
      `check_unary`, apply exactly one `int_fits` check gated by `is_integer_scalar`, and do
      **not** context-type the operand — `compiler/src/sema/call.rs`,
      `compiler/src/sema/items.rs`, `compiler/src/sema/mod.rs`,
      `compiler/src/sema/ops.rs`, `compiler/src/own/program.rs`,
      `compiler/src/own/place.rs`, `compiler/src/types/table.rs` ✅ (completed: 2026-10-17; the literal at a concrete generic parameter context-typed (`sema/call.rs:239/630`); TY-4 at the nested type-argument positions and the explicit call type argument with the declaration checks kept (`sema/items.rs:156/180/189`, `sema/mod.rs:575/582/606`, `sema/stmt.rs:91/147`, `sema/call.rs:30/92/176/301`, `sema/expr.rs:1037`); the multi-segment named type resolved (`types/table.rs:663`); the folded negated literal with the integer-scalar fit and no operand context (`sema/ops.rs:27/139`, `sema/expr.rs:64`); the span correction per AR-43)
- [x] 3.2.2 Run the suite to green; on any failure STOP and fix the implementation; if the
      nullable check rejects a legal nullable type, **narrow per AR-34(b)** (never park) ✅ (completed: 2026-10-17; checker 192/192 and impl 105/105 plain and under `RDC_ASAN=1` (the nullable-handle ASan control clean); `make check` exit 0; the hosted-amiga `DEF-384`/`385` legs; the `DEF-386` matrix; the differential intended deltas only, no head-only strict failure)

### Step 3.3: Impl Tests & Review

- [x] 3.3.1 Write the phase implementation pins (the literal's resolved type, the `E10011` span
      and count per nested site with no duplicates, the qualified type symbol, the folded unary
      fit and the position matrix, a re-emission determinism pin) in
      `compiler/tests/checker_conformance_impl_test.rs` ✅ (completed: 2026-10-17; six impl pins: the literal's resolved type and the `E10202`, the five nullable sites once each, the `ClArray__StringQ` control, the qualified `ClArray__i32` byte identity, the folded matrix, and the folded fit with the non-integer reports; the base discrimination 5/1 at `4cfe123`; checker impl 111/111, plain and ASan)
- [x] 3.3.2 Phase review — dispatch the lenses (correctness + semantics + emission-integrity);
      apply the corrections; the AR-34 stop conditions are checked and recorded; run the
      differential over `m371_*`, `m384_*`, `m385_*`, `m386_*` (AR-37) ✅ (completed: 2026-10-17; the three lenses dispatched: correctness PASS WITH MINORS (0 blockers), semantics FAIL (1 blocker: the integer-only literal context), emission CONFIRMED (0); the correction round added the float/cstring literal context at a concrete generic parameter and the value-headed qualified type rejection, plus the register hygiene; the AR-34 stop conditions checked (no legal nullable type rejected); the differential over `m371_*`/`m384_*`/`m385_*`/`m386_*` clean)
- [x] 3.3.3 Bounded confirmation: re-run the phase probes, the nullable-handle ASan control, and
      the focused suites
### Step 3.4: Phase Record ✅ (completed: 2026-10-17; the bounded confirmation: semantics PASS (0 blockers; L3S-001/L3S-002 closed, the regression matrix clean); the correction round verified)

- [x] 3.4.1 Phase record: append the verdicts, fix rounds, and filed rows to `05-findings.md`;
      update the Progress block and commit per the active commit mode ✅ (completed: 2026-10-17; the ledger section recorded; rows `DEF-416`–`DEF-420` filed and `DEF-396`/`401`/`405` extended; progress 24/60)

**Verify**: `make check`; the checker-conformance suite; the strict builds; the nullable-handle
ASan control; the hosted-amiga leg for `DEF-384`/`385`; the `DEF-386` matrix on host

**Deliverables**:
- [ ] ST-9..ST-12 red then green
- [ ] The batch-L literal/conversion oracles stay green

---

## Phase 4: Dropped store targets

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-367` (critical), `DEF-377`
> **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-27; semantics owns
> the place rule and the store target)
> **Reasoning**: high — the critical store changes the emitted C, the RHS side effects, the target
> order, and the drop order; the place rejection is a frozen language rule

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-dropped-store-targets.md) · [07](07-testing-strategy.md)
ST-13..ST-14 · AR-6, AR-7, AR-33
**Objective**: Pin the runtime-indexed element/member store in every form (with the residue-suite
bounds fault) and the conditional-base member store rejection, including the `unsafe` forms.

- [x] 4.1.1 [spec-author] Write the phase cases —
      `compiler/tests/conditional_projection_residue_spec_test.rs` (indexedelementfieldstore,
      conditionalmemberstore) with the inline `assert_traps` bounds case (precedent
      `borrow_store_integrity_spec_test.rs:363`); the red cases committed first. The frozen
      `run_fault_spec_test.rs` data and the Amiga fault list stay untouched (AR-26) ✅ (completed: 2026-10-17; two spec cases added to the residue suite: the indexed-element field store in every form (local, borrow, generic, constant, compound, update, deep chain, nested generic member, multi-index, target order, the generic owning stores with the LSan leg, the compile-only slice, and the inline `assert_traps` bounds case), and the conditional-base store rejection (the five forms, the two unsafe-wrapped conditionals, the raw-prefix acceptance with read-back, the prefix conditional rejection, the read and block controls))
- [x] 4.1.2 Run the suites and record the red phase (the constant-index store, the read spelling,
      the `s.f.g`/`s.arr[i]`/`p.f.g` controls verified green first; AR-5) ✅ (completed: 2026-10-17; the red phase recorded: residue 161 passed / 2 failed; the constant-index, read, `s.f.g`/`s.arr[i]`/`p.f.g` block controls and the prefix-conditional rejection green at head; the order read-back rewritten to discriminate the target index order)

### Step 4.2: Implementation

- [ ] 4.2.1 Resolve the element/member type through the store's `Value` path
      (`resolved_value_ty` on the `AddressOf` operand, or a target `Place`/type duty); generalize
      the target builder for plain/compound/update/deep-chain/multi-index/nested stores; always
      materialize the base/index before the RHS; make the `None` fallback a debug assertion or pin
      its unreachability (no `E10221` false-rejection path for accepted targets); apply the same
      no-silent-drop rule to `emit_indexed_update`; add
      `Member => is_place(base)` and `Unsafe(inner) => is_place(inner)`, and remove the Index bypass —
      `compiler/src/own/expr.rs`, `compiler/src/sema/ops.rs`
- [ ] 4.2.2 Run the suites to green; on any failure STOP and fix the implementation. If a profile
      refuses the route, **stop for a user ruling** (AR-34(a)); do not park

### Step 4.3: Impl Tests & Review

- [ ] 4.3.1 Write the phase implementation pins (the emitted store sequence, the retained index
      and bounds guard, the per-instance owning store and its drop, the single RHS call, the
      compound/update/nested/multi-index forms, the target-order materialization, the `E10221`
      spans for the five conditional forms including the unsafe-wrapped conditional forms, the accepted
      raw-prefix forms with their read-back, a re-emission determinism pin) in
      `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 4.3.2 Phase review — dispatch the lenses (all four); apply the corrections; the memory lens
      probes every store form under ASan/LSan; the emission lens runs the differential over
      `m367_*`/`m377_*` (AR-37)
- [ ] 4.3.3 Bounded confirmation: re-run the phase probes, the ASan matrix, the residue-suite
      `assert_traps` case, and the focused suites
### Step 4.4: Phase Record

- [ ] 4.4.1 Phase record: append the verdicts, fix rounds, and filed rows to `05-findings.md`
      (including the `(*p).f` and `unsafe (*p).f` residual candidates); update the Progress block and commit per the
      active commit mode

**Verify**: `make check`; the residue suite plain and `RDC_ASAN=1`; the ASan/LSan store matrix;
the strict builds; the hosted-amiga compile of the accepted store forms (AR-28)

**Deliverables**:
- [ ] ST-13..ST-14 red then green; the five conditional forms and the unsafe-wrapped **conditional** forms reject with one `E10221`; the unsafe **raw-prefix** forms are accepted with a store-emitting read-back; the bounds trap `E11211` stays green
- [ ] The batch-L store/literal origin pins stay green

---

## Phase 5: Element and store loans

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-372`, `DEF-373`, `DEF-374` (control)
> **Lenses**: correctness + memory-safety + semantics (AR-27)
> **Reasoning**: high — the loan transfers decide heap-use-after-free reports; the union record,
> the ASan matrix, and the report counts are the proof

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-element-store-loans.md) · [07](07-testing-strategy.md)
ST-15..ST-17 · AR-17, AR-18, AR-19, AR-32
**Objective**: Pin the element-read store loan, the copy-source loan union (mixed positions, fill
form, conditional arm), and the borrow-element conditional take control.

- [ ] 5.1.1 [spec-author] Write the phase cases —
      `compiler/tests/conditional_projection_residue_spec_test.rs` (elementreadloan,
      copysourceloans with the mixed-position, different-arm-count, and fill-form cases,
      borrowelemtakecontrol) with their controls; the red cases committed first
- [ ] 5.1.2 Run the suite and record the red phase (the direct/member twins, the in-scope copies,
      the mixed-position union, and the `DEF-374` two-report parity verified green first; AR-5,
      AR-32)

### Step 5.2: Implementation

- [ ] 5.2.1 Carry the element-read store's loan; snapshot both copy arms and the array-literal
      elements and write the destination's loan **union** once, including the fill form, a
      conditional arm that is an array literal, and a conditional element inside an array literal
      — `compiler/src/own/place.rs`, `compiler/src/own/statements.rs`,
      `compiler/src/own/expr.rs`. **`DEF-374` is a control: no code change for it** (AR-32)
- [ ] 5.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 5.3: Impl Tests & Review

- [ ] 5.3.1 Write the phase implementation pins (the loan records and the `E10030` code/count/
      span per shape, the union record and mixed positions, the accepted copy emissions, the
      overwritten-loan drop, the `DEF-374` control parity, a re-emission determinism pin) in
      `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 5.3.2 Phase review — dispatch the lenses (correctness + memory-safety + semantics); the
      memory lens runs the ASan/LSan matrix over the red shapes (before) and the accepted
      controls (after); apply the corrections
- [ ] 5.3.3 Bounded confirmation: re-run the phase probes, the ASan matrix, and the focused
      suites
### Step 5.4: Phase Record

- [ ] 5.4.1 Phase record: append the verdicts, fix rounds, and filed rows to `05-findings.md`;
      update the Progress block and commit per the active commit mode

**Verify**: `make check`; the residue suite plain and `RDC_ASAN=1`; the host ASan probes

**Deliverables**:
- [ ] ST-15..ST-16 red then green; ST-17 stays the green control
- [ ] The batch-L `DEF-315` loan matrix stays green

---

## Phase 6: Deref and take channels

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-375`, `DEF-376`
> **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-27)
> **Reasoning**: high — the shared materializer and the deferred pointer fill decide double frees
> across the store, return, aggregate/field, array-element, conditional-arm, and guarded paths; the emitted pointer identity and move are the proof

### Step 6.1: Specification Tests

**Reference**: [03-06](03-06-deref-take-channels.md) · [07](07-testing-strategy.md)
ST-18..ST-19 · AR-20, AR-21
**Objective**: Pin the single-evaluated owning deref source in the store/return/field/array/
conditional positions and the guarded take's consumed source and deferred pointer fill.

- [ ] 6.1.1 [spec-author] Write the phase cases —
      `compiler/tests/conditional_projection_residue_spec_test.rs` (derefoperandonce,
      guardeddereftake) with the five `DEF-375` shapes, the discarded/non-taking/concrete-
      duplicable controls, and the false/true impure `DEF-376` selections; the red cases
      committed first
- [ ] 6.1.2 Run the suite and record the red phase (the discarded read, the duplicable pointee,
      and the eager take controls verified green first; AR-5)

### Step 6.2: Implementation

- [ ] 6.2.1 Add the shared owning-deref materializer (take-path predicate) used by the store, the
      return, the aggregate/struct-literal field, the array-literal element, and the conditional
      arm; the field source is moved (pointee zeroed); gate the take materialization on
      `defer_move_outs == 0`; add `value_move_outs: Vec<Value>` to `Materialize` (keep
      `move_outs` as the place channel), carry the pointer-temp fill in `Materialize.value`, and
      emit `MoveOutValue` in both guard forms — `compiler/src/own/statements.rs`,
      `compiler/src/own/lower.rs`, `compiler/src/ir.rs`, `compiler/src/own/call.rs`,
      `compiler/src/codegen/guards.rs`, `compiler/src/codegen/function.rs`,
      `compiler/src/codegen/cleanup.rs`; audit the exhaustive matches in
      `compiler/src/own/generics.rs`, `compiler/src/own/instance_check.rs`,
      `compiler/src/own/loops.rs`
- [ ] 6.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 6.3: Impl Tests & Review

- [ ] 6.3.1 Write the phase implementation pins (the single `cl_alt` call in the store/return/field/array/conditional C, the read/empty pointer identity, the guarded emptying position, the deferred
      pointer fill, the false-path absence, a re-emission determinism pin) in
      `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 6.3.2 Phase review — dispatch the lenses (all four); the memory lens runs all five
      `DEF-375` shapes and both `DEF-376` selections under ASan; apply the corrections; run the
      differential over `m375_*`/`m376_*` (AR-37)
- [ ] 6.3.3 Bounded confirmation: re-run the phase probes, the ASan matrix, and the focused
      suites
### Step 6.4: Phase Record

- [ ] 6.4.1 Phase record: append the verdicts, fix rounds, and filed rows to `05-findings.md`;
      update the Progress block and commit per the active commit mode

**Verify**: `make check`; the residue suite plain and `RDC_ASAN=1`; the host ASan probes

**Deliverables**:
- [ ] ST-18..ST-19 red then green (all five `DEF-375` shapes read back 12; both `DEF-376` selections)
- [ ] The batch-L `DEF-329`/guarded-take oracles stay green

---

## Phase 7: Literal destinations and guards

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-378`, `DEF-379`
> **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-27)
> **Reasoning**: high — the unguarded destination and the deferred argument carrier change
> emitted C and evaluation order; the call counters, the drop/re-fill analysis, and the sanitizer
> gate are the proof

### Step 7.1: Specification Tests

**Reference**: [03-07](03-07-literal-destinations-guards.md) · [07](07-testing-strategy.md)
ST-20..ST-21 · AR-22, AR-23
**Objective**: Pin the hidden destination at every literal position (the unguarded builder
included) and the owning nested conditional's deferred carrier/fill.

- [ ] 7.1.1 [spec-author] Write the phase cases —
      `compiler/tests/conditional_projection_residue_spec_test.rs` (hiddenviewdestination,
      guardnestedowning with the loop fixture `b379_loop`) with the brace-list and eager-order
      controls; the red cases committed first
- [ ] 7.1.2 Run the suite and record the red phase (the concrete brace-list, the direct-literal,
      and the unguarded nested controls verified green first; AR-5)

### Step 7.2: Implementation

- [ ] 7.2.1 Route a call-valued view-array element to the binding store at every literal position
      (consult `struct_literal_needs_stores` at the **unguarded codegen value/read** call site,
      including the projection-base position) and defer the by-value argument carrier
      under `defer_move_outs` into the `Materialize` channel; keep the fill in the guard's
      evaluated path — `compiler/src/own/lower.rs`, `compiler/src/codegen/function.rs`,
      `compiler/src/codegen/guards.rs`, `compiler/src/ir.rs` (the `Materialize` value)
- [ ] 7.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 7.3: Impl Tests & Review

- [ ] 7.3.1 Write the phase implementation pins (the bound destination array, the
      call-with-destination form, the field copy, the call counters, the `__cl_owned0` fill
      position inside the guarded path, the loop's re-fill/re-drop/re-release order, a re-emission
      determinism pin) in `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 7.3.2 Phase review — dispatch the lenses (all four); the memory lens analyzes the guard
      drop/re-fill (`codegen/guards.rs:273-307`); the emission lens runs the differential over
      `m378_*`/`m379_*` (AR-37); apply the corrections
- [ ] 7.3.3 Bounded confirmation: re-run the phase probes, the ASan matrix, and the focused
      suites
### Step 7.4: Phase Record

- [ ] 7.4.1 Phase record: append the verdicts, fix rounds, and filed rows to `05-findings.md`;
      update the Progress block and commit per the active commit mode

**Verify**: `make check`; the residue suite plain and `RDC_ASAN=1`; the strict builds; the
hosted-amiga leg

**Deliverables**:
- [ ] ST-20..ST-21 red then green
- [ ] The batch-L phase-5 laziness/literal pins stay green

---

## Phase 8: Close-out

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: none new (per the `AGENTS.md` dispatch matrix)
> **Reasoning**: low — verification and bookkeeping

### Step 8.1: Verification and Sync

- [ ] 8.1.1 Run the full verification: `make check`, `make verify`, the strict-build sweep, the
      focused ASan legs, and the per-phase emission differentials; record the evidence
- [ ] 8.1.2 Update the roadmap rows: the 18 red rows move to Done with per-row evidence; the two
      controls (`DEF-361`, `DEF-374`) move to Done with their control-pin evidence; the two
      parked majors stay open. Add/check **every row filed during execution** with its
      reproduction and evidence; record `DEF-380`'s boundary from the `DEF-378` gridel family
      (incidental vs open), and keep `DEF-382` open unless the fix substitutes explicit type
      arguments (the `DEF-371` concrete-parameter fix does not cover `identity<u8>(5)`); recompute
      the next-batch count from the roadmap. State the accounting explicitly (18 Done + 2
      controls Done; 2 parked majors open; the remaining minors counted by audit — no
      "minors untouched" claim without the count) (AR-38)
- [ ] 8.1.3 Record the close-out section in the findings ledger and mark the plan Complete in
      `00-index.md`
- [ ] 8.1.4 Refresh the specialist check (`analyze-agents`) and record its state; update the
      `AGENTS.md` test map only if the batch wording needs it

**Verify**: `make check`; `make verify`

**Deliverables**:
- [ ] The 20 rows accounted for: 18 Done with evidence plus 2 controls Done
- [ ] Every in-execution filed row checked (incidental vs open); the next-batch count recomputed
- [ ] The plan marked Complete; the specialist check recorded

---

## Dependencies

```
Phase 1 (call identity + written arguments)
   ├──→ Phase 2 (substitution I: conditional/element)
   │        ├──→ Phase 3 (substitution II: context typing; the conditional arm check feeds it)
   │        └──→ Phase 5 (element/store loans)
   └──→ Phase 3

Phase 4 (dropped store targets)          [independent; runs fourth]

Phase 6 (deref/take channels)
        └──→ Phase 7 (literal destinations/guards)

All → Phase 8 (close-out)
```

Arrow legend: `A → B` marks a machinery dependency (B extends the judgment A touches). The scope
is severity-first (the critical is planned first); the **execution is dependency-ordered**: P1
settles the call-site inference, then P2 runs and feeds P3 (the conditional expected-type arm
check) and P5, P4 is independent and runs fourth, and P6 settles the deferred move-out/
`Materialize` channel before P7 uses it (AR-36).
The phases run 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 in order.

---

## Verification Commands

| Command | Purpose |
|---------|---------|
| `make check` | Per-phase gate: `./spec/check.sh` plus `cargo test --workspace` in the pinned container |
| `make -C docker exec CMD="cargo test --test checker_conformance_spec_test --test checker_conformance_impl_test"` | The P1–P3 focused suites |
| `make -C docker exec CMD="RDC_ASAN=1 cargo test --test checker_conformance_spec_test --test checker_conformance_impl_test"` | The P2/P3 memory leg (P2 full; P3's nullable-handle control) |
| `make -C docker exec CMD="cargo test --test callable_release_integrity_spec_test --test callable_release_integrity_impl_test"` | The P1 `DEF-390` focused suites |
| `make -C docker exec CMD="cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test"` | The P4–P7 focused suites |
| `make -C docker exec CMD="RDC_ASAN=1 cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test"` | The P4–P7 memory leg |
| `make -C docker exec CMD="target/debug/clc --profile host -o out.c prog.clo && cc -std=c99 -O0 -g -Wall -Wextra -Werror -o out.bin out.c"` | The strict-build probes (the pinned container) |
| `cc -std=c99 -O0 -g -fsanitize=address -no-pie` + `ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30` | The host ASan/LSan probes |
| `make -C docker exec CMD="target/debug/clc --profile hosted-amiga -o out.c prog.clo"` + the pinned `m68k-amigaos-gcc` | The hosted-amiga emission leg for `DEF-365`/`368`/`378`/`384`/`385` and the no-import `m364_control_fs` control |
| `make -C docker exec CMD="target/debug/clc --profile freestanding -o out.c prog.clo"` | The freestanding compile for `DEF-365` and the no-import `m364_control_fs` control |
| Per-phase clc+cc over the named probe sets and the corpus | The base/head emission differential (AR-37) |
| `make verify` | Close-out: every hosted-amiga leg plus the runtime-fault programs |

## Reviewer Dispatch Conventions

- Correctness and semantics via the project briefs (`.opencode/agents/correctness-reviewer.md`,
  `.opencode/agents/semantics-reviewer.md`); memory-safety and emission-integrity via the
  dedicated read-only subagents (`memory-safety-auditor`, `emission-integrity-auditor`).
- Every reviewer packet includes the **blast-radius step** (every call site, exhaustive match,
  and parallel path of the changed construct) and the **test-integrity step** (an assertion must
  fail without the fix); the read-only auditors stay read-only.
- Phase dispatch follows the change surface (AR-27) and the `AGENTS.md` matrix: P1 includes the
  memory-safety auditor (symbol/IR); P3 excludes it; P5 excludes the emission auditor; P4
  includes semantics. The `DEF-367` emission route (AR-6) and the `DEF-384` type-argument scope
  (AR-34) are checked at their phases and the outcome recorded in the ledger.
- Spec cases are never re-pinned; only `_impl_test.rs` byte-identity pins may be, with the
  differential evidence (AR-35).

## Success Criteria

**Feature is complete when:**

1. [ ] All phases completed
2. [ ] All verification passing (`make check`; `make verify` at close-out)
3. [ ] No warnings/errors; no dead code
4. [ ] Security hardened — not applicable beyond the compiler's own input validation (unchanged)
5. [ ] Documentation updated (the roadmap rows and the findings ledger)
6. [ ] Code reviewed (every phase's declared lenses)
