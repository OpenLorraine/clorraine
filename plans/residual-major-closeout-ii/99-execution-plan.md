# Execution Plan: Residual Major Closeout II (Batch L)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-18 18:00
> **Progress**: 20/51 tasks (39%) — Executing Phase 4
> **Planning baseline**: `8829be6` (the batch-K close-out commit)
> **CodeOps Artifact Schema**: 1

## Overview

Close the 23 remaining un-parked 🟠 major rows — 21 red and `DEF-341`/`DEF-350` verified
incidentally fixed at the planning head — in eight phases by machinery (nullable-view arms; the
walk's projected-argument resolver; instance substitution/element surfaces; ownership transfer
judgments; store emission; call conversions/`ffi`; member typing; module/global identity and the
const folder) and a close-out. Every fix extends existing machinery; every phase follows the
specification-first ordering (spec cases → red → implement → green → impl pins → review). The
new cases append to the origin suites (AR-9); the probe-verified controls (the `DEF-341`/`DEF-350`
spellings, the non-null view twins, the walk/element/`k18` green controls, the `DEF-357` value
folds) stay green as pinned controls.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Nullable-view arms and initializer view stores | 6 |
| 2 | The generic walk's projected-argument resolver | 6 |
| 3 | Instance substitution and element surfaces | 6 |
| 4 | Ownership transfer judgments | 6 |
| 5 | Store emission: conditional bases and guarded view literals | 6 |
| 6 | Call conversions and `ffi` typing | 6 |
| 7 | Member typing on concrete paths | 6 |
| 8 | Module/global identity and the const folder | 6 |
| 9 | Close-out | 3 |

**Total: 51 tasks across 9 phases**

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

## Phase 1: Nullable-view arms and initializer view stores

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + memory-safety + emission-integrity (AR-10) — the read-only
> `memory-safety-auditor` and `emission-integrity-auditor` join the phase
> **Reasoning**: high — a null-guarded widening at three view positions; mirror the existing
> `store_text` guard contract, never re-architect

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-nullable-view-arms.md) · [07](07-testing-strategy.md) ST-1..ST-3 ·
AR-3, AR-5
**Objective**: Pin the call arm, the argument, and the initializer member projection with the
non-null controls.

- [x] 1.1.1 [spec-author] Write the phase cases — `compiler/tests/module_initializer_integrity_spec_test.rs`
      (nullguardcall, nullguardarg, nullguardmember) with their controls ✅ (completed: 2026-10-17 23:10; four tests (twelve programs) appended to `compiler/tests/module_initializer_integrity_spec_test.rs` (+311 lines, additions only; 48 → 52 tests) — the nullable owned call/place arms at a view destination, the nullable conditional argument, and the nullable member projection in a module initializer, each with its non-null and body controls; the place-arm read-back folds 9 on the null path (the read-back rule) and the call-arm case adds the taken-selection control)
- [x] 1.1.2 Run the suite and record the red phase (the non-null controls verified green first —
      AR-3/AR-5) ✅ (completed: 2026-10-17 23:10; plain 48 passed / 4 failed and ASan 48 passed / 4 failed — exactly the four new tests (five red programs: the SIGSEGV 139 / the ASan SEGV on the null paths); the controls green in both legs; parent-verified)

### Step 1.2: Implementation

- [x] 1.2.1 Guard the per-arm view widening at the conditional, the argument, and the initializer
      store — `compiler/src/codegen/expr.rs`, `compiler/src/codegen/calls.rs`,
      `compiler/src/codegen/function.rs`, `compiler/src/codegen/program.rs` ✅ (completed: 2026-10-18 00:10; the four positions share one guard helper (`codegen/function.rs:2067` `needs_view_guard`, `:2083` `guarded_view_widening`, called by `store_text` at `:2150` byte-identically): `conditional_arm` (`codegen/expr.rs:642`) guards a place-rooted nullable owned arm; the initializer store (`codegen/program.rs:1058-1090`/`:1441-1494`) binds a non-place nullable owned source into an init local retained by the view destination; `argument_text` (`codegen/calls.rs:300`) guards a nullable handle argument, with the owner keeping the conditional's nullness (`own/lower.rs:4996`); the initializer member projection widens under the guard (`codegen/program.rs:1077-1085`); the five reds return 9/9/0/0/9 ASan-clean; two adjacent shapes (a direct nullable call argument and a nested initializer conditional) fixed beyond the suite)
- [x] 1.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-18 00:10; the module-initializer spec 52/52 plain and under `RDC_ASAN=1`; the suite family green; `make check` exit 0 (2247 passed); the differential shows 12 guard-only emission diffs and no head-only strict failure; the hosted-amiga cross leg builds; parent-verified 52/52 plain + ASan)

