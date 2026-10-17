# Execution Plan: Residual Major Completion (Batch M)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-17 00:00
> **Progress**: 0/60 tasks (0%) — Planning
> **Planning baseline**: `86d6f51` (the batch-L follow-up close-out commit)
> **CodeOps Artifact Schema**: 1

## Overview

Close the last 20 un-parked rows — the critical `DEF-367` and 19 majors, with `DEF-361` and
`DEF-374` verified green and carried as pinned controls — in seven phases by machinery (call
identity and written arguments; substitution edges I; substitution edges II; dropped store
targets; element/store loans; deref/take channels; literal destinations and guards) and a
close-out. Every fix extends existing machinery; every phase follows the specification-first
ordering (spec cases → red → implement → green → impl pins → review). The new cases append to the
origin suites (AR-26); the probe-verified controls (the `DEF-361` direct-name matrix, the
`DEF-374` two-report pair, the nullable-handle instance, the constant-index store, the read
positions) stay green as pinned controls.

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
> **Lenses**: correctness + semantics + emission-integrity (AR-27)
> **Reasoning**: high — the written-argument check feeds the later phases' argument re-check;
> the collision and the callee identity touch the whole program's binding set

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-call-identity-written-arguments.md) · [07](07-testing-strategy.md)
ST-1..ST-4 · AR-5, AR-8, AR-9, AR-24, AR-31
**Objective**: Pin the dot-call written argument, the declaration/import collision, the
qualified callee identity, and the direct-name control matrix.

- [ ] 1.1.1 [spec-author] Write the phase cases —
      `compiler/tests/checker_conformance_spec_test.rs` (dotwrittenarg, directshadowcontrol,
      importdeclcollision) and `compiler/tests/callable_release_integrity_spec_test.rs`
      (qualifiedcallidentity) with their controls; the red cases committed first
- [ ] 1.1.2 Run the suites and record the red phase (the controls — the matching dot calls, the
      renamed import, the qualified mismatch, the direct-name matrix — verified green first;
      AR-5)

### Step 1.2: Implementation

- [ ] 1.2.1 Run the written-argument conversion check on the dot-call path (the receiver fixes
      the first parameter), extend the import duplicate test to the module's declarations, and
      thread the module-qualified callee identity — `compiler/src/sema/call.rs`,
      `compiler/src/sema/expr.rs`, `compiler/src/own/instance_check.rs`,
      `compiler/src/load/mod.rs`, `compiler/src/own/call.rs`,
      `compiler/src/own/call_check.rs`, `compiler/src/codegen.rs`
- [ ] 1.2.2 Run the suites to green; on any failure STOP and fix the implementation (or park per
      AR-29 if a frozen code is insufficient)

### Step 1.3: Impl Tests & Review

- [ ] 1.3.1 Write the phase implementation pins (the dot-call report code/span, the `E11103`
      site and count, the emitted qualified symbol, the absent dead instance for the shadow
      controls, a re-emission determinism pin) in the two `_impl_test.rs` files
- [ ] 1.3.2 Phase review — dispatch the lenses (correctness + semantics + emission-integrity);
      apply the corrections; the reviewer packet includes the blast-radius step (every dot-call
      route, every import binding kind, every qualified callee path) and the test-integrity step
- [ ] 1.3.3 Bounded confirmation: re-run the phase probes and the focused suites after the
      corrections; record FAIL/PASS per round
- [ ] 1.4.1 Phase record: append the verdicts, fix rounds, and filed rows to `05-findings.md`;
      update the Progress block and commit per the active commit mode

**Verify**: `make check`; the checker-conformance and callable suites; the strict builds

**Deliverables**:
- [ ] ST-1..ST-3 red then green; ST-4 red then green; the direct-name control stays green
- [ ] The origin suites and the batch-L identity oracles stay green

---

## Phase 2: Substitution edges I: conditional and element keys

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-364`, `DEF-365`, `DEF-368`
> **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-27)
> **Reasoning**: high — the conditional's per-instance judgment and the element-substitution
> surfaces reach ownership and emission; the keyed-symbol and concrete-twin parity are the proof

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-substitution-conditional-element.md) · [07](07-testing-strategy.md)
ST-5..ST-8 · AR-10, AR-11, AR-12
**Objective**: Pin the opaque conditional's two judgments, the nested element key, and the
substituted element member widening.

- [ ] 2.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs`
      (opaqueconditional, nullconditionalarm, nestedelementkey, elementmemberview) with their
      controls; the red cases committed first
