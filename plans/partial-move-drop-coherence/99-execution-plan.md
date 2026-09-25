# Execution Plan: Partial-Move / Drop Coherence

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-25 23:45
> **Progress**: 0/16 tasks (0%)
> **CodeOps Artifact Schema**: 1

## Overview

Fix DEF-11 by making a value-consuming read of an owned place a move that zeroes
the source, and by releasing an assignment's old target after the RHS is
evaluated. The ownership lowering owns the rule (03-01); the emitter stops
dropping a stale save-copy (03-02). Tests are spec-derived regressions in the
existing Rust suites, written red before each fix (AR-4).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Nested move lowering | 7 |
| 2 | Assignment ordering | 6 |
| 3 | Full acceptance and closeout | 3 |

**Total: 16 tasks across 3 phases**

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
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the Last Updated
>    stamp after EVERY task — never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is resumed
>    first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line.
>    The plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented. Failure to keep the marks
> current means progress is invisible after crashes, context resets, or session handoffs.

---

## Phase 1: Nested move lowering

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Expected modification set**: `compiler/src/own/lower.rs`, `compiler/src/own/call.rs`,
> `compiler/src/own/statements.rs`, `compiler/tests/codegen_run_spec_test.rs`,
> `compiler/src/own/tests.rs` (strict scope)
> **Lenses**: correctness, resource lifetime (drop)

### Step 1.1: Specification tests (red)

**Reference**: [03-01](03-01-nested-move-lowering.md) · [07](07-testing-strategy.md) ST-1, ST-7, ST-10 · AR-1, AR-3, AR-4
**Objective**: Pin the temporary-form, `take`-field, and lowered-IR expectations before the helper exists.

- [ ] 1.1.1 [spec-author] Add the ASan compile-and-run helper to `compiler/tests/codegen_run_spec_test.rs` and write the temporary-form regression (ST-1, ASan clean) plus the `take`-field regression (ST-7); add the lowered-IR move case (ST-10) to `compiler/src/own/tests.rs`
- [ ] 1.1.2 Confirm ST-1, ST-7, and ST-10 FAIL (red)

**Deliverables**:
- The temporary-binding form double-frees under ASan today; the `take`-field source is not zeroed; the lowered IR lacks the field move

**Verify**: `cargo test -p clc --test codegen_run_spec_test` and `cargo test -p clc own::tests` show the new cases red

### Step 1.2: Implementation (green)

**Reference**: [03-01](03-01-nested-move-lowering.md) §Implementation Details · AR-1, AR-3
**Objective**: Add the consume helper and apply it at the value-consuming sites.

- [ ] 1.2.1 Add `consume_place` and route the aggregate-literal field/element lowering through it — `compiler/src/own/lower.rs`
- [ ] 1.2.2 Route a `take` argument that is a place (including a field projection) through `consume_place`, and express `emit_store`'s move through the same rule — `compiler/src/own/call.rs`, `compiler/src/own/statements.rs`
- [ ] 1.2.3 Confirm the Step 1.1 cases PASS (green)

**Deliverables**:
- ST-1, ST-7, ST-10 pass; a duplicable place is still copied

**Verify**: `cargo test -p clc --test codegen_run_spec_test` and `cargo test -p clc own::tests`

### Step 1.3: Implementation tests and phase verification

**Reference**: [03-01](03-01-nested-move-lowering.md) §Testing Requirements
**Objective**: Cover the copy branch and verify the phase.

- [ ] 1.3.1 Add implementation tests for the duplicable copy branch and the nested owned move — `compiler/src/own/tests.rs`
- [ ] 1.3.2 Full phase verification — `make check`

**Deliverables**:
- All verification passing

**Verify**: `make check` (exit 0)

---

## Phase 2: Assignment ordering

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Expected modification set**: `compiler/src/codegen/function.rs`,
> `compiler/tests/codegen_run_spec_test.rs`, `compiler/tests/stage_spec_test.rs` (strict scope)
> **Lenses**: correctness, resource lifetime (drop)

### Step 2.1: Specification tests (red)

