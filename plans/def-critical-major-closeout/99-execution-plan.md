# Execution Plan: Critical/Major DEF Closeout

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-27
> **Progress**: 61/76 tasks (80%)
> **CodeOps Artifact Schema**: 1

## Overview

Close the 34 active critical/major DEFs in five risk-ordered batches. The plan
tracks 35 DEF rows including the stale `DEF-470` close. Every defect gets a
specification regression, a red-phase confirmation (ST-12 is the green stale
pin), a direct fix, implementation/ASAN/strict-build pins, and a roadmap close
through the roadmap skill with evidence.

**🚨 Update this document after EACH completed task.**

---

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 0 | Revalidation and baseline | 6 |
| 1 | Critical flow — DEF-543 | 8 |
| 2 | Raw store integrity — DEF-454…466 | 17 |
| 3 | Conditional materialization — DEF-470…481 | 16 |
| 4 | Semantic parity — DEF-411…522 | 15 |
| 5 | Strict build and type identity — DEF-167/171/416 | 9 |
| 6 | Full verification, review, close-out | 5 |

**Total: 76 tasks across 7 phases.**

> **Execution rule:** the checkbox lists below are the single source of truth.
> Mark `[~]` on implementation, promote to `[x]` only on verify pass, and update
> the header after every task. Mark `[!]` with `Blocked: <reason>` on a blocker.
> Resume at the first `[~]` task, otherwise the first `[ ]` task.

---

## Phase 0: Revalidation and Baseline

> **Phase baseline tree**: `bdf231350a5e3371a40a524c0f9dc5ddc0d6ee55`
> **Lenses**: correctness, memory safety, roadmap hygiene
> **Reasoning**: medium — no behavior changes; establish which rows are real

- [x] 0.1.1 Build the workspace (`cargo build --workspace` in the container) and
  confirm `target/debug/clc` matches the phase baseline before probing. Then
  re-run the evidence probe for all 35 DEF rows (34 active plus the stale
  `DEF-470`) and record the current diagnostic/ASAN/C result per row in
  `plans/def-critical-major-closeout/05-findings.md`; re-record the `DEF-470`
  `E10020` proof (the row is already Done in the working tree). Revalidate
  `DEF-506`/`DEF-507` explicitly. The binary build comes before the probes; the
  full clean baseline is 0.2.1.
  ✅ (completed: 2026-10-25 23:47)
- [x] 0.1.2 Verify `plans/00-roadmap.md` points the 34 active rows at this plan,
  preserves their severity/evidence notes, and has `DEF-470` Done; record the
  links in `plans/def-critical-major-closeout/05-findings.md`.
  ✅ (completed: 2026-10-25 23:47)
- [x] 0.2.1 Capture the clean baseline: `make check`, `make verify`, and the
  focused ownership/codegen suites; record it in
  `plans/def-critical-major-closeout/05-findings.md`.
  ✅ (completed: 2026-10-25 23:47)
- [x] 0.2.2 Create the four new spec/impl **pairs** (eight files) and use the
  established per-file compile/run/ASAN/strict-C helper pattern; do not add a
  shared helper module. Reserve `compiler/src/own/tests.rs` for the internal
  source-set pin, but author that pin in 1.3.2 after the field exists; no
  behavior tests yet. Add the four pairs (and the pre-existing
  `store_emission_integrity_*` omission) to the `AGENTS.md` test-naming map.
  ✅ (completed: 2026-10-25 23:47)
- [x] 0.2.3 Confirm the baseline has no open critical/major row other than the
  confirmed 34, and record the count in
  `plans/def-critical-major-closeout/05-findings.md`.
  ✅ (completed: 2026-10-25 23:47)
- [x] 0.2.4 Inventory every existing test that touches an active DEF and record
  the expected post-fix change: the stage-spec test, the E10213/E10214
  exclusion, `qinitstructname` (it must move from `emit_pair` success to the
  `E11101` rejection), the `generic-mixed`/`generic-mixed-local` counts, the
  accepted out-slot controls, the `ClS`/`ClPoint` C pins, and the conditional
  projection/residue, store-emission, call-value, runtime-value,
  eval-emission, module-initializer, callable-release, and `own`/`sema` unit
  families that pin a changed shape. Each phase close retargets the tests in
  its inventory and records every expected change.
  ✅ (completed: 2026-10-25 23:47)