- [ ] 2.1.2 Run the suite and record the red phase (the identical-arm, non-null-sibling, `v.len`,
      and concrete twin controls verified green first; AR-5)

### Step 2.2: Implementation

- [ ] 2.2.1 Judge the substituted conditional arms per instance, bind the nested element level
      from the substituted element, and apply the substituted member's view conversion —
      `compiler/src/sema/ops.rs`, `compiler/src/sema/call.rs`,
      `compiler/src/own/instance_check.rs`, `compiler/src/types/mod.rs`,
      `compiler/src/codegen/expr.rs`, `compiler/src/codegen/function.rs`
- [ ] 2.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 2.3: Impl Tests & Review

- [ ] 2.3.1 Write the phase implementation pins (the conditional report code/count/span, the
      keyed `cl_id__u8` and the absent `#`, the member widening text and its concrete twin, a
      re-emission determinism pin) in `compiler/tests/checker_conformance_impl_test.rs`
- [ ] 2.3.2 Phase review — dispatch the lenses (correctness + semantics + memory-safety +
      emission-integrity); apply the corrections; the blast-radius step covers every conditional
      position and every element substitution; the memory lens probes the arm values and the
      member read
- [ ] 2.3.3 Bounded confirmation: re-run the phase probes and the focused suites; record the
      round results
- [ ] 2.4.1 Phase record: append the verdicts, fix rounds, and filed rows to `05-findings.md`;
      update the Progress block and commit per the active commit mode

**Verify**: `make check`; the checker-conformance suite; the strict builds; the hosted-amiga and
freestanding emission for `DEF-365`

**Deliverables**:
- [ ] ST-5..ST-8 red then green
- [ ] The origin conversion/element pins and the batch-L `DEF-345`/`351` oracles stay green

---

## Phase 3: Substitution edges II: context typing

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-371`, `DEF-384`, `DEF-385`, `DEF-386`
> **Lenses**: correctness + semantics + emission-integrity (AR-27)
> **Reasoning**: high — the literal/context and TY-4 readings are language rules; the qualified
> type path changes the resolved type identity

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-substitution-context-typing.md) · [07](07-testing-strategy.md)
ST-9..ST-12 · AR-13, AR-14, AR-15, AR-16
**Objective**: Pin the literal at a concrete generic parameter, the nullable type argument, the
qualified type path, and the negated literal.

- [ ] 3.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs`
      (genericliteralarg, nullabletypearg, qualifiedtypepath, negatedliteral) with their
      controls; the red cases committed first
- [ ] 3.1.2 Run the suite and record the red phase (the concrete `setS`, the `Array<String?>`
      and nullable-capable instances, the unqualified `Array<i32>`, the positive literals, and
      the fit controls verified green first; AR-5)

### Step 3.2: Implementation

- [ ] 3.2.1 Context-type the literal at a concrete generic parameter, apply TY-4 to every
      resolved type node, resolve the multi-segment named type, and propagate the unary
      operand's context — `compiler/src/sema/call.rs`, `compiler/src/sema/items.rs`,
      `compiler/src/sema/mod.rs`, `compiler/src/sema/ops.rs`,
      `compiler/src/own/program.rs`, `compiler/src/types/table.rs`
- [ ] 3.2.2 Run the suite to green; on any failure STOP and fix the implementation (or park per
      AR-34 if a legal nullable type is rejected)

### Step 3.3: Impl Tests & Review

- [ ] 3.3.1 Write the phase implementation pins (the literal's resolved type, the `E10011` span,
      the qualified type symbol, the unary context and the `u8` wrapping, a re-emission
      determinism pin) in `compiler/tests/checker_conformance_impl_test.rs`
- [ ] 3.3.2 Phase review — dispatch the lenses (correctness + semantics + emission-integrity);
      apply the corrections; the AR-34 stop conditions are checked and recorded
- [ ] 3.3.3 Bounded confirmation: re-run the phase probes and the focused suites
- [ ] 3.4.1 Phase record: append the verdicts, fix rounds, and filed rows to `05-findings.md`;
      update the Progress block and commit per the active commit mode