**Reference**: [03-02](03-02-assignment-ordering.md) · [07](07-testing-strategy.md) ST-2, ST-3, ST-4, ST-5, ST-6, ST-8, ST-9 · AR-3, AR-11..AR-14
**Objective**: Pin the reassignment, self-assignment, multi-field, borrow control, and emitted-C expectations.

- [ ] 2.1.1 [spec-author] Write regressions — ST-2 (generic struct-literal reassignment, ASan clean), ST-3 (non-generic), ST-4 (borrow control `open/dup/close/close`), ST-5 (self-assignment), ST-6 (multi-field), ST-8 (non-move byte identity), ST-9 (emitted C has no `__cl_save` and zeroes the moved field) — `compiler/tests/codegen_run_spec_test.rs`, `compiler/tests/stage_spec_test.rs`
- [ ] 2.1.2 Confirm ST-2, ST-3, ST-5, ST-6 FAIL (red)

**Deliverables**:
- ST-2/ST-3/ST-5/ST-6 fail today (double free or stale save); ST-4, ST-8, ST-9 are controls

**Verify**: `cargo test -p clc --test codegen_run_spec_test` and `cargo test -p clc --test stage_spec_test` show the new cases red

### Step 2.2: Implementation (green)

**Reference**: [03-02](03-02-assignment-ordering.md) §Implementation Details · AR-3
**Objective**: Evaluate the RHS, release the old target, then store; remove the save path.

- [ ] 2.2.1 Rework `emit_assign` to evaluate the RHS into a temporary, release the old target, then store; remove `value_reads_base` and the save/restore branch — `compiler/src/codegen/function.rs`
- [ ] 2.2.2 Confirm the Step 2.1 cases PASS (green)

**Deliverables**:
- ST-2, ST-3, ST-4, ST-5, ST-6, ST-8, ST-9 pass

**Verify**: `cargo test -p clc --test codegen_run_spec_test` and `cargo test -p clc --test stage_spec_test`

### Step 2.3: Implementation tests and phase verification

**Reference**: [03-02](03-02-assignment-ordering.md) §Testing Requirements
**Objective**: Cover the reordered emission and verify the phase.

- [ ] 2.3.1 Add implementation tests that a moved-field reassignment emits no `__cl_save` and that the borrow case keeps its order — `compiler/tests/stage_spec_test.rs`
- [ ] 2.3.2 Full phase verification — `make check`

**Deliverables**:
- All verification passing

**Verify**: `make check` (exit 0)

---

## Phase 3: Full acceptance and closeout

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Expected modification set**: `compiler/tests/`, `plans/00-roadmap.md`,
> `plans/partial-move-drop-coherence/99-execution-plan.md` (strict scope)
> **Lenses**: regression, documentation

### Step 3.1: Acceptance and documentation

**Reference**: [01-requirements.md](01-requirements.md) §Acceptance Criteria · AR-2, AR-5
**Objective**: Prove the whole feature and record completion.

- [ ] 3.1.1 Run the ASan matrix across ST-1..ST-6 (generic and non-generic forms); confirm every form is ASan clean
- [ ] 3.1.2 Full acceptance run — `make verify`
- [ ] 3.1.3 Mark DEF-11 Fixed on `plans/00-roadmap.md`, set this plan's roadmap row to Done, and set PLAN-10 back to Executing (Phase 4 pending); update `AGENTS.md` only if a factual note changed

**Deliverables**:
- `make verify` green; roadmap and plan status updated

**Verify**: `make verify` (exit 0)

---

## Dependencies

```
Phase 1 (consume owned places)
    ↓
Phase 2 (assignment releases the old target after the RHS)
    ↓
Phase 3 (full acceptance)
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check` per task; `make verify` final)
3. ✅ No warnings/errors
4. ✅ No dead code — `value_reads_base` and the save/restore branch are removed; the duplicated consume rule has one owner
5. ✅ Security N/A (AR-9) — the fix closes a double-free; no external input surface
6. ✅ Documentation updated — roadmap DEF-11 fixed; plan done; PLAN-10 Phase 4 resumes
7. ✅ Code reviewed (strict profile — independent review of each phase's diff)
8. ✅ No `spec/` file modified
9. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
