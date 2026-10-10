# Execution Plan: Evaluation and Emission Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-10 12:41
> **Progress**: 6/49 tasks (12%)
> **CodeOps Artifact Schema**: 1

## Overview

Five phases close the 24 fix rows ([AR-B1](00-ambiguity-register.md)): module-initializer
integrity (DEF-117, 121, 122), strict-build emission shapes (DEF-76, 97, 80, 90, 98, 112, 120,
128), value typing and checker conformance (DEF-99, 111, 108, 107, 118, 129), write-mutability
rules (DEF-119, 124), and ownership lifetimes and releases (DEF-95, 96, 125, 126, 127). DEF-103
closes with its evidence in the plan close. Each phase follows the specification-first ordering
and cites its owning [03-XX](00-index.md) document and the ST-cases in
[07-testing-strategy.md](07-testing-strategy.md).

Execution commit mode: **auto-commit** (user instruction) — commit and push each verified task
via the git-commit skill in push mode.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Module-initializer integrity (DEF-117, 121, 122) | 8 |
| 2 | Strict-build emission shapes (DEF-76, 97, 80, 90, 98, 112, 120, 128) | 12 |
| 3 | Value typing and checker conformance (DEF-99, 111, 108, 107, 118, 129) | 12 |
| 4 | Write-mutability rules (DEF-119, 124) | 7 |
| 5 | Ownership lifetimes and releases (DEF-95, 96, 125, 126, 127) | 10 |

**Total: 49 tasks across 5 phases** (no fabricated hour estimates — scope is bounded by the
task-size criteria in the make-plan quality checklist)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the Last Updated
>    stamp after EVERY task — never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is resumed
>    first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line.
>    The plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

---

## Phase 1: Module-initializer integrity (DEF-117, DEF-121, DEF-122)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer + emission-integrity auditor + memory-safety auditor
> (codegen and evaluation-order changes; AR-B16)
> **Reasoning**: high — emission contracts and double evaluation

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-module-initializer-integrity.md) §Proposed Changes ·
[AR-B5, AR-B13](00-ambiguity-register.md)
**Objective**: Pin the six init positions, the single-evaluation run, and the literal rejection
before any implementation.

- [x] 1.1.1 [spec-author] Write specification tests from ST-1 – ST-7 — `compiler/tests/eval_emission_integrity_spec_test.rs` ✅ (completed: 2026-10-10 12:13)
- [x] 1.1.2 Run the spec suite and record the red phase (expected: ST-1 – ST-6 fail against the broken emission; ST-7's `E10200` assertion fails while the `slice()` control passes) ✅ (completed: 2026-10-10 12:13)

**Red-phase evidence (2026-10-10 12:13)** — the suite (7 tests) fails exactly as designed: 0
passed, 7 failed. ST-1, ST-2, ST-4, and ST-5 fail with the destination-less calls (`too few
arguments to function 'cl_spec_eei_…_mk'`), ST-3 with the array assignment (`assignment to
expression with array type`), ST-6 with the double-evaluated guard (the second `idx()` call
reads out of range, so the run faults with `error E11211: index out of range` and never reaches
the expected exit), and ST-7's rejection is absent (the literal slice is accepted; its `slice()`
control passes). The first run also caught two missing `slice` imports in the test sources, fixed
before recording.