**Verify**: `make check` green; every active DEF has a current-HEAD reproduction
or a documented stale proof; the existing-test inventory is recorded.

---

## Phase 1: Critical Flow — DEF-543

> **Phase baseline tree**: `577f2aa695853a97f05cd7dccbceafd7241bee96`
> **Lenses**: correctness, memory safety, semantics
> **Reasoning**: high — ownership state across branches and a same-frame escape

- [x] 1.1.1 [spec-author] Write ST-1 from `07-testing-strategy.md` in
  `compiler/tests/borrow_store_integrity_spec_test.rs`, including the
  same-frame, cross-frame, static-only, owned-rebind, definite-overwrite,
  conditional-assignment, take-root, and mixed-root controls.
  ✅ (completed: 2026-10-26 10:52)
- [x] 1.1.2 Confirm red: the same-frame mixed branch is accepted and the
  callee-side `E10034` is missing before the fix; the cross-frame count
  assertion is green before and after and is not the red gate.
  ✅ (completed: 2026-10-26 10:52)
- [x] 1.2.1 Add the bounded `sources` set to `PlaceState` in
  `compiler/src/own/flow.rs`; join it in `FlowState::merge`
  (`compiler/src/own.rs`), and confirm `drop_bindings_from`/`edge_state` prune
  it with the place. Do not add a parallel `Binding` or `FlowState` map
  (PF-004). No `lower.rs` constructor change is needed.
  ✅ (completed: 2026-10-26 10:52)
- [x] 1.2.2 Strong-update the set on every assignment, flatten
  `storage_atoms`, and consult it at the whole-out-slot store in
  `compiler/src/own/statements.rs`, `compiler/src/own/place.rs`, and
  `compiler/src/own/expr.rs` with the destination-root exclusion, the `take`
  deferral, and the type-parameter deferral (`03-A §Implementation Details`).
  Resolve conditional, projection, call, and literal store values through the
  same atoms, and join an element store into the base binding's set (AR-25).
  Record the pre-existing generic `T = str?` false rejection as a residual; do
  not change the instance duty in this batch.
  ✅ (completed: 2026-10-26 10:52)
- [x] 1.2.3 Extend the direct callee scan in `compiler/src/own/call_check.rs`
  with the must-source walk (intersection at branches, conservative at
  loops/defers) so the cross-frame form keeps one report when the store is
  provable, a later assignment cannot suppress an earlier store, and an
  unprovable store keeps the caller guard.
  ✅ (completed: 2026-10-26 10:52)
- [x] 1.3.1 Green: focused `borrow_store_integrity_*` suites and
  `RDC_ASAN=1` for the accepted controls.
  ✅ (completed: 2026-10-26 10:52)
- [x] 1.3.2 Implementation pins in
  `compiler/tests/borrow_store_integrity_impl_test.rs` (cross-frame one-report,
  byte-identical re-emission) and the internal source-set/rollback pin in
  `compiler/src/own/tests.rs` (including a speculative temporary id reuse).
  ✅ (completed: 2026-10-26 10:52)
- [x] 1.3.3 `make check` and `make verify`; retarget the Phase 0 inventory's
  out-slot/conditional self-store tests and record every expected change; close
  `DEF-543` through the roadmap skill with the commit and evidence; add the
  phase record to `05-findings.md`.

  ✅ (completed: 2026-10-26 12:58)
**Verify**: `make check` and `make verify`; focused ASAN clean; `DEF-543` Done.

---

## Phase 2: Raw Store Integrity — DEF-454…466

> **Phase baseline tree**: `9cbe86990b8d601a10a99e098a5a720d7d0382c4`
> **Lenses**: correctness, memory safety, emission integrity, semantics
> **Reasoning**: high — raw aliases, conditional saves, evaluation order, and double-free paths