**Verify**: `make check`; the checker-conformance suite; the strict builds; the hosted-amiga leg

**Deliverables**:
- [ ] ST-9..ST-12 red then green
- [ ] The batch-L literal/conversion oracles stay green

---

## Phase 4: Dropped store targets

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-367` (critical), `DEF-377`
> **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-27)
> **Reasoning**: high — the critical store changes the emitted C, the RHS side effects, and the
> drop order; the place rejection is a frozen language rule

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-dropped-store-targets.md) · [07](07-testing-strategy.md)
ST-13..ST-14 · AR-6, AR-7, AR-33
**Objective**: Pin the runtime-indexed element field store (with the bounds fault) and the
conditional-base member store rejection.

- [ ] 4.1.1 [spec-author] Write the phase cases —
      `compiler/tests/conditional_projection_residue_spec_test.rs` (indexedelementfieldstore,
      conditionalmemberstore) and `compiler/tests/run_fault_spec_test.rs` (the `DEF-367`
      bounds case), with their controls; the red cases committed first
- [ ] 4.1.2 Run the suites and record the red phase (the constant-index store, the read spelling,
      the ordinary member/element stores verified green first; AR-5)

### Step 4.2: Implementation

- [ ] 4.2.1 Route the member-over-runtime-index store through the element address and recurse the
      place judgment — `compiler/src/own/expr.rs`, `compiler/src/sema/ops.rs`
- [ ] 4.2.2 Run the suites to green; on any failure STOP and fix the implementation (or park the
      row per AR-34)

### Step 4.3: Impl Tests & Review

- [ ] 4.3.1 Write the phase implementation pins (the emitted store sequence, the retained index
      and bounds guard, the single RHS call, the owning-member drop order, the `E10221` spans, a
      re-emission determinism pin) in
      `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 4.3.2 Phase review — dispatch the lenses (correctness + semantics + memory-safety +
      emission-integrity); apply the corrections; the memory lens probes every store form under
      ASan; the emission lens diffs the store shapes
- [ ] 4.3.3 Bounded confirmation: re-run the phase probes, the ASan matrix, and the focused
      suites
- [ ] 4.4.1 Phase record: append the verdicts, fix rounds, and filed rows to `05-findings.md`;
      update the Progress block and commit per the active commit mode

**Verify**: `make check`; the residue and run-fault suites; the ASan store matrix; the strict builds

**Deliverables**:
- [ ] ST-13..ST-14 red then green; the bounds trap `E11211` stays green
- [ ] The batch-L store/literal origin pins stay green

---

## Phase 5: Element and store loans

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-372`, `DEF-373`, `DEF-374` (control)
> **Lenses**: correctness + memory-safety + semantics (AR-27)
> **Reasoning**: high — the loan transfers decide heap-use-after-free reports; the ASan matrix
> and the report counts are the proof

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-element-store-loans.md) · [07](07-testing-strategy.md)
ST-15..ST-17 · AR-17, AR-18, AR-19, AR-32
**Objective**: Pin the element-read store loan, the copy-source loans, and the borrow-element
conditional take control.

- [ ] 5.1.1 [spec-author] Write the phase cases —
      `compiler/tests/conditional_projection_residue_spec_test.rs` (elementreadloan,
      copysourceloans, borrowelemtakecontrol) with their controls; the red cases committed first
- [ ] 5.1.2 Run the suite and record the red phase (the direct and member twins, the in-scope
      copies, and the `DEF-374` two-report parity verified green first; AR-5, AR-32)

### Step 5.2: Implementation

- [ ] 5.2.1 Carry the element-read store's loan and transfer the conditional/literal/single-view
      copy sources' loans — `compiler/src/own/place.rs`, `compiler/src/own/statements.rs`,
      `compiler/src/own/expr.rs`
- [ ] 5.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 5.3: Impl Tests & Review

- [ ] 5.3.1 Write the phase implementation pins (the loan records and the `E10030` code/count/
      span per shape, the accepted copy emissions, the overwritten-loan drop, the `DEF-374`
      control parity, a re-emission determinism pin) in
      `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 5.3.2 Phase review — dispatch the lenses (correctness + memory-safety + semantics); the
      memory lens runs the ASan/LSan matrix over the red shapes (before) and the accepted
      controls (after); apply the corrections