### Step 1.3: Impl Tests & Review

- [x] 1.3.1 Write the phase implementation pins (the null-guarded arm spellings, the argument and
      initializer guards, the byte-identical non-null controls, a re-emission determinism pin) —
      `compiler/tests/module_initializer_integrity_impl_test.rs` ✅ (completed: 2026-10-18 00:50; six impl pins added (36 tests total) plus one spec case for the adjacent fixed shape (the direct nullable call argument; the spec 53 tests), additions only — the guarded call/place arms, the guarded argument (count 2), the guarded member projection with the move-out order, the disclosed base-stable non-null control, and a re-emission pin; the discriminating pins fail at `4346a84` for the intended reason; 36/36 + 53/53 green; `make check` exit 0)
- [x] 1.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses (correctness + memory-safety + emission-integrity; the reviewer packet
      includes the blast-radius step and the test-integrity step) ✅ (completed: 2026-10-18 02:30; all three lenses dispatched plus three correction rounds and a bounded confirmation (FAIL then PASS after the fix): the fresh arm roots, the shared `selects_null`, the conditional-argument routing, the operand-guard restoration, and the plain/inline split; the closing verification: the module-initializer spec 62/62 plain and ASan, the impl pins 42/42, `make check` exit 0, no acceptance change, 0 head-only strict failures, the six probe re-runs as designed)

**Verify**: `make check`; the view cases under `RDC_ASAN=1`; the strict-build probes

**Deliverables**:
- [ ] ST-1..ST-3 red then green (the non-null twins as controls)
- [ ] The origin suites and the pinned oracles stay green

---

## Phase 2: The generic walk's projected-argument resolver

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-10)
> **Reasoning**: high — one shared resolver between the walk and the emitter; the keyed-symbol
> parity is the proof; the resolved instances make previously invalid-C programs run

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-generic-walk-resolver.md) · [07](07-testing-strategy.md) ST-4..ST-6 ·
AR-3, AR-5
**Objective**: Pin the local-headed qualified receiver, the temp-rooted projection, and the
conditional/non-local place bases with their controls.

