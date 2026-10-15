# Execution Plan: Conditional, Projection, and Update Residue II

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-15 (planning)
> **Progress**: 5/33 tasks (15%) — Phase 1 in review
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

> **Phase baseline tree**: `a462084` (the planning baseline; re-recorded at phase start)
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
- [ ] 1.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses (correctness + memory-safety; the reviewer packet includes the
      blast-radius step and the test-integrity step)

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

- [ ] 2.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (guardcallroot, carrierliteral, discardresidual, barediscard)
- [ ] 2.1.2 Run the suite and record the red phase

### Step 2.2: Implementation

- [ ] 2.2.1 Extend the deferred branch to the call-wrapped element position; bind the carrier
      literal's taken-arm fresh root for the local's frame; judge the discarded arm by the
      root's residual ownership; materialize the bare discarded literal's fresh leaves —
      `compiler/src/own/lower.rs`, `compiler/src/codegen/guards.rs`,
      `compiler/src/codegen/function.rs`
- [ ] 2.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 2.3: Impl Tests & Review

- [ ] 2.3.1 Write the phase implementation pins (the deferred fill shapes, the arm bindings, the
      release counts, the re-emission determinism pins) —
      `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 2.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses

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

- [ ] 3.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (condfield, condrootfield, guardviewfield, condcallliteral, condrowreturn, guardfallback, arrayelement, nestedread)
- [ ] 3.1.2 Run the suite and record the red phase (the `DEF-306` store/iterable controls
      verified green first — AR-3)

### Step 3.2: Implementation

- [ ] 3.2.1 Route the conditional field values and conditional-root field projections through
      the per-arm destination store; use the array-elements initializer for the guarded
      view-array field; fill the conditional-of-literals per arm; bind each arm in the
      fixed-array return emitter (the critical); route the guarded expression fallback through
      the per-arm store; extend `fixed_array_field_needs_stores` to `ArrayLit` elements;
      distribute the nested-arm read base — `compiler/src/codegen/function.rs`,
      `compiler/src/codegen/guards.rs`, `compiler/src/codegen/cleanup.rs`,
      `compiler/src/own/lower.rs`, `compiler/src/own/statements.rs`
- [ ] 3.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 3.3: Impl Tests & Review

- [ ] 3.3.1 Write the phase implementation pins (the per-arm destination shapes, the
      hidden-destination pass, the copy shapes, the arm rebuild, the re-emission determinism
      pins) — `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 3.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses plus the base/head emission differential

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

- [ ] 4.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (loopreceiver, loopmember, guardupdate, arrayfill, rawderefstore)
- [ ] 4.1.2 Run the suite and record the red phase (the plain loop-condition and fill-free
      controls verified green first; the `DEF-294` dual-toolchain measurement recorded — AR-10)

### Step 4.2: Implementation

- [ ] 4.2.1 Settle/pin the mixed-arm loop receiver; bind the loop-condition projection's root
      once; pin the guarded operand's updating parts inside the guard path; pin the array fill
      before the element loop; lower the bare raw-deref store target with the update pin —
      `compiler/src/own/lower.rs`, `compiler/src/own/call.rs`, `compiler/src/own/expr.rs`,
      `compiler/src/own/loops.rs`, `compiler/src/codegen/guards.rs`,
      `compiler/src/codegen/control.rs`, `compiler/src/own/statements.rs`,
      `compiler/src/ir.rs`
- [ ] 4.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 4.3: Impl Tests & Review

- [ ] 4.3.1 Write the phase implementation pins (the receiver pin shape, the loop root binding,
      the guarded fill positions, the fill pin, the deref store statement, the re-emission
      determinism pins) — `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 4.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; record the `DEF-294` closure (fixed or control) in the ledger

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

- [ ] 5.1.1 [spec-author] Write the phase cases — `compiler/tests/conditional_projection_residue_spec_test.rs` (onereportnested, deferphrase, iterableark)
- [ ] 5.1.2 Run the suite and record the red phase (the controls' counts re-measured first)

### Step 5.2: Implementation

- [ ] 5.2.1 Dedupe the reports at one span and withhold the consequence report per AR-7; route
      the deferred owner display through the generated-owner phrase; align
      `mark_read_conditional_move` with the in-place conditional iterable —
      `compiler/src/own/liveness.rs`, `compiler/src/own/statements.rs`,
      `compiler/src/own/place.rs`, `compiler/src/own/expr.rs`
- [ ] 5.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 5.3: Impl Tests & Review

- [ ] 5.3.1 Write the phase implementation pins (the counts and spans, the displayed phrase,
      the mark shape, the re-emission determinism pins) —
      `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 5.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; run the pinned oracles (`callable_release_integrity`,
      `checker_conformance`)

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