**Deliverables**:
- Specification suite with ST-1 – ST-7
- Red-phase evidence with per-pin justification

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`

---

### Step 1.2: Implementation

**Reference**: [03-01](03-01-module-initializer-integrity.md) §Proposed Changes
**Objective**: Bind every fixed-array call through its destination in module initializers,
materialize impure guard operands once, and reject the backing-less slice literal.

- [x] 1.2.1 Add the module-init materialization pass for fixed-array calls across the six positions — `compiler/src/codegen/program.rs`, `compiler/src/codegen/expr.rs` ✅ (completed: 2026-10-10 12:31)
- [x] 1.2.2 Apply the guarded-sub-operand materialization in the scalar single-assignment path — `compiler/src/codegen/program.rs` ✅ (completed: 2026-10-10 12:34)
- [x] 1.2.3 Stop adopting `None`-bearing expected extents in `check_array_literal`, so the fix reaches `E10200` for module and body spellings — `compiler/src/sema/expr.rs` ✅ (completed: 2026-10-10 12:40)
- [x] 1.2.4 Run the spec suite and `./spec/check.sh` to green (ST-1 – ST-7); on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-10 12:40)

**Deliverables**:
- The init materialization and evaluation fixes and the checker rejection
- Green spec suite and green frozen-suite check

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`; `./spec/check.sh`

---

### Step 1.3: Implementation Tests & Hardening

**Reference**: [03-01](03-01-module-initializer-integrity.md) §Testing Requirements
**Objective**: Pin the emitted destinations, the shared operand, and record the phase review
evidence.

- [ ] 1.3.1 Write the Phase 1 implementation pins (destination-carrying emission per position, the shared init local for the guard operand, byte-identical re-emission of unaffected initializers) — `compiler/tests/eval_emission_integrity_impl_test.rs`
- [ ] 1.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode

**Deliverables**:
- Implementation pins for Phase 1
- Review evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test eval_emission_integrity_impl_test"`; `make check`

---

## Phase 2: Strict-build emission shapes (DEF-76, 97, 80, 90, 98, 112, 120, 128)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer + emission-integrity auditor (codegen and symbol-name changes;
> AR-B16)
> **Reasoning**: high — emitted-C contracts and order changes

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-strict-build-emission.md) §Proposed Changes ·
[AR-B13](00-ambiguity-register.md)
**Objective**: Pin each strict-build shape and the bounded-field run before any implementation.

- [ ] 2.1.1 [spec-author] Write specification tests from ST-8 – ST-16 — `compiler/tests/eval_emission_integrity_spec_test.rs`
- [ ] 2.1.2 Run the spec suite and record the red phase (expected: ST-8, ST-10 – ST-16 fail against the strict build or run behavior; ST-9 reproduces the recorded shape first)

**Deliverables**:
- Specification suite with ST-8 – ST-16
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`

---

### Step 2.2: Implementation

**Reference**: [03-02](03-02-strict-build-emission.md) §Proposed Changes
**Objective**: Fix each emission shape; reproduce DEF-97 first and fix it with DEF-76 when the
root matches.

- [ ] 2.2.1 Reproduce the DEF-97 shape from `codegen_run_spec_test`, then skip the owned `len` binding for a fixed-array argument — `compiler/src/own/lower.rs`
- [ ] 2.2.2 Emit the struct zero-helper family after the struct definitions and return it from `empty_return_value` — `compiler/src/codegen/program.rs`, `compiler/src/codegen/cleanup.rs`
- [ ] 2.2.3 Provide the boolean type in the freestanding preamble — `compiler/src/codegen/program.rs`
- [ ] 2.2.4 Coerce a view argument to its pointer for an uncontracted extern parameter — `compiler/src/codegen/calls.rs`
- [ ] 2.2.5 Seed the generated spellings into the name de-dup pass — `compiler/src/codegen/function.rs`
- [ ] 2.2.6 Split the slice-typedef emission (pointer-element before the struct definitions, complete-row-element after) and revise the order record — `compiler/src/codegen/program.rs`, `compiler/src/codegen.rs`
- [ ] 2.2.7 Resolve the bounded field call through the instance field/closure call path — `compiler/src/own/call.rs` or `compiler/src/codegen/expr.rs` (chosen by probe)
- [ ] 2.2.8 Run the spec suite, `./spec/check.sh`, and the independently affected recorded suites to green; on any failure STOP and fix the implementation, never the test

**Deliverables**:
- The seven emission fixes
- Green suites and frozen-suite check

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`; `./spec/check.sh`; `make -C docker exec CMD="cargo test --test codegen_run_spec_test --test freestanding_spec_test"`