- [x] 2.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs`
      (walkdotreceiver, walktemprooted, walkconditional) with their controls ✅ (completed: 2026-10-18 03:30; five tests (eleven programs) appended to `compiler/tests/checker_conformance_spec_test.rs` (+403 lines, additions only) plus five two-file helpers — the aliased-receiver initializer dot call (the `cl_pick__Box` implicit; the body and direct controls), the temp-rooted member argument in a borrow loop (the `cl_keep__String` implicit; the ref and concrete controls), and the conditional argument in a generic instance (the `cl_id__i32` implicit; the nested-loop, call-result, local-place and projected controls); `DEF-338`'s recorded `d03` probe is ill-typed (the dot-call checker accepts a mismatched written argument — filed `DEF-360`) and the nearest conforming shape (the aliased imported receiver) is used)
- [x] 2.1.2 Run the suite and record the red phase (the `id(v[i].y)`, `id(s.y)`, `id(o.s.y)`, and
      direct-dot controls verified green first — AR-3/AR-5) ✅ (completed: 2026-10-18 03:30; plain 133 passed / 3 failed and ASan 133 passed / 3 failed — exactly the three new tests (the implicit `cl_pick__Box`/`cl_keep__String`/`cl_id__i32`); the controls green in both legs; parent-verified)

### Step 2.2: Implementation

- [x] 2.2.1 Unify the walk's value resolver with the emitter's temp-following resolution: the
      local-headed qualified receiver, the temp root, the conditional arms, and the non-local
      place bases — `compiler/src/own/generics.rs`, `compiler/src/codegen/expr.rs` ✅ (completed: 2026-10-18 04:20; the initializer walk's dot receiver resolves its head through the module scope (`own/generics.rs:541` `visible_bindings`, wired at `:41` and threaded through the walkers; the qualified head reads the map at `:594`); the instance walk's projected-place resolver mirrors the emitter (new `place_base_ty` `:415` covering all six bases incl. `Temp`/`DerefTemp`/`EnvField` and `field_ty` `:440`), with `Function`/`ProgramInfo` threaded through the call/arg/value walkers; the conditional arm keys through the same resolver with the existing both-arms rule (`expr.rs` unchanged — the emitter was the reference); the three reds run 0 with the keyed symbols; the eight controls byte-identical)
- [x] 2.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-18 04:20; the checker-conformance spec 136/136 plain and under `RDC_ASAN=1`; the differential shows 7 head-only C fixes (addition-only keyed specializations) and no emit/reject/accept regressions; the cross profiles build the changed shapes; determinism holds; parent-verified 136/136 plain + ASan)

### Step 2.3: Impl Tests & Review

- [x] 2.3.1 Write the phase implementation pins (the keyed instance symbols present and the
      opaque/undeclared keys absent, the conditional both-arms agreement, the receiver
      evaluation count, a re-emission determinism pin) —
      `compiler/tests/checker_conformance_impl_test.rs` ✅ (completed: 2026-10-18 05:00; five pins added (55 tests total, additions only) plus a two-file emitter helper and a keyed-instance helper — the aliased-receiver dot call, the temp-rooted member argument, and the conditional argument each assert the keyed instance declared, defined, and called exactly once; the four controls keep their keyed instances; a re-emission pin; three of the five fail at `6c5b98a` for the intended reason, two are disclosed base-stable guards; 55/55 green, `make check` exit 0)
- [x] 2.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses (correctness + semantics + memory-safety + emission-integrity) ✅ (completed: 2026-10-18 09:10; all four lenses dispatched plus two correction rounds and a bounded re-confirmation (FAIL on the expression-lambda corner, fixed and parent-verified): the own-wins collision precedence, the initializer-only scope map, and the shared empty lambda scope; rows filed `DEF-360`–`DEF-366` (+ the `DEF-347` extension; `DEF-362` folded into phase 4, AR-18); the closing verification: the checker-conformance spec 139/139 plain and ASan, the impl pins 58/58, `make check` exit 0, the seven intended fixes unchanged, no unintended accept/reject)

**Verify**: `make check`; the focused suites; the strict builds

**Deliverables**:
- [ ] ST-4..ST-6 red then green
- [ ] The batch-K element cases and the `checker_conformance` oracle stay green

---

## Phase 3: Instance substitution and element surfaces

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-10)
> **Reasoning**: high — substitution reaches drop decisions and a duty set; the `DEF-341`
> control pins the capture parity

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-element-instance-substitution.md) · [07](07-testing-strategy.md)
ST-7..ST-10 · AR-3, AR-8
**Objective**: Pin the element store's release, the substituted conversion/member judgment, the
duplicable out-slot, and the capture pair control.

- [x] 3.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs`
      (elementstoredrop, elementconversion, duplicableoutslot) and the capture pair
      (`conditional_projection_residue_spec_test.rs` or the element suite) with their controls ✅ (completed: 2026-10-18 10:10; five tests appended across the two suites (+597 and +62 lines, additions only) — the borrowed-element store drop (the loop and early-exit leak programs with the concrete control), the converted element at the seven concrete positions with their seven concrete twins and the matching-position controls, the duplicable out-slot conforming control (ST-9's original oracle contradicted the frozen copy-lend rule — `DEF-353` reclassified as a false positive per AR-19), and the `DEF-341` capture pair in the residue suite (PF-006 placement); the seven conversion specs pin `E10200`; every accepted program folds a read-back)
- [x] 3.1.2 Run the suite and record the red phase (the concrete twins and the `DEF-341` pair
      verified green first — AR-3/AR-5/AR-8) ✅ (completed: 2026-10-18 10:10; the checker-conformance suite plain 142 passed / 1 failed and ASan 141 passed / 2 failed — exactly the intended reds (the seven conversion spellings accepted where the concrete twins reject; the leaked element store); the residue suite 146/146 in both legs (the capture pair green); the controls green; parent-verified)

### Step 3.2: Implementation

- [x] 3.2.1 Substitute the element store's drop decision; record and judge the substituted (DEF-353 reclassified as a false positive per AR-19 — the duplicable copy-write is conformant; the ST-9 slot is the conforming control)
      conversion/member duties; restore the duplicable out-slot write-through —
      `compiler/src/own/instance_check.rs`, `compiler/src/codegen/drop.rs`,
      `compiler/src/own/lower.rs`, `compiler/src/types/`, `compiler/src/sema/call.rs`,
      `compiler/src/own/call_check.rs` ✅ (completed: 2026-10-18 11:00; `DEF-339`: the new `resolved_place_ty` (`codegen/expr.rs:283`) completes the recorded destination type through the place base and projections after substitution, and the three drop-decision sites read it (`codegen/drop.rs:23`, `codegen/function.rs:1015`/`:1030`), so the overwritten owning field drops before the store (`if ((*__cl_tmp1).name) { FreeString((*__cl_tmp1).name); } (*__cl_tmp1).name = NULL;`) and the loop/early-exit leaks are clean; `DEF-345`: the deferred-conversion audit substitutes the source and destination through the element-aware bindings (`own/instance_check.rs:1219-1226`), so the seven concrete positions each report exactly one `E10200` matching their concrete twins; `DEF-353` and `DEF-341`: conforming controls only (AR-19); left: `e01_scalar_member_min` stays a pre-existing red outside the row's scope per the design)
- [x] 3.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-18 11:00; the checker-conformance spec 143/143 plain and under `RDC_ASAN=1`; the residue spec 146/146 both legs; the differential shows 2 addition-only emission diffs (the element store drop) and 9 head-only rejects (the seven conversion spellings + `d05_cond_place_arg` + `p2c_capture`), no head-only strict failure; parent-verified both suites plain + ASan)

### Step 3.3: Impl Tests & Review

- [x] 3.3.1 Write the phase implementation pins (the substituted drop emission, the duty records
      and report spans, the out-slot store shape, the rejection parity, a re-emission determinism
      pin) — `compiler/tests/checker_conformance_impl_test.rs` ✅ (completed: 2026-10-18 11:45; four pins added (62 tests total, additions only) plus a one-report rejection helper — the element store's drop sequence (the concrete twin included), the seven conversion spellings each rejected once with the text, the matching-element control instances, and a re-emission pin; three fail at `b42297a` for the intended reason, one is a disclosed base-stable guard; 62/62 green, `make check` exit 0)
- [x] 3.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; the memory lens probes the element store forms on both paths ✅ (completed: 2026-10-18 17:00; all four lenses dispatched plus three correction rounds and a bounded re-confirmation (FAIL on the local-container guard, fixed and parent-verified): the move routing/source emptying (F1), the element-store save path (L3S-1), and the local-binding guard widening (L3S-4); rows filed `DEF-367`–`DEF-371` (+ the `DEF-362` extension; `DEF-369` folded into phase 4, AR-20); the closing verification: the checker-conformance spec 147/147 plain and ASan, the impl pins 66/66, the residue 146/146, `make check` exit 0, the six-probe re-run green, no unintended accept/reject)

**Verify**: `make check`; the element cases under `RDC_ASAN=1`; the strict builds

**Deliverables**:
- [ ] ST-7..ST-9 red then green; ST-10 stays the green control
- [ ] The element-loop origin pins and the `checker_conformance` oracle stay green

---

## Phase 4: Ownership transfer judgments (rows DEF-315, DEF-329, DEF-362, DEF-369)

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + memory-safety + semantics (AR-10)
> **Reasoning**: high — loans and moves with ASan evidence; the loan matrix is the proof

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-ownership-transfer-judgments.md) · [07](07-testing-strategy.md)
ST-11..ST-12 · AR-3, AR-5
**Objective**: Pin the view-like copy's loan transfer and the owned raw deref read.

- [x] 4.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs`
      (viewcopyloans, rawreadowner) with their controls ✅ (completed: 2026-10-18 18:00; four tests (twenty-eight programs) appended to `compiler/tests/conditional_projection_residue_spec_test.rs` (+723 lines, additions only) — the whole-array copy loan shapes (`DEF-315`: the loop/assign/take spellings accepted with ASan heap-use-after-free; the parameter/return/backing/member/element/read controls), the owned raw deref read (`DEF-329`: the double free; the discard/duplicable/named controls), the taken opaque conditional member (`DEF-362` folded: the recorded b2 false/local spellings red; the concrete/duplicable controls; b16 incidentally fixed by the phase-3 F1 correction — carried as a control), and the borrowed element member move (`DEF-369` folded: the v21/v22 invalid-C red class re-derived, the m09 acceptance parity, the concrete/let controls); ST-12's E10020 alternative is not authorable (no checkable shape) — recorded; an adjacent element-read gap filed as `DEF-372`)
- [x] 4.1.2 Run the suite and record the red phase (the recorded controls verified green first —
      AR-3/AR-5) ✅ (completed: 2026-10-18 18:00; the residue suite plain 146 passed / 4 failed and ASan 146 passed / 4 failed — exactly the four new tests (the UAF shapes, the double frees, and the two rejection expectations); the controls green; parent-verified both legs)

### Step 4.2: Implementation

- [ ] 4.2.1 Cover the view-like container in the copy transfer/judgment; route the owned raw
      deref read through the ordinary read's move/duplicate judgment —
      `compiler/src/own/expr.rs`, `compiler/src/own/statements.rs`, `compiler/src/own/place.rs`
- [ ] 4.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 4.3: Impl Tests & Review

- [ ] 4.3.1 Write the phase implementation pins (the loan records and the conflict reports, the
      deref read's move-out/emptying shape, the byte-identical discarded/duplicable twins, a
      re-emission determinism pin) —
      `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 4.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; run the ASan matrix

**Verify**: `make check`; the ownership cases under `RDC_ASAN=1`

**Deliverables**:
- [ ] ST-11..ST-12 red then green (ASan/LSan-clean)
- [ ] The loan/carrier origin pins stay green

---

## Phase 5: Store emission: conditional bases and guarded view literals

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + memory-safety + emission-integrity (AR-10)
> **Reasoning**: high — the frozen `E10221` rejection and a guard-literal materialization

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-store-emission-conditional-literals.md) · [07](07-testing-strategy.md)
ST-13..ST-14 · AR-6
**Objective**: Pin the per-arm conditional-base store and the guarded view-array literal.

- [ ] 5.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs`
      (condbasestore, guardviewliteral) with their controls
- [ ] 5.1.2 Run the suite and record the red phase (the read spelling, the scalar `E10221`, and
      the direct-literal take controls verified green first)

### Step 5.2: Implementation

- [ ] 5.2.1 Report the frozen `E10221` for the conditional/projected-base element store (the
      assignment-place judgment — the scalar twin's rule and site); extend the guard-literal
      materialization to the view-array elements — `compiler/src/sema/ops.rs`,
      `compiler/src/own/lower.rs`, the store/guard emitters
- [ ] 5.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 5.3: Impl Tests & Review

- [ ] 5.3.1 Write the phase implementation pins (the per-arm store shapes, the guard-literal
      destination fill, the call-count shapes, the byte-identical controls, a re-emission
      determinism pin) —
      `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 5.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses

**Verify**: `make check`; the guard cases under `RDC_ASAN=1`; the strict builds

**Deliverables**:
- [ ] ST-13..ST-14 red then green (ST-13 the frozen `E10221` rejection)
- [ ] The batch-J literal/store origin pins stay green

---

## Phase 6: Call conversions and `ffi` typing

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-10)
> **Reasoning**: high — the conversion duty's routes and the count ruling; the legal instances
> are the bounded surface and run under the sanitizers