- [x] 2.1.1 [spec-author] Write ST-2 (`DEF-454`), ST-3 (`455`), ST-4 (`456`),
  ST-5 (`457`), and ST-6 (`460`) in
  `compiler/tests/def_raw_store_integrity_spec_test.rs`.
  ✅ (completed: 2026-10-26 13:13)
- [x] 2.1.2 [spec-author] Write ST-7 (`DEF-461`), ST-8 (`462`), ST-9 (`464`),
  ST-10 (`465`), ST-11 (`466`), and ST-35 (`DEF-458`) in the same file; include
  the address-of spelling, the destination-index/run-time-source control, both
  containment directions, the exact `unsafe` contexts, and the strict-C/ASAN
  controls.
  ✅ (completed: 2026-10-26 13:13)
- [x] 2.1.3 Confirm red: each unsafe reproducer fails (UAF, double-free, lost
  value, leak, or invalid C) before the fix.
  ✅ (completed: 2026-10-26 13:13)
- [x] 2.2.1 DEF-454 guarded struct-literal raw move — `compiler/src/codegen/guards.rs`,
  `compiler/src/codegen/expr.rs`, `compiler/src/codegen/function.rs`.
  ✅ (completed: 2026-10-26 13:22)
- [x] 2.2.2 DEF-455 fixed-array take capture — `compiler/src/codegen/function.rs`.
  ✅ (completed: 2026-10-26 13:24)
- [x] 2.2.3 DEF-456 source/destination containment in both directions with
  integer addresses, the taken subrange mapped onto the save copy, and the
  capture carrying the range — `compiler/src/codegen/function.rs`.
  ✅ (completed: 2026-10-26 13:29)
- [x] 2.2.4 DEF-457 take-capture stack — `compiler/src/codegen.rs`,
  `compiler/src/codegen/function.rs`, `compiler/src/codegen/guards.rs`.
  ✅ (completed: 2026-10-26 13:41)
- [x] 2.2.5 DEF-460 raw self-alias save restore/zero — `compiler/src/own/place.rs`,
  `compiler/src/codegen/function.rs`.
  ✅ (completed: 2026-10-26 13:48)
- [x] 2.2.6 DEF-461 fixed-array parameter alias guard with the adjusted array
  address — `compiler/src/codegen/function.rs` (`is_array_param_place`,
  `array_source_address`).
  ✅ (completed: 2026-10-26 13:56)
- [x] 2.2.7 DEF-462 intermediate raw-field pin — `compiler/src/own/expr.rs`,
  `compiler/src/codegen/function.rs`, `compiler/src/codegen/guards.rs`.
  ✅ (completed: 2026-10-26 14:03)
- [x] 2.2.8 DEF-464 spec-compliant element-move rejection — report one
  `E10026` for a conditional element source (the frozen v1 element-move rule,
  AR-26); keep a same-place stable self-arm a self-copy; the destination read
  control uses a non-element source — `compiler/src/own/expr.rs`.
  ✅ (completed: 2026-10-26 14:11; re-closed by the Phase 2 review fix)
- [x] 2.2.9 DEF-465 RHS-before-clear outside the `needs_drop` gate, including
  the fill twin — `compiler/src/codegen/function.rs`.
  ✅ (completed: 2026-10-26 14:15)
- [x] 2.2.10 DEF-466 runtime element alias test — `compiler/src/codegen/function.rs`.
  ✅ (completed: 2026-10-26 14:19)
- [x] 2.2.11 DEF-458 place-move-out take-ran capture — `compiler/src/codegen/function.rs`.
  ✅ (completed: 2026-10-26 14:23)
- [x] 2.3.1 Green: focused raw-store suites, strict-C builds, and
  `RDC_ASAN=1` for ST-2…ST-11 and ST-35.
  ✅ (completed: 2026-10-26 14:29)
- [x] 2.3.2 Implementation pins in `compiler/tests/def_raw_store_integrity_impl_test.rs`:
  save/capture shapes, both containment ranges, self-alias restore, evaluated
  counts/order, determinism.
  ✅ (completed: 2026-10-26 14:30)