---

### Step 2.3: Implementation Tests & Hardening

**Reference**: [03-02](03-02-strict-build-emission.md) §Testing Requirements
**Objective**: Pin the internals and record the phase review evidence.

- [ ] 2.3.1 Write the Phase 2 implementation pins (the absent `__cl_owned` binding, the `cl_zero_*` spellings, the seeded de-dup spelling, the typedef order, the bound-field call shape, byte-identity where unaffected) — `compiler/tests/eval_emission_integrity_impl_test.rs`
- [ ] 2.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode

**Deliverables**:
- Implementation pins for Phase 2
- Review evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test eval_emission_integrity_impl_test"`; `make check`

---

## Phase 3: Value typing and checker conformance (DEF-99, 111, 108, 107, 118, 129)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer + semantics reviewer (checker and typing changes; AR-B16)
> **Reasoning**: high — language-rule judgments and one new diagnostic

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-value-typing-conformance.md) §Proposed Changes ·
[AR-B4, AR-B6, AR-B7, AR-B8, AR-B9](00-ambiguity-register.md)
**Objective**: Land the approved `E10043` row and golden, then pin the corrections before any
implementation.

- [ ] 3.1.1 [spec] Add the `E10043` registry row (`spec/14-diagnostics.md`), the golden `spec/tests/compile-fail/E10043_str_loop.clo`, and the compile-fail driver registration (AR-B6)
- [ ] 3.1.2 [spec-author] Write specification tests from ST-17 – ST-25 — `compiler/tests/eval_emission_integrity_spec_test.rs`
- [ ] 3.1.3 Run the spec suite and `./spec/check.sh`; record the red phase (expected: ST-17, ST-18, ST-21 – ST-25 fail or reject wrongly; ST-19/20 and the controls pass as deliberate pins)

**Deliverables**:
- The registry row and golden
- Specification suite with ST-17 – ST-25
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`; `./spec/check.sh`

---

### Step 3.2: Implementation

**Reference**: [03-03](03-03-value-typing-conformance.md) §Proposed Changes
**Objective**: Resolve the substituted projection, keep the nullable inference building, and land
the three rejections.

- [ ] 3.2.1 Reproduce DEF-99 and fix the `len` fold (with DEF-111 when they share the root) — `compiler/src/codegen/calls.rs`, `compiler/src/codegen/expr.rs`, `compiler/src/own/lower.rs`
- [ ] 3.2.2 Resolve the generic field projection through the substituted base — the value path per the chosen lever
- [ ] 3.2.3 Keep the nullable inference and make the declaration and `null` spelling agree — `compiler/src/sema/stmt.rs`, `compiler/src/codegen/types.rs`, `compiler/src/codegen/expr.rs`
- [ ] 3.2.4 Reject `str`/`static str` pair equality with `E10222` (null-pair exemption and `cstring` stay) — `compiler/src/sema/ops.rs`
- [ ] 3.2.5 Reject a `str` element loop with `E10043` — `compiler/src/sema/stmt.rs` or `compiler/src/own/loops.rs`
- [ ] 3.2.6 Reject a member call on a bare-parameter receiver with `E10212` — `compiler/src/sema/call.rs`
- [ ] 3.2.7 Run the spec suite and `./spec/check.sh` to green (ST-17 – ST-25 and the golden); on any failure STOP and fix the implementation, never the test

**Deliverables**:
- The typing/projection fixes and the three checker rejections
- Green spec suite and frozen-suite check

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`; `./spec/check.sh`

---

### Step 3.3: Implementation Tests & Hardening

**Reference**: [03-03](03-03-value-typing-conformance.md) §Testing Requirements
**Objective**: Pin the internals and record the phase review evidence.