- [ ] 5.3.3 Bounded confirmation: re-run the phase probes, the ASan matrix, and the focused
      suites
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
> **Reasoning**: high — the deref materialization and the guarded move-out decide double frees;
> the emitted pointer identity is the proof

### Step 6.1: Specification Tests

**Reference**: [03-06](03-06-deref-take-channels.md) · [07](07-testing-strategy.md)
ST-18..ST-19 · AR-20, AR-21
**Objective**: Pin the single-evaluated impure deref operand and the guarded take's consumed
source.

- [ ] 6.1.1 [spec-author] Write the phase cases —
      `compiler/tests/conditional_projection_residue_spec_test.rs` (derefoperandonce,
      guardeddereftake) with the discarded/duplicable and both-selection controls; the red cases
      committed first
- [ ] 6.1.2 Run the suite and record the red phase (the discarded read, the duplicable pointee,
      and the eager take controls verified green first; AR-5)

### Step 6.2: Implementation

- [ ] 6.2.1 Materialize the deref operand under the substituted ownership and carry the guarded
      take's consumed non-place source — `compiler/src/own/statements.rs`,
      `compiler/src/own/lower.rs`
- [ ] 6.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 6.3: Impl Tests & Review

- [ ] 6.3.1 Write the phase implementation pins (the single `cl_alt` call, the read/empty
      pointer identity, the guarded emptying position and the false-path absence, a re-emission
      determinism pin) in `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 6.3.2 Phase review — dispatch the lenses (correctness + semantics + memory-safety +
      emission-integrity); the memory lens runs both guard selections under ASan; apply the
      corrections
- [ ] 6.3.3 Bounded confirmation: re-run the phase probes, the ASan matrix, and the focused
      suites
- [ ] 6.4.1 Phase record: append the verdicts, fix rounds, and filed rows to `05-findings.md`;
      update the Progress block and commit per the active commit mode

**Verify**: `make check`; the residue suite plain and `RDC_ASAN=1`; the host ASan probes

**Deliverables**:
- [ ] ST-18..ST-19 red then green (both `DEF-376` selections)
- [ ] The batch-L `DEF-329`/guarded-take oracles stay green

---

## Phase 7: Literal destinations and guards

> **Phase baseline tree**: _(recorded at phase start)_
> **Rows**: `DEF-378`, `DEF-379`
> **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-27)
> **Reasoning**: high — the hidden destination and the guard-path fill change emitted C and
> evaluation order; the call counters and the sanitizer gate are the proof

### Step 7.1: Specification Tests

**Reference**: [03-07](03-07-literal-destinations-guards.md) · [07](07-testing-strategy.md)
ST-20..ST-21 · AR-22, AR-23
**Objective**: Pin the hidden destination at every literal position and the owning nested
conditional's guarded fill.

- [ ] 7.1.1 [spec-author] Write the phase cases —
      `compiler/tests/conditional_projection_residue_spec_test.rs` (hiddenviewdestination,
      guardnestedowning) with the brace-list and eager-order controls; the red cases committed
      first
- [ ] 7.1.2 Run the suite and record the red phase (the concrete brace-list, the direct-literal,
      and the unguarded nested controls verified green first; AR-5)

### Step 7.2: Implementation

- [ ] 7.2.1 Route a call-valued view-array element to the binding store at every literal
      position and keep the guarded literal's fill in the guard's evaluated path —
      `compiler/src/own/lower.rs`
- [ ] 7.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 7.3: Impl Tests & Review

- [ ] 7.3.1 Write the phase implementation pins (the bound destination array, the call-with-
      destination form, the field copy, the call counters, the guarded fill position, a
      re-emission determinism pin) in
      `compiler/tests/conditional_projection_residue_impl_test.rs`
- [ ] 7.3.2 Phase review — dispatch the lenses (correctness + semantics + memory-safety +
      emission-integrity); the emission lens runs the base/head differential over the literal
      corpus; apply the corrections
- [ ] 7.3.3 Bounded confirmation: re-run the phase probes, the ASan matrix, and the focused
      suites
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

- [ ] 8.1.1 Run the full verification: `make check`, `make verify`, the strict-build sweep, and
      the ASan suites; record the evidence
- [ ] 8.1.2 Update the roadmap rows: the 18 red rows move to Done with per-row evidence;
      `DEF-361` and `DEF-374` recorded as already green with their control pins; the two parked
      majors stay open; the minors stay untouched
- [ ] 8.1.3 Record the close-out section in the findings ledger and mark the plan Complete in
      `00-index.md`
- [ ] 8.1.4 Refresh the specialist check (`analyze-agents`) and record its state; update the
      `AGENTS.md` test map only if the batch wording needs it

**Verify**: `make check`; `make verify`

**Deliverables**:
- [ ] The 20 rows accounted for: 18 Done with evidence plus 2 controls
- [ ] The parked rows stay open; the minors untouched
- [ ] The plan marked Complete; the specialist check recorded

---

## Dependencies

```
Phase 1 (call identity + written arguments)
        ↓