- [x] 2.3.3 `make check` and `make verify`; retarget the Phase 0 inventory's
  affected conditional/store tests and record every expected change; close the
  eleven Batch B roadmap rows through the roadmap skill with ASAN/strict-C
  evidence; add the phase record to `05-findings.md`.

  ✅ (completed: 2026-10-26 14:36)
**Verify**: `make check` and `make verify`; ASAN clean; no invalid-C accepted
program remains in Batch B.

---

## Phase 3: Conditional Materialization — DEF-470…481

> **Phase baseline tree**: `11140b3`
> **Lenses**: correctness, memory safety, emission integrity, semantics
> **Reasoning**: high — hidden destinations, selected-arm evaluation, and
> ownership release

- [x] 3.1.1 [spec-author] Write ST-12 (`DEF-470`), ST-13 (`471`), ST-14
  (`472`), ST-15 (`473`), and ST-16 (`475`) in
  `compiler/tests/def_conditional_value_integrity_spec_test.rs`; ST-12 is the
  green stale pin recording the Phase 0 `E10020` proof, with no red phase.
  ✅ (completed: 2026-10-27)
- [x] 3.1.2 [spec-author] Write ST-17 (`DEF-476`), ST-18 (`477`), ST-19
  (`478`), ST-20 (`479`), ST-21 (`480`), and ST-22 (`481`) in the same file;
  include LSan/strict-C controls and the differing-declaration-order scalar
  nested-literal trace.
  ✅ (completed: 2026-10-27)
- [x] 3.1.3 Confirm red: the read-base, deferred-take, and maybe-target
  reproducers fail before the fixes; `DEF-470` stays green.
  ✅ (completed: 2026-10-27; red: 471, 472, 473, 476, 477, 479; stale at HEAD:
  475, 478, 480, 481 per AR-16; the evidence table is in `05-findings.md`)
- [x] 3.2.1 DEF-471 deref-chain guard — `compiler/src/codegen/expr.rs`.
  ✅ (completed: 2026-10-27; ST-13 green; conditional-projection 230/224 and
  raw-store 29/29 under ASAN unchanged)
- [x] 3.2.2 DEF-472 concrete-lend maybe-target judgment with the recorded
  concrete arm and destination type — thread the destination through
  `compiler/src/own/statements.rs`, `compiler/src/own/place.rs`,
  `compiler/src/ir/mono.rs`, and `compiler/src/own/instance_check.rs`.
  ✅ (completed: 2026-10-27; ST-14 owning instance reports one `E10034`, the
  `T = str` control is ASAN clean; borrow-generic 45/21 unchanged)
- [x] 3.2.3 DEF-473 definite-move `E10020` with the destination-aware duty —
  `compiler/src/own/statements.rs`, `compiler/src/own/place.rs`,
  `compiler/src/ir/mono.rs`, `compiler/src/own/instance_check.rs`.
  ✅ (completed: 2026-10-27; ST-15 reports one `E10020` at the later use, the
  view control exits 0; conditional-projection 230/224 unchanged)
- [x] 3.2.4 DEF-475 owned mixed read base — `compiler/src/codegen/guards.rs`,
  `compiler/src/codegen/program.rs`.
  ✅ (completed: 2026-10-27; stale close per AR-16 — ST-16 exits 3, strict C
  builds, ASAN clean at HEAD; no code change)
- [x] 3.2.5 DEF-476 guarded duplicable literal store — `compiler/src/codegen/guards.rs`.
  ✅ (completed: 2026-10-27; ST-17 strict-C/exit-3 green; conditional-projection
  61/27, residue 230/224, call-value 63/62 unchanged)
- [x] 3.2.6 DEF-477 nested conditional arm recursion — `compiler/src/codegen/function.rs`,
  `compiler/src/codegen/guards.rs`.
  ✅ (completed: 2026-10-27; ST-18 strict-C/exit-0 green; only ST-20 remains red)
- [x] 3.2.7 DEF-478 deferred duplicable take — `compiler/src/codegen/guards.rs`.
  ✅ (completed: 2026-10-27; stale close per AR-16 — ST-19's taken and skipped
  twins both pass at HEAD; no code change)