### Step 6.1: Specification Tests

**Reference**: [03-06](03-06-call-conversions-ffi.md) · [07](07-testing-strategy.md)
ST-15..ST-17 · AR-8
**Objective**: Pin the sibling-fixed null argument, the inverted conversion routes, and the
`DEF-350` control table.

- [ ] 6.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs`
      (inferrednullarg, instanceconversions, ffiresultcontrol) with the count and legal-instance
      controls
- [ ] 6.1.2 Run the suite and record the red phase (the legal instances and the `DEF-350`
      controls verified green first; the `DEF-344` count ruling measured — AR-3/AR-8)

### Step 6.2: Implementation

- [ ] 6.2.1 Judge the sibling-fixed null argument against the resolved parameter; record the
      destination-side and explicit-type-argument conversion duties and judge them per instance —
      `compiler/src/own/call_check.rs`, `compiler/src/sema/call.rs`,
      `compiler/src/own/instance_check.rs`
- [ ] 6.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 6.3: Impl Tests & Review

- [ ] 6.3.1 Write the phase implementation pins (the duty records and the per-instance reports,
      the legal-instance emissions, the `ffi` prototype typing, the control table, a re-emission
      determinism pin) — `compiler/tests/checker_conformance_impl_test.rs`
- [ ] 6.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; record the count ruling in the ledger

**Verify**: `make check`; the conversion/`ffi` origin cases; the compile-only strict builds

**Deliverables**:
- [ ] ST-15..ST-16 red then green; ST-17 stays the green control (per-spelling table)
- [ ] The legal instances and the `checker_conformance` oracle stay green

---

## Phase 7: Member typing on concrete paths

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + semantics + emission-integrity (AR-10)
> **Reasoning**: medium — the unknown-member rejection and the AR-7 conversion; the controls are
> the pinned typed members

### Step 7.1: Specification Tests

**Reference**: [03-07](03-07-member-typing-concrete-paths.md) · [07](07-testing-strategy.md)
ST-18..ST-20 · AR-7
**Objective**: Pin the unknown-member rejections, the mutable raw destinations, and the raw-enum
member.

- [ ] 7.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs`
      (unknownviewmembers, constptrmutdest, rawenummember) with the typed-member and cast
      controls