- [ ] 3.3.1 Write the Phase 3 implementation pins (the per-instance projection spelling, unchanged concrete emissions, the one-report counts and spans for E10222/E10043/E10212, the null-local declaration, byte-identity of the controls) — `compiler/tests/eval_emission_integrity_impl_test.rs`
- [ ] 3.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode

**Deliverables**:
- Implementation pins for Phase 3
- Review evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test eval_emission_integrity_impl_test"`; `make check`

---

## Phase 4: Write-mutability rules (DEF-119, DEF-124)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer + semantics reviewer (language-rule changes; AR-B16)
> **Reasoning**: high — user-ruled write checks and one registry wording

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-write-mutability-rules.md) §Proposed Changes ·
[AR-B10, AR-B11](00-ambiguity-register.md)
**Objective**: Land the approved `E10042` wording and pin the six cases before any implementation.

- [ ] 4.1.1 [spec] Amend the `E10042` registry row wording (`spec/14-diagnostics.md`) and write specification tests from ST-26 – ST-31 — `compiler/tests/eval_emission_integrity_spec_test.rs`
- [ ] 4.1.2 Run the spec suite and `./spec/check.sh`; record the red phase (expected: ST-26, ST-27, ST-29 fail — accepted today; ST-28 and the controls pass as deliberate pins)

**Deliverables**:
- The registry wording
- Specification suite with ST-26 – ST-31
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`; `./spec/check.sh`

---

### Step 4.2: Implementation

**Reference**: [03-04](03-04-write-mutability-rules.md) §Proposed Changes
**Objective**: Route the value-position updates through the statement checks and add `cstring`
to the immutable-view predicate.

- [ ] 4.2.1 Route `check_update_value_write` through `read_place`+`write_place` (locals) and the const/immutable checks (globals), one report per site — `compiler/src/own/expr.rs`
- [ ] 4.2.2 Add `TyKind::CString` to `report_immutable_view_write` — `compiler/src/own/place.rs`
- [ ] 4.2.3 Run the spec suite and `./spec/check.sh` to green (ST-26 – ST-31); on any failure STOP and fix the implementation, never the test

**Deliverables**:
- The write-check routing and the predicate extension
- Green spec suite and frozen-suite check

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`; `./spec/check.sh`

---

### Step 4.3: Implementation Tests & Hardening

**Reference**: [03-04](03-04-write-mutability-rules.md) §Testing Requirements
**Objective**: Pin the internals and record the phase review evidence.

- [ ] 4.3.1 Write the Phase 4 implementation pins (the exact code per shape, one report per site, the registry wording, byte-identity of the accepted controls) — `compiler/tests/eval_emission_integrity_impl_test.rs`
- [ ] 4.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode

**Deliverables**:
- Implementation pins for Phase 4
- Review evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test eval_emission_integrity_impl_test"`; `make check`

---

## Phase 5: Ownership lifetimes and releases (DEF-95, 96, 125, 126, 127)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer + memory-safety auditor + semantics reviewer (ownership,
> captures, releases; AR-B16)
> **Reasoning**: high — memory-safety consequences and capture-rule changes

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-ownership-lifetimes.md) §Proposed Changes ·
[AR-B2, AR-B12, AR-B13](00-ambiguity-register.md)
**Objective**: Pin the leak-free runs and the ruled rejections before any implementation.

- [ ] 5.1.1 [spec-author] Write specification tests from ST-32 – ST-41 — `compiler/tests/eval_emission_integrity_spec_test.rs`
- [ ] 5.1.2 Run the spec suite and record the red phase (expected: ST-32, ST-34, ST-36 and the block/conditional rejections fail; the controls pass as deliberate pins — the leak oracles are the ASan probes in the green step for the plain-run cases)