- [x] 3.2.8 DEF-479 source-order nested fields including the scalar literal,
  with a struct declared in an order that differs from the source order —
  `compiler/src/codegen/guards.rs`, `compiler/src/codegen/expr.rs`.
  ✅ (completed: 2026-10-27; the owning and scalar ST-20 traces are `12`/`13`;
  the conditional suite is 13/13, and the call/eval/store/module pins are
  unchanged)
- [x] 3.2.9 DEF-480 fresh-root release — `compiler/src/codegen/program.rs`,
  `compiler/src/codegen/guards.rs`.
  ✅ (completed: 2026-10-27; stale close per AR-16 — ST-21 exits 3/4, strict C
  builds, LSan clean at HEAD; no code change)
- [x] 3.2.10 DEF-481 call-arm release — `compiler/src/codegen/guards.rs`.
  ✅ (completed: 2026-10-27; stale close per AR-16 — ST-22 exits 0, strict C
  builds, LSan clean at HEAD; no code change)
- [x] 3.3.1 Green: focused conditional suites, strict-C builds, and ASAN/LSan
  for ST-12…ST-22.
  ✅ (completed: 2026-10-27; spec 13/13 strict and 13/13 under ASAN;
  conditional-projection 61/61 and residue 230/230 under ASAN)
- [x] 3.3.2 Implementation pins in `compiler/tests/def_conditional_value_integrity_impl_test.rs`:
  per-arm store shapes, evaluated-arm counts, source order, release counts,
  read/return C, determinism.
  ✅ (completed: 2026-10-27; 11 pins pass: the E11210 guard, the one-report
  lend/local duties, the mixed base, the guarded literal, the nested store,
  the deferred take, source order, the fresh/call releases, and byte-identical
  re-emission)
- [x] 3.3.3 `make check` and `make verify`; retarget the Phase 0 inventory's
  conditional-projection/residue and fresh-root pins and record every expected
  change; close the Batch C rows through the
  roadmap skill (`DEF-470` stays a stale close); add the phase record to
  `05-findings.md`.
  ✅ (completed: 2026-10-27; `make check` and `make verify` all legs pass; no
  emission-pin retarget was needed; the ten active Batch C rows are Done and
  the phase record is in `05-findings.md`)

**Verify**: `make check` and `make verify`; ASAN/LSan clean; strict C builds.

---

## Phase 4: Semantic Parity — DEF-411…522

> **Phase baseline tree**: `283ced8`
> **Lenses**: semantics, correctness, memory safety, emission integrity
> **Reasoning**: high — null rules, substitution parity, and precision without
> weakening safety

- [x] 4.1.1 [spec-author] Write ST-23 (`DEF-411`), ST-24 (`412`), ST-25
  (`413`), ST-26 (`415`), and ST-27 (`505`) in
  `compiler/tests/def_semantic_parity_integrity_spec_test.rs`; split the
  DEF-411 contexts (including the nullable conditional loop head) and the
  DEF-505 accepted/owning cases.
  ✅ (completed: 2026-10-27)
- [x] 4.1.2 [spec-author] Write ST-28 (`DEF-506`), ST-29 (`507`), ST-30
  (`513`), and ST-31 (`522`) in the same file; pair every accepted precision
  fix with its unsafe twin and record the revalidation result for 506/507.
  ✅ (completed: 2026-10-27)
- [x] 4.1.3 Confirm red: false rejects and accepted-invalid null/placeholder
  shapes fail before the fixes; stale rows get a green HEAD pin instead.
  ✅ (completed: 2026-10-27; red: 411, 412, 415, 505, 506, 513, 522; stale
  green: 413, 507; the evidence table is in `05-findings.md`)
- [x] 4.2.1 DEF-411 null-arm contexts, including the nullable conditional loop
  head reporting `E10012` — `compiler/src/sema/ops.rs`,
  `compiler/src/own/loops.rs`, `compiler/src/types/mod.rs` predicates.
  ✅ (completed: 2026-10-27; ST-23 green; lib 379, checker 212, cpr 61, and
  borrow-generic 45 unchanged)