- [ ] 7.1.2 Run the suite and record the red phase (the typed `.ptr`/`.len`, the sanctioned cast,
      and the scoped enum controls verified green first)

### Step 7.2: Implementation

- [ ] 7.2.1 Report the unknown member on the concrete fallthrough; emit the sanctioned member
      cast at the mutable raw destinations (or the frozen duty, AR-7); route the raw-enum member
      through the enum rule — `compiler/src/sema/expr.rs`, `compiler/src/own/expr.rs`,
      `compiler/src/codegen/function.rs`
- [ ] 7.2.2 Run the suite to green; on any failure STOP and fix the implementation (or park per
      AR-7 if the stop condition fires)

### Step 7.3: Impl Tests & Review

- [ ] 7.3.1 Write the phase implementation pins (the rejection spans, the emitted casts per
      destination, the enum member shape, the byte-identical controls, a re-emission determinism
      pin) — `compiler/tests/checker_conformance_impl_test.rs`
- [ ] 7.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses

**Verify**: `make check`; the strict builds

**Deliverables**:
- [ ] ST-18..ST-20 red then green (or the registered AR-7 park)
- [ ] The batch-K member pins and the `checker_conformance` oracle stay green

---

## Phase 8: Module/global identity and the const folder

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + semantics + emission-integrity (AR-10)
> **Reasoning**: high — the identity threading and the checker typing; the `k18` matrix gates
> the redirection

