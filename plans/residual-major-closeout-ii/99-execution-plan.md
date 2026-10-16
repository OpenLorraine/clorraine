# Execution Plan: Residual Major Closeout II (Batch L)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-17 23:10
> **Progress**: 2/51 tasks (4%) — Executing Phase 1
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

- [ ] 1.2.1 Guard the per-arm view widening at the conditional, the argument, and the initializer
      store — `compiler/src/codegen/expr.rs`, `compiler/src/codegen/calls.rs`,
      `compiler/src/codegen/function.rs`, `compiler/src/codegen/program.rs`
- [ ] 1.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 1.3: Impl Tests & Review

- [ ] 1.3.1 Write the phase implementation pins (the null-guarded arm spellings, the argument and
      initializer guards, the byte-identical non-null controls, a re-emission determinism pin) —
      `compiler/tests/module_initializer_integrity_impl_test.rs`
- [ ] 1.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses (correctness + memory-safety + emission-integrity; the reviewer packet
      includes the blast-radius step and the test-integrity step)

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

- [ ] 2.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs`
      (walkdotreceiver, walktemprooted, walkconditional) with their controls
- [ ] 2.1.2 Run the suite and record the red phase (the `id(v[i].y)`, `id(s.y)`, `id(o.s.y)`, and
      direct-dot controls verified green first — AR-3/AR-5)

### Step 2.2: Implementation

- [ ] 2.2.1 Unify the walk's value resolver with the emitter's temp-following resolution: the
      local-headed qualified receiver, the temp root, the conditional arms, and the non-local
      place bases — `compiler/src/own/generics.rs`, `compiler/src/codegen/expr.rs`
- [ ] 2.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 2.3: Impl Tests & Review

- [ ] 2.3.1 Write the phase implementation pins (the keyed instance symbols present and the
      opaque/undeclared keys absent, the conditional both-arms agreement, the receiver
      evaluation count, a re-emission determinism pin) —
      `compiler/tests/checker_conformance_impl_test.rs`
- [ ] 2.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses (correctness + semantics + memory-safety + emission-integrity)

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

- [ ] 3.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs`
      (elementstoredrop, elementconversion, duplicableoutslot) and the capture pair
      (`conditional_projection_residue_spec_test.rs` or the element suite) with their controls
- [ ] 3.1.2 Run the suite and record the red phase (the concrete twins and the `DEF-341` pair
      verified green first — AR-3/AR-5/AR-8)

### Step 3.2: Implementation

- [ ] 3.2.1 Substitute the element store's drop decision; record and judge the substituted
      conversion/member duties; restore the duplicable out-slot write-through —
      `compiler/src/own/instance_check.rs`, `compiler/src/codegen/drop.rs`,
      `compiler/src/own/lower.rs`, `compiler/src/types/`, `compiler/src/sema/call.rs`,
      `compiler/src/own/call_check.rs`
- [ ] 3.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 3.3: Impl Tests & Review

- [ ] 3.3.1 Write the phase implementation pins (the substituted drop emission, the duty records
      and report spans, the out-slot store shape, the rejection parity, a re-emission determinism
      pin) — `compiler/tests/checker_conformance_impl_test.rs`
- [ ] 3.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; the memory lens probes the element store forms on both paths

**Verify**: `make check`; the element cases under `RDC_ASAN=1`; the strict builds

**Deliverables**:
- [ ] ST-7..ST-9 red then green; ST-10 stays the green control
- [ ] The element-loop origin pins and the `checker_conformance` oracle stay green

---

## Phase 4: Ownership transfer judgments

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + memory-safety + semantics (AR-10)
> **Reasoning**: high — loans and moves with ASan evidence; the loan matrix is the proof

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-ownership-transfer-judgments.md) · [07](07-testing-strategy.md)
ST-11..ST-12 · AR-3, AR-5
**Objective**: Pin the view-like copy's loan transfer and the owned raw deref read.

- [ ] 4.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs`
      (viewcopyloans, rawreadowner) with their controls
- [ ] 4.1.2 Run the suite and record the red phase (the recorded controls verified green first —
      AR-3/AR-5)

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