Phase 2 (substitution I) → Phase 3 (substitution II)
        ↓
Phase 4 (dropped store targets) → Phase 5 (element/store loans)
        ↓
Phase 6 (deref/take channels) → Phase 7 (literal destinations/guards) → Phase 8 (close-out)
```

Arrow legend: `A → B` marks a machinery dependency (B extends the judgment A touches); the
vertical order is the executable sequence — the phases run 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 in
order. P1's written-argument re-check is the hook P2's conditional argument and P3's literal
argument reuse; P2's element substitution feeds P5's loans; P4's store and place work is
independent; P6 and P7 share the guarded/deferred lowering (P6 first so the move-out channel is
settled).

---

## Verification Commands

| Command | Purpose |
|---------|---------|
| `make check` | Per-phase gate: `./spec/check.sh` plus `cargo test --workspace` in the pinned container |
| `make -C docker exec CMD="cargo test --test checker_conformance_spec_test --test checker_conformance_impl_test"` | The P1–P3 focused suites |
| `make -C docker exec CMD="cargo test --test callable_release_integrity_spec_test --test callable_release_integrity_impl_test"` | The P1 `DEF-390` focused suites |
| `make -C docker exec CMD="cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test"` | The P4–P7 focused suites |
| `make -C docker exec CMD="cargo test --test run_fault_spec_test"` | The `DEF-367` bounds fault and the runtime-fault oracle |
| The suites' `RDC_ASAN=1` switch | The memory runs (the suite helper) |
| `make -C docker exec CMD="target/debug/clc --profile host -o out.c prog.clo"` + `cc -std=c99 -O0 -g -Wall -Wextra -Werror` | The strict-build probes (the pinned container) |
| `cc -std=c99 -O0 -g -fsanitize=address -no-pie` + `ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30` | The host ASan/LSan probes |
| `make -C docker exec CMD="target/debug/clc --profile hosted-amiga -o out.c prog.clo"` + the pinned `m68k-amigaos-gcc` | The hosted-amiga emission leg for `DEF-365`/`368`/`378`/`384`/`385` |
| `make -C docker exec CMD="target/debug/clc --profile freestanding -o out.c prog.clo"` | The freestanding compile for `DEF-365` |
| `make verify` | Close-out: every hosted-amiga leg plus the runtime-fault programs |

## Reviewer Dispatch Conventions

- Correctness and semantics via the project briefs (`.opencode/agents/correctness-reviewer.md`,
  `.opencode/agents/semantics-reviewer.md`); memory-safety and emission-integrity via the
  dedicated read-only subagents (`memory-safety-auditor`, `emission-integrity-auditor`).
- Every reviewer packet includes the **blast-radius step** (every call site, exhaustive match,
  and parallel path of the changed construct) and the **test-integrity step** (an assertion must
  fail without the fix); the read-only auditors stay read-only.
- Phase dispatch follows the change surface (AR-27) and the `AGENTS.md` matrix; the `DEF-367`
  emission route (AR-6) and the `DEF-384` type-argument scope (AR-34) are checked at their phases
  and the outcome recorded in the ledger.

## Success Criteria

**Feature is complete when:**

1. [ ] All phases completed
2. [ ] All verification passing (`make check`; `make verify` at close-out)
3. [ ] No warnings/errors; no dead code
4. [ ] Security hardened — not applicable beyond the compiler's own input validation (unchanged)
5. [ ] Documentation updated (the roadmap rows and the findings ledger)
6. [ ] Code reviewed (every phase's declared lenses)