### Step 8.1: Specification Tests

**Reference**: [03-08](03-08-module-global-identity-and-folds.md) · [07](07-testing-strategy.md)
ST-21..ST-23 · AR-13
**Objective**: Pin the aliased/qualified identity, the deferred extent fold, and the scope-aware
checker typing.

- [ ] 8.1.1 [spec-author] Write the phase cases — `compiler/tests/callable_release_integrity_spec_test.rs`
      (funcglobalresiduals, sconetyping) and
      `compiler/tests/module_initializer_integrity_spec_test.rs` (constfoldabi) with their
      controls
- [ ] 8.1.2 Run the suites and record the red phase (the self-qualified/plain/`k18` controls and
      the `DEF-357` value folds verified green first)

### Step 8.2: Implementation

- [ ] 8.2.1 Thread the aliased-import and qualified value-head identity; defer the extent-position
      `sizeof` fold to the C type size (AR-13); type the bare func-global call through the
      scope's binding — `compiler/src/codegen/expr.rs`, `compiler/src/own/call.rs`,
      `compiler/src/load/`, `compiler/src/consts/mod.rs`, `compiler/src/sema/call.rs`
- [ ] 8.2.2 Run the suites to green; on any failure STOP and fix the implementation (or park per
      AR-13 if the stop condition fires)

### Step 8.3: Impl Tests & Review

- [ ] 8.3.1 Write the phase implementation pins (the emitted alias/binding symbols, the deferred
      extent spelling, the checker typing record, the byte-identical controls, re-emission
      determinism pins) in the origin `_impl_test.rs` files
- [ ] 8.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; the emission lens runs the base/head differential over the `k18`
      matrix

**Verify**: `make check`; the identity matrix and the profile legs