**Deliverables**:
- Specification suite with ST-32 – ST-41
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`

---

### Step 5.2: Implementation

**Reference**: [03-05](03-05-ownership-lifetimes.md) §Proposed Changes
**Objective**: Close each memory hole and the capture-duty gaps.

- [ ] 5.2.1 Apply the func-value call's own modes and materialize owned temporary arguments — `compiler/src/own/call.rs`, `compiler/src/own/call_check.rs`
- [ ] 5.2.2 Suppress the not-lend duty for capture bindings and record the capture-move duty at the block return and take sites; update the closure batch's block-body pin deliberately — `compiler/src/own/place.rs`, `compiler/src/sema/stmt.rs`, `compiler/src/sema/call.rs`, `compiler/src/sema/expr.rs`
- [ ] 5.2.3 Recurse conditional arms in the capture rejection and the recorder with per-site dedupe — `compiler/src/sema/expr.rs`
- [ ] 5.2.4 Arm the superseded-result gate for an ordinary store whose deferred scopes can preempt (byte-identity where unreachable) — `compiler/src/codegen/cleanup.rs`
- [ ] 5.2.5 Give a view bound from an owned projection temporary a binding-scoped owner, with `StructLit` descent in the temp-owner walk — `compiler/src/own/statements.rs`, `compiler/src/own/lower.rs`
- [ ] 5.2.6 Run the spec suite to green (ST-32 – ST-41) and the host ASan probes for the DEF-95, DEF-96, DEF-125, and DEF-127 shapes; on any failure STOP and fix the implementation, never the test

**Deliverables**:
- The five ownership/release fixes
- Green spec suite and clean ASan probes

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`; the host ASan probes are the memory oracle (emit, `cc -std=c99 -g -fsanitize=address -no-pie`, run with leak detection; AR-B14)

---

### Step 5.3: Implementation Tests & Hardening

**Reference**: [03-05](03-05-ownership-lifetimes.md) §Testing Requirements
**Objective**: Pin the internals, record the phase review evidence, and close the plan.

- [ ] 5.3.1 Write the Phase 5 implementation pins (the owner binding and release counts, the preemption-edge drop shape, the argument materialization shape, the duty/report counts, byte-identity where pinned) — `compiler/tests/eval_emission_integrity_impl_test.rs`
- [ ] 5.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode; run `make check` and the `make verify` legs; record the DEF-103 closure (marker site, recursion, top-level analogue, fail-closed proof) and sync the roadmap to Done

**Deliverables**:
- Implementation pins for Phase 5
- Review evidence recorded; `make check` and `make verify` green
- DEF-103 closed with its evidence; roadmap synced

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test eval_emission_integrity_impl_test"`; `make check`; `make verify`

---

## Dependencies

```
Phase 1 (init emission + evaluation)
    ↓  (the init path's fixes precede the emission-shape work)
Phase 2 (emission shapes; DEF-128 shares the call-emission surface with P1's DEF-121 fix)
    ↓  (independent of P3–P5 code surfaces; the spec suite accumulates)
Phase 3 (typing/checker; DEF-99/111 share the fold root)
    ↓  (independent surfaces)
Phase 4 (write rules)
    ↓
Phase 5 (ownership/releases; DEF-126 extends the closure batch's duties)
```

The phases are independent in behavior; the sequence keeps the shared suite file and the
`codegen/expr.rs`/`own/lower.rs` surfaces touched by one phase at a time.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All five phases completed (49/49 tasks)
2. ✅ All verification passing (`make check` per phase; `make verify` at close-out)
3. ✅ No warnings/errors (strict builds for every covered emission; the corpus legs green)
4. ✅ No dead code — no unused parameters, functions, or modules
5. ✅ Memory hardening — host ASan probes clean for DEF-95, 96, 121, 125, 127
6. ✅ Documentation — the approved registry edits and goldens; plain-language comments updated
7. ✅ Code reviewed per the repository's phase-review dispatch (AR-B16)
8. ✅ DEF-103 closed with its evidence; roadmap rows at Done; post-completion project re-analysis
   (handled by the exec-plan skill)