- [x] 4.2.2 DEF-412 view-array return — `compiler/src/own/place.rs`,
  `compiler/src/own/statements.rs`, `compiler/src/own/expr.rs`.
  ✅ (completed: 2026-10-27; an explicit `borrow T` is a view-return candidate
  and the plain `T` keeps the concrete twin's count; ST-24 green, checker 212
  and borrow-store 58 unchanged)
- [x] 4.2.3 DEF-413 nested element substitution — `compiler/src/own/instance_check.rs`.
  ✅ (completed: 2026-10-27; stale close per AR-16 — ST-25 is accepted with no
  placeholder and strict C at the baseline; no code change)
- [x] 4.2.4 DEF-415 same-arm nullable conditional — `compiler/src/sema/ops.rs`,
  `compiler/src/own/instance_check.rs`.
  ✅ (completed: 2026-10-27; ST-26 reports two E10010 at the arms; checker 212,
  borrow-store 58, and conditional-projection 61/230 unchanged)
- [x] 4.2.5 DEF-505 nested projection — `compiler/src/own/instance_check.rs`,
  `compiler/src/own/expr.rs`.
  ✅ (completed: 2026-10-27; the necessary projected-place correction also
  touches `compiler/src/sema/{mod,ops,expr}.rs` and `compiler/src/own/place.rs`:
  a conditional arm resolves only the projection naming its final member, the
  whole-read span replaces the base span, and a take parameter's deep
  projection records the partial-move duty. ST-27 green (accepted duplicable,
  one E10026 owning, one E10034 lend); checker 212, borrow-generic 45,
  conditional-projection 61, and borrow-store 58 unchanged)
- [x] 4.2.6 DEF-506 static-only viewed position — revalidate; if it reproduces,
  `compiler/src/own/call_check.rs` with the conservative proof, plus the
  view-return derivation in `compiler/src/own/place.rs`/`own/statements.rs`
  when the probe reports `E10036`; otherwise close stale with a green HEAD pin
  and a `05-findings.md` proof.
  ✅ (completed: 2026-10-27; the bounded proof lives in
  `compiler/src/own/place.rs` beside the existing return-flow summary: a
  `static str` value, and a callee whose every direct return is static, are
  static storage. The static tag, the local static tag, and the caller-returned
  static view are accepted with strict C/ASAN; the owned-field twin keeps the
  frozen E10036. ST-28 green; eval-emission 68, runtime-value 30,
  borrow-view 16, checker 212, borrow-store 58, and spec check 110 unchanged.
  The plan's E10034 expectation for the name twin conflicted with the frozen
  EL-2/EL-3/BR-7; the test follows the frozen oracle — recorded in
  `05-findings.md`)
- [x] 4.2.7 DEF-507 static-return helper — revalidate; if it reproduces,
  `compiler/src/own/place.rs`/`own/statements.rs`; otherwise close stale with a
  green HEAD pin and a `05-findings.md` proof.
  ✅ (completed: 2026-10-27; stale close per AR-16 — ST-29 is accepted, strict-
  C, and ASAN-clean at the baseline; no code change)
- [x] 4.2.8 DEF-513 inline-iterable alias return — `compiler/src/own/expr.rs`,
  `compiler/src/own/loops.rs`.
  ✅ (completed: 2026-10-27; the return recognizes a borrow element of an
  inline iterable (no alias container, a compiler address temporary) and
  reports one E10030 at the return instead of E10035. ST-30 green;
  element-loop 25, codegen-ownership 16, eval-emission 68, callable-release 69,
  runtime-value 30, checker 212, borrow-store 58, and conditional-projection 61
  unchanged)