**Deliverables**:
- [ ] ST-21..ST-23 red then green (or the registered AR-13 park)
- [ ] The `k18` matrix and the origin suites stay green

---

## Phase 9: Close-out

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: none new (per the `AGENTS.md` dispatch matrix)
> **Reasoning**: low — verification and bookkeeping

### Step 9.1: Verification and Sync

- [ ] 9.1.1 Run the full verification: `make check`, `make verify`, the strict-build sweep, and
      the ASan suites; record the evidence
- [ ] 9.1.2 Update the roadmap rows: the 21 rows move to Done with per-row evidence;
      `DEF-341`/`DEF-350` recorded as incidentally fixed with their control pins; the six older
      parked rows stay open (AR-2); the minors stay untouched
- [ ] 9.1.3 Record the close-out section in the findings ledger and mark the plan Complete in
      `00-index.md`

**Verify**: `make check`; `make verify`

**Deliverables**:
- [ ] The 23 rows accounted for: 21 Done with evidence plus 2 incidental fixes with controls
- [ ] The parked rows' rulings collected (AR-2)
- [ ] The plan marked Complete

---

## Dependencies

```
Phase 1 (nullable-view arms)
        ↓
Phase 2 (walk resolver) → Phase 3 (instance substitution/element surfaces)
        ↓
Phase 4 (ownership judgments) → Phase 5 (store emission) → Phase 6 (conversions/ffi)
        ↓
Phase 7 (member typing) → Phase 8 (module/global identity + folds) → Phase 9 (close-out)
```

Arrow legend: `A → B` marks a machinery dependency (B extends the judgment A touches); the
vertical order is the executable sequence — the phases run 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 in
order. P3's audits ride P2's resolution; P6's duties mirror P3's instance pattern; P4/P5 are
independent but reviewed after the instance work so their fixes ride the settled substitution.

---

## Verification Commands

| Command | Purpose |
|---------|---------|
| `make check` | Per-phase gate: `./spec/check.sh` plus `cargo test --workspace` in the pinned container |
| `make -C docker exec CMD="cargo test --test module_initializer_integrity_spec_test --test module_initializer_integrity_impl_test"` | The P1/P8(`DEF-357`) focused suites |
| `make -C docker exec CMD="cargo test --test checker_conformance_spec_test --test checker_conformance_impl_test"` | The P2/P3/P6/P7 focused suites |
| `make -C docker exec CMD="cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test"` | The P4/P5 focused suites (plus the `DEF-341` control) |
| `make -C docker exec CMD="cargo test --test callable_release_integrity_spec_test --test callable_release_integrity_impl_test"` | The P8(`DEF-356`/`DEF-359`) focused suites |
| The suites' `RDC_ASAN=1` switch | The memory runs (the suite helper) |
| `make -C docker exec CMD="target/debug/clc --profile host -o out.c prog.clo"` + `cc -std=c99 -O0 -g -Wall -Wextra -Werror` | The strict-build probes (the pinned container); compile-only with the recorded stubs where an `ffi` extern is unimplemented |
| `make verify` | Close-out: every hosted-amiga leg plus the runtime-fault programs |

## Reviewer Dispatch Conventions

- Correctness and semantics via the project briefs (`.opencode/agents/correctness-reviewer.md`,
  `.opencode/agents/semantics-reviewer.md`); memory-safety and emission-integrity via the
  dedicated read-only subagents (`memory-safety-auditor`, `emission-integrity-auditor`).
- Every reviewer packet includes the **blast-radius step** (every call site, exhaustive match,
  and parallel path of the changed construct) and the **test-integrity step** (an assertion must
  fail without the fix); the read-only auditors stay read-only.
- Phase dispatch follows the change surface (AR-10) and the `AGENTS.md` matrix; the `DEF-347`
  (AR-7) and `DEF-357` (AR-13) stop conditions are checked at their phases and the outcome
  recorded in the ledger (the `DEF-322` route is parked; its fix is the frozen `E10221`, PF-001).

## Success Criteria

**Feature is complete when:**

1. [ ] All phases completed
2. [ ] All verification passing (`make check`; `make verify` at close-out)
3. [ ] No warnings/errors; no dead code
4. [ ] Security hardened — not applicable beyond the compiler's own input validation (unchanged)
5. [ ] Documentation updated (the roadmap rows and the findings ledger)
6. [ ] Code reviewed (every phase's declared lenses)
