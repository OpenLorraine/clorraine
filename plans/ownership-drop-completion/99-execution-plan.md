# Execution Plan: Ownership and Drop Model Completion

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-26 17:02
> **Progress**: 23/47 tasks (49%)
> **CodeOps Artifact Schema**: 1

## Overview

Make a fixed array an owned aggregate, lower a by-value fixed-array return,
give fixed and growable arrays owning-element drop glue, and complete the
view-field and fill checks. The work is edits to existing type, checker, and
emitter code, a standard-library body change, and the existing build-and-run
oracle; it adds no new layer, dependency, or harness (AR-21).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Array classification, nullability, element moves, and the fixed-array return | 17 |
| 2 | Nested-array literals | 6 |
| 3 | Array element drop glue, `set`, and un-stage `E90000` | 11 |
| 4 | View-field and fill enforcement | 8 |
| 5 | Full verification and documentation | 5 |

**Total: 47 tasks across 5 phases** (no fabricated hour estimates — scope is bounded by the task-size criteria in the make-plan quality checklist)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of
> truth** for progress. Every task line appears exactly once in this document.
> The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** and the Last Updated stamp after EVERY task —
>    never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning top-to-bottom: the first `[~]` task is resumed first,
>    else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>`.

---

## Phase 1: Array Classification, Nullability, and Element Moves

> **Phase baseline tree**: 2759dbc2e3226f1000bc43466c1794f0ed25f3de
> **Lenses**: compiler and language

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-array-classification.md) · [07](07-testing-strategy.md) · AR-5, AR-6, AR-22, AR-23
**Objective**: Express the required shape, nullability, and move behavior as failing tests.

- [x] 1.1.1 [spec-author] Write specification tests for ST-2 (moved element is `E10020`), ST-13 (shape predicate), ST-14 (aggregate classification), ST-15 (fixed-array lend candidate), ST-16 (fixed array not nullable) — `compiler/tests/ownership_drop_spec_test.rs` ✅ (completed: 2026-09-26 14:57) — note: ST-13/ST-14 are predicate-level and are written as the 1.3.1 unit tests; the spec file carries ST-2, ST-15, ST-16
- [x] 1.1.2 Run `make check` and confirm the new tests fail for the right reason (red phase) ✅ (completed: 2026-09-26 14:57)

**Deliverables**:
- The five ST cases exist and fail before the fix
- Red-phase evidence recorded in the task line

**Verify**: `make check` (expected: new tests red, existing suite green)

### Step 1.2: Implementation

**Reference**: [03-01](03-01-array-classification.md) §Implementation Details · AR-5, AR-6, AR-22, AR-23
**Objective**: Add the shape predicate, route the shape consumers, and consume array-literal elements.

- [x] 1.2.1 Add `Ty::is_slice` and `Ty::is_fixed_array` — `compiler/src/types/mod.rs` ✅ (completed: 2026-09-26 15:00)
- [x] 1.2.2 Route `is_view`, `is_elided_view`, and `is_nullable_capable` through `is_slice`, and add a fixed array to `is_handle_or_aggregate`; leave `is_lend_candidate` unchanged — `compiler/src/borrow.rs` ✅ (completed: 2026-09-26 15:00)
- [x] 1.2.3 Treat a slice as duplicable in `duplicable_at` (fixed arrays still recurse) — `compiler/src/types/table.rs` ✅ (completed: 2026-09-26 15:00; also corrected the unit test that pinned the old slice rule, per the corrected spec)
- [x] 1.2.4 Consume array-literal elements in the ownership walk and the lowering — `compiler/src/own/expr.rs`, `compiler/src/own/lower.rs` ✅ (completed: 2026-09-26 15:00)
- [x] 1.2.5 Run `make check` and confirm the Phase 1 spec tests pass (green phase) ✅ (completed: 2026-09-26 15:00)

**Deliverables**:
- One predicate decides the array shape for the view/elision/nullability/aggregate consumers
- An array literal moves an owned element and copies a duplicable one

**Verify**: `make check`

### Step 1.3: Implementation Tests and Hardening

**Reference**: [03-01](03-01-array-classification.md) §Testing Requirements
**Objective**: Cover the internals and the two classification edges.

- [x] 1.3.1 Add unit tests for `is_slice` / `is_fixed_array` and `is_handle_or_aggregate` — `compiler/src/types/mod.rs`, `compiler/src/borrow.rs` ✅ (completed: 2026-09-26 15:00; the `types` unit tests live in `compiler/src/types/tests.rs`, included by `mod.rs`)
- [x] 1.3.2 Add the lend-candidate regression test (ST-15) — `compiler/tests/ownership_drop_spec_test.rs` ✅ (completed: 2026-09-26 15:00; the spec-level case plus a `borrow.rs` unit test pin that a fixed array stays a lend candidate)
- [x] 1.3.3 Run `make check` (phase gate) ✅ (completed: 2026-09-26 15:00)

**Deliverables**:
- Internals covered by unit tests
- A view return over a fixed-array lend is still accepted

**Verify**: `make check`

### Step 1.4: Fixed-Array Return Lowering (pulled forward per RV-001)

**Reference**: [03-04](03-04-fixed-array-return.md) · [07](07-testing-strategy.md) · PF-001
**Objective**: Lower a by-value fixed-array return now that a fixed array is owned, so classification and return lowering land together and no build accepts invalid C.

- [x] 1.4.1 [spec-author] Write specification tests that `func pair(): String[2]` compiles and that a slice return is unchanged — `compiler/tests/ownership_drop_spec_test.rs` ✅ (completed: 2026-09-26 15:33)
- [x] 1.4.2 Run `make check` and confirm the new tests fail for the right reason (red phase) ✅ (completed: 2026-09-26 15:33; cc rejects the emitted C: `expected expression before '{'` and `assignment to expression with array type`)
- [x] 1.4.3 Emit a fixed-array return as `void` with a trailing `T __cl_ret[N]` out-parameter — `compiler/src/codegen.rs`, `compiler/src/codegen/function.rs` ✅ (completed: 2026-09-26 15:40)
- [x] 1.4.4 Copy the returned value into `__cl_ret` in `emit_return`/`emit_closing_return` and suppress the result local — `compiler/src/codegen/function.rs`, `compiler/src/codegen/cleanup.rs` ✅ (completed: 2026-09-26 15:40)
- [x] 1.4.5 Pass the caller destination at fixed-array call sites — `compiler/src/codegen/calls.rs` ✅ (completed: 2026-09-26 15:40; delivered via `call_with_destination` in `compiler/src/codegen/expr.rs` plus the `Assign`/`emit_call` routing in `function.rs`. Re-review follow-up: the returned-call and discarded-call paths also pass a destination — RV-006/RV-007)
- [x] 1.4.6 Run `make check` and confirm the Step 1.4 spec tests pass (green phase) ✅ (completed: 2026-09-26 15:40)
- [x] 1.4.7 Add a build-and-run case: `let p: String[2] = pair();` releases each element once (ST-1) — `compiler/tests/ownership_drop_impl_test.rs` ✅ (completed: 2026-09-26 15:42; two cases: owning return frees once, scalar return round-trips)

**Deliverables**:
- `func pair(): String[2]` emits `void cl_pair(..., ClString __cl_ret[2])` and writes into it
- A returned `String[2]` compiles as valid C and is released exactly once

**Verify**: `make check`

---

## Phase 2: Nested-Array Literals

> **Phase baseline tree**: 7ce34ab1ab32c3cfaf81c7da88e64542c901e78c
> **Lenses**: compiler and language

### Step 2.1: Specification Tests

**Reference**: [03-05](03-05-nested-array-literals.md) · [07](07-testing-strategy.md) · PF-003
**Objective**: Express the nested array literal as a failing test.

- [x] 2.1.1 [spec-author] Write a specification test that a nested array literal `u8[3][2] = [[1,2],[3,4],[5,6]]` is accepted — `compiler/tests/ownership_drop_spec_test.rs` ✅ (completed: 2026-09-26 16:55; case `should_emit_valid_c_when_a_nested_array_literal_initializes_a_fixed_array`, driving the host C compiler as the oracle)
- [x] 2.1.2 Run `make check` and confirm the new test fails for the right reason (red phase) ✅ (completed: 2026-09-26 16:55; cc rejects the emitted C: `error: expected expression before '{' token` on the three element assignments)

**Deliverables**:
- The ST case exists and fails before the fix

**Verify**: `make check` (expected: new test red)

### Step 2.2: Implementation

**Reference**: [03-05](03-05-nested-array-literals.md) §Implementation Details · PF-003
**Objective**: Emit a nested array literal one dimension per level.

- [x] 2.2.1 Recurse one dimension per level in `emit_array_assign` — `compiler/src/codegen/function.rs` ✅ (completed: 2026-09-26 16:59; `emit_array_assign_to` derives the element type from the remaining extents and recurses on a nested `ArrayLit` element and on a nested fill value; the 1-D path is unchanged)
- [x] 2.2.2 Run `make check` and confirm the Phase 2 spec test passes (green phase) ✅ (completed: 2026-09-26 16:59)

**Deliverables**:
- `u8[3][2] = [[1,2],[3,4],[5,6]]` emits per-dimension element assignments

**Verify**: `make check`

### Step 2.3: Implementation Tests and Hardening

**Reference**: [03-05](03-05-nested-array-literals.md) §Testing Requirements · [07](07-testing-strategy.md) ST-9
**Objective**: Prove the nested literal at run time.

- [x] 2.3.1 Add a build-and-run case: a nested `u8[3][2]` read — `compiler/tests/ownership_drop_impl_test.rs` ✅ (completed: 2026-09-26 17:02; `a_nested_array_literal_keeps_its_element_values` reads `d[2][1] == 6`)
- [x] 2.3.2 Run `make check` (phase gate) ✅ (completed: 2026-09-26 17:02)

**Deliverables**:
- The oracle compiles and runs the emitted C; no invalid-C error

**Verify**: `make check`

---

## Phase 3: Array Element Drop Glue, `set`, and Un-stage `E90000`

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: compiler and language

### Step 3.1: Specification Tests

**Reference**: [03-02](03-02-array-drop-glue.md) · [07](07-testing-strategy.md) · AR-2, AR-3, AR-7, AR-25
**Objective**: Express the accepted growable container, the drop shapes, and `set`, as failing tests.

- [ ] 3.1.1 [spec-author] Write specification and emitter tests for ST-8 (`Array<Buffer>` accepted), ST-17 (`set` drops the replaced element), and the fixed-array (per dimension) and growable drop shapes — `compiler/tests/ownership_drop_spec_test.rs`, `compiler/src/codegen/drop.rs`
- [ ] 3.1.2 Run `make check` and confirm the new tests fail for the right reason (red phase)

**Deliverables**:
- The ST cases exist and fail before the fix

**Verify**: `make check` (expected: new tests red)

### Step 3.2: Implementation

**Reference**: [03-02](03-02-array-drop-glue.md) §Implementation Details · AR-2, AR-3, AR-7, AR-8, AR-25
**Objective**: Emit array element drop, fix `set`, and remove the `E90000` trigger.

- [ ] 3.2.1 Add the per-dimension fixed-array arm to `emit_drop_expr` — `compiler/src/codegen/drop.rs`
- [ ] 3.2.2 Add the growable `Array<T>` arm to `emit_drop_expr` (elements then buffer, null-safe, `unique_local` loop locals) — `compiler/src/codegen/drop.rs`
- [ ] 3.2.3 Make `set<T>` drop the replaced element — `stdlib/std.mem.clo`
- [ ] 3.2.4 Remove `check_sequence_element` and its call — `compiler/src/sema/mod.rs`
- [ ] 3.2.5 Update the `E90000` unit test to assert `Array<move-only>` is accepted — `compiler/src/sema/tests.rs`
- [ ] 3.2.6 Un-stage `E90000`: delete the staged golden, remove its README row, update the `check.sh` comment — `spec/tests/staged/`, `spec/check.sh`
- [ ] 3.2.7 Run `make check` and confirm the Phase 3 spec tests pass (green phase)

**Deliverables**:
- Fixed (per dimension) and growable arrays release their elements once
- `Array<move-only>` is accepted; `set` drops the old element; `E90000` has no live trigger

**Verify**: `make check`

### Step 3.3: Implementation Tests and Hardening

**Reference**: [03-02](03-02-array-drop-glue.md) §Testing Requirements · [07](07-testing-strategy.md) ST-7, ST-9
**Objective**: Prove the runtime drop behavior with the build-and-run oracle.

- [ ] 3.3.1 Add build-and-run cases: a fixed array, a nested `String[2][2]` (ST-9), and `Array<String>` drop exactly once; and `set` drops the replaced element (ST-17) — `compiler/tests/ownership_drop_impl_test.rs`
- [ ] 3.3.2 Run `make check` (phase gate)

**Deliverables**:
- No `free(): double free` and no leak in any run

**Verify**: `make check`

---

## Phase 4: View-Field and Fill Enforcement

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: compiler and language

### Step 4.1: Specification Tests

**Reference**: [03-03](03-03-view-field-enforcement.md) · [07](07-testing-strategy.md) · AR-4, AR-10, AR-20
**Objective**: Express the field and fill rules as failing tests.

- [ ] 4.1.1 [spec-author] Write specification tests for ST-4, ST-5, ST-6, ST-10, ST-11, ST-12 — `compiler/tests/ownership_drop_spec_test.rs`
- [ ] 4.1.2 Run `make check` and confirm the new tests fail for the right reason (red phase)

**Deliverables**:
- The six ST cases exist and fail before the fix

**Verify**: `make check` (expected: new tests red)

### Step 4.2: Implementation

**Reference**: [03-03](03-03-view-field-enforcement.md) §Implementation Details · AR-4, AR-10, AR-20
**Objective**: Reject a slice view field and a move-only fill, and state the rule in the chapters.

- [ ] 4.2.1 Extend the `E10205` check to a slice field — `compiler/src/sema/items.rs`
- [ ] 4.2.2 Add the `E10215` move-only fill guard (reusing the existing message text) — `compiler/src/sema/expr.rs`
- [ ] 4.2.3 Extend the `AL` rule and the `E10215` text — `spec/08-arrays-strings.md`, `spec/14-diagnostics.md`
- [ ] 4.2.4 Run `make check` and confirm the Phase 4 spec tests pass (green phase)

**Deliverables**:
- A slice field is `E10205`; `cstring`/`static str`/fixed arrays stay accepted
- A move-only fill is `E10215`; a duplicable fill is unchanged

**Verify**: `make check`

### Step 4.3: Implementation Tests and Hardening

**Reference**: [03-03](03-03-view-field-enforcement.md) §Testing Requirements · PF-013
**Objective**: Cover the edges of the two checks.

- [ ] 4.3.1 Add edge tests: a nullable view field, a 1-D move-only fill (not a multi-dimensional fill), and a nested fixed array field — `compiler/tests/ownership_drop_spec_test.rs`
- [ ] 4.3.2 Run `make check` (phase gate)

**Deliverables**:
- Nullable `str?`/slice fields are rejected; the allowed fields stay allowed
- The fill edge test uses the 1-D form the guard covers

**Verify**: `make check`

---

## Phase 5: Full Verification and Documentation

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: compiler and language

### Step 5.1: Full Verification

**Reference**: [07](07-testing-strategy.md) · AR-13
**Objective**: Prove the whole feature with the full acceptance command.

- [ ] 5.1.1 Run `make verify` (hosted-amiga legs and the corpus)
- [ ] 5.1.2 Run `./spec/check.sh` and confirm the extended `E10215` text and the `E90000` un-staging are consistent
- [ ] 5.1.3 Confirm no dead code: the removed `E90000` trigger leaves no unused function and no stale helper

**Deliverables**:
- `make verify` green
- `spec/check.sh` OK

**Verify**: `make verify`

### Step 5.2: Documentation and Close-out

**Reference**: [00-index](00-index.md)
**Objective**: Sync the plan state and the roadmap.

- [ ] 5.2.1 Set PLAN-11 to Done and link this plan; update `plans/00-roadmap.md`
- [ ] 5.2.2 Post-phase review: confirm every R1–R10 traces to a passing test, then record the review evidence

**Deliverables**:
- Roadmap synced
- Review evidence recorded

**Verify**: `make check`

---

## Dependencies

```
Phase 1 (classification + nullability + moves)
    ↓
Phase 2 (fixed-array return + nested literals; needs a fixed array to be owned)
    ↓
Phase 3 (drop glue + set; multi-dim drop test needs Phase 2's literal fix)
    ↓
Phase 4 (view-field + fill; uses is_slice from Phase 1)
    ↓
Phase 5 (full verification)
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check` per task, `make verify` final)
3. ✅ No warnings/errors
4. ✅ No dead code — the removed `E90000` trigger leaves nothing unused
5. ✅ Security: the use-after-free, double-free, and leak cases are gone (memory safety is the goal)
6. ✅ Documentation updated — the `AL` rule, the `E10215` text, and the roadmap
7. ✅ Code reviewed
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