- [x] 4.2.9 DEF-522 type-parameter view-member move **and read/return** —
  `compiler/src/own/instance_check.rs`, `compiler/src/codegen/expr.rs`.
  ✅ (completed: 2026-10-27; the instance lend judgment exempts a view's
  intrinsic `.ptr`/`.len` member (a value copy) and resolves a view's element
  to its concrete element type, so an owning `String[]` element keeps the
  concrete E10034. The move, read/return, and local static-tag forms are
  accepted with valid strict C (`own/instance_check.rs` only; the emitter
  already lowers the opaque member at the instance). ST-31 green. The plan's
  local-arm E10030 conflicted with the concrete twin and frozen EL-4/BR-6;
  the test follows the oracle — recorded in `05-findings.md`. Semantic 9,
  borrow-generic 45, borrow-store 58, checker 212, and the eleven focused
  regression suites unchanged)
- [x] 4.3.1 Green: focused parity suites, ASAN for safe controls, and strict-C
  comparison against the concrete twin.
  ✅ (completed: 2026-10-27; the semantic suite is 9/9 and `RDC_ASAN=1` is
  clean for every safe control; the focused regression batch (borrow-generic
  45, borrow-store 58, checker 212, conditional-projection 61, eval-emission
  68, callable-release 69, runtime-value 30, element-loop 25, codegen-ownership
  16, backlog-closeout 50, raw-store 29, conditional-value 18) is green; the
  accepted generic view-member C builds under `-Werror` with the concrete
  twin's `.tag.ptr` shape)
- [x] 4.3.2 Implementation pins in `compiler/tests/def_semantic_parity_integrity_impl_test.rs`:
  one-report counts, no placeholder, concrete/generic parity, read/return C,
  determinism.
  ✅ (completed: 2026-10-27; five pins: the six rejected shapes' exact error
  counts (two E10010 and five single reports), no `#element` placeholder with
  `cl_f__arr2_u8`, the generic `cl_pack__H` and concrete `.tag.ptr` shapes, the
  `cl_readTag__H`/`static u8 *` read/return unit, and byte-identical re-
  emission; all five green)
- [ ] 4.3.3 `make check` and `make verify`; retarget the Phase 0 inventory's
  semantic-parity and view-return pins and record every expected change; close
  the Batch D rows through the roadmap skill and record incidental minor
  closures without expanding scope; add the phase record to `05-findings.md`.

**Verify**: `make check` and `make verify`; no safe twin regressed; no unsafe
twin accepted.

---

## Phase 5: Strict Build and Type Identity — DEF-167/171/416

> **Phase baseline tree**: recorded at phase start
> **Lenses**: correctness, semantics, emission integrity, type identity
> **Reasoning**: medium — three bounded parser/analysis/table fixes
>
> **R-6 resolved (2026-10-24):** follow `spec/04` §16; a module-level
> `let embed` reports `E10213`. The `spec/13` §1 sentence is recorded as a
> frozen inconsistency; no spec chapter changes.

- [ ] 5.1.1 [spec-author] Write ST-32, ST-33, and ST-34 in
  `compiler/tests/def_strict_build_integrity_spec_test.rs`; keep the frozen
  stage parse and the E10213 golden text unchanged; the E10213 golden is
  re-enabled in the harness.
- [ ] 5.1.2 Confirm red: the generic-call C failure, accepted `embed`, and the
  type-table collision reproduce before the fixes.
- [ ] 5.2.1 DEF-167 explicit type arguments on a non-generic callee —
  `compiler/src/sema/call.rs`; reuse one `E10300` for free, qualified, dot,
  func-typed, **and non-callable local/parameter** forms. The filed stage shape
  is the local i32 `a`; the non-generic function form is the `DEF-418` twin.
- [ ] 5.2.2 DEF-171 `embed` outside a constant context —
  `compiler/src/sema/expr.rs` and the compile-fail harness (`E10213` moves from
  `EXCLUDED_CODES` to `ASSERTED_CODES`; `E10214` stays excluded). Add a
  constant-context flag (`sema/items.rs`) so a module `const` keeps only
  `E10224`; a module-level `let embed` reports `E10213` (R-6).
- [ ] 5.2.3 DEF-416 cross-module same-simple-name collision — report one
  `E11101` at the second declaration from a `compiler/src/sema/mod.rs`
  cross-module check with a pure `compiler/src/types/table.rs` collision query;
  keep the simple table and do not change `types/keys.rs`/`codegen/types.rs`.
  Retarget `qinitstructname` from `emit_pair` success to the rejection.
- [ ] 5.2.4 DEF-167 parse-only/stage-select mode — add the `clc` option in
  `compiler/src/driver.rs` and update `compiler/tests/stage_spec_test.rs` to use
  it for the `a < b > (c)` snippet; a full compile of the same local-i32 source
  reports one `E10300`.
- [ ] 5.3.1 Green: focused strict-build suite, stage/compile-fail goldens, and a
  strict-C build of the same-module control; the collision case writes no C.
- [ ] 5.3.2 Implementation pins in `compiler/tests/def_strict_build_integrity_impl_test.rs`:
  diagnostic spans/once, no translation unit, the collision, the parse-only
  mode, determinism.
- [ ] 5.3.3 `make check` and `make verify`; retarget the Phase 0 inventory's
  stage/golden/`qinitstructname` tests and record every expected change; close
  DEF-167/171/416 through the roadmap skill with the frozen spec references;
  add the phase record to `05-findings.md`.

**Verify**: `make check` and `make verify`; the strict-C corpus has no new
failure; the stage-spec parse is green through the parse-only mode.

---

## Phase 6: Full Verification, Review, and Close-out

> **Phase baseline tree**: recorded at phase start
> **Lenses**: all four review lenses
> **Reasoning**: high — program gate; no open critical/major may survive

- [ ] 6.1.1 Run the full `make check` and `make verify`; run every focused
  `RDC_ASAN=1` suite and the determinism pins.
- [ ] 6.1.2 Dispatch the phase review: correctness plus memory-safety;
  emission-integrity for codegen; semantics for the language-rule batches.
- [ ] 6.1.3 Fix critical/major review findings, verify, and perform the single
  re-review on the fix diff; record minor rulings in `05-findings.md`.
- [ ] 6.2.1 Close every remaining active DEF row through the roadmap skill with
  commit/evidence; verify no open critical or major row remains and the notes
  are preserved.
- [ ] 6.2.2 Post-completion re-analysis, workspace cleanup, and final summary.

**Verify**: `make check` and `make verify`; zero open critical/major DEFs; all
review findings closed or explicitly recorded.

---

## Dependencies

```
Phase 0
    ↓
Phase 1
    ↓
Phase 2
    ↓
Phase 3
    ↓
Phase 4
    ↓
Phase 5
    ↓
Phase 6
```

The chain is **risk order**, not a strict technical dependency:

- Batch A's flow-sensitive source union is independent of Batch B.
- Batch B and C share the `codegen/function.rs`/`guards.rs` conditional and
  capture surface; B lands first as risk control.
- Batch C's maybe-target source split is a soft dependency for Batch D's
  instance-audit fixes.
- Batch D's bounded scans do not add a summary subsystem.
- Batch E is source-independent but output-coupled to the C pins; it lands last
  and re-runs the A–D determinism pins.
- `DEF-167`'s parse-only mode is a driver/test integration, not a language
  change; `DEF-416` is a collision report, not a type-identity layer.

---

## Success Criteria

**The program is complete when:**

1. ✅ All 34 active DEF rows are Done; `DEF-470` is closed as stale with HEAD
   evidence, so the plan tracks 35 DEF rows.
2. ✅ No open critical or major DEF remains in `plans/00-roadmap.md`; the rows
   are synced through the roadmap skill and their evidence notes are preserved.
3. ✅ Every ST-1…ST-35 case for an active DEF was red before its fix and green
   after it. ST-12, and any `DEF-506`/`DEF-507` stale close under AR-16 with a
   HEAD proof, are the documented green pins.
4. ✅ Memory rows are ASAN/LSan clean; emission rows build strict C.
5. ✅ `make check` and `make verify` pass.
6. ✅ No spec or golden changed without approval; no new generalized subsystem.
   `DEF-167` uses the parse-only mode; `DEF-416` is the documented E11101
   collision limitation.
7. ✅ Review findings are closed or explicitly recorded.
8. ✅ Documentation, roadmap, and originating ledgers are in sync.
