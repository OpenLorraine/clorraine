# Execution Plan: Partial-Move / Drop Coherence

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-26 00:20
> **Progress**: 0/10 tasks (0%)
> **CodeOps Artifact Schema**: 1

## Overview

Fix DEF-11 by consuming an owned place nested in an aggregate literal as a move:
the ownership stage records the move, and the lowering emits the copy-plus-zero
so the source is emptied before the literal is built. Once the field is consumed,
the assignment fast path already releases the (empty) old target correctly, so no
assignment change is made (AR-3). Tests are spec-derived regressions in the
existing suites, written red before each fix (AR-4).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Owned literal-field consumption | 7 |
| 2 | Full acceptance and closeout | 3 |

**Total: 10 tasks across 2 phases**

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

## Phase 1: Owned literal-field consumption

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Expected modification set**: `compiler/src/own/expr.rs`, `compiler/src/own/lower.rs`,
> `compiler/src/own/tests.rs`, `compiler/tests/stage_spec_test.rs`,
> `compiler/tests/codegen_run_spec_test.rs`, `compiler/tests/partial_move_impl_test.rs` (strict scope)
> **Lenses**: correctness, resource lifetime (drop)

### Step 1.1: Specification tests (red)

**Reference**: [03-01](03-01-nested-move-lowering.md) · [07](07-testing-strategy.md) ST-1..ST-10 · AR-1, AR-3, AR-14, AR-15
**Objective**: Pin the move, source-zero, and diagnostic expectations before the fix.

- [ ] 1.1.1 [spec-author] Write regressions — ST-1/ST-2/ST-9 (temporary and reassignment forms; emitted C zeroes the source and has no stale save), ST-6 (`E10020`), ST-7 (`E10026`), ST-10 (`E10034`) in `compiler/tests/stage_spec_test.rs`; ST-4 (borrow control) and ST-8 (non-reading fast-path control) in `compiler/tests/codegen_run_spec_test.rs`
- [ ] 1.1.2 Confirm ST-1, ST-2, ST-6, ST-7, ST-9, and ST-10 FAIL (red); ST-4 and ST-8 are controls

**Deliverables**:
- The temporary and reassignment forms double-free (no source zero); a double read is not `E10020`; a deeper path is not `E10026`; a borrowed field is not `E10034`

**Verify**: `make -C docker exec CMD="cargo test -p clc --test stage_spec_test"` and `make -C docker exec CMD="cargo test -p clc --test codegen_run_spec_test"` show the new cases red

### Step 1.2: Implementation (green)

**Reference**: [03-01](03-01-nested-move-lowering.md) §Implementation Details · AR-1, AR-3
**Objective**: Record the move in the ownership flow and emit the source zero.

- [ ] 1.2.1 Mark a consumed non-duplicable struct-literal field as moved via `move_target` — `compiler/src/own/expr.rs`
- [ ] 1.2.2 Add `consume_place` (with the `mentions_type_param` guard) and route the `StructLit` field lowering through it — `compiler/src/own/lower.rs`
- [ ] 1.2.3 Confirm the Step 1.1 cases PASS (green)

**Deliverables**:
- ST-1, ST-2, ST-6, ST-7, ST-9, ST-10 pass; a duplicable or type-parameter place is still copied

**Verify**: `make -C docker exec CMD="cargo test -p clc --test stage_spec_test"`, `make -C docker exec CMD="cargo test -p clc --test codegen_run_spec_test"`

### Step 1.3: Implementation tests and phase verification

**Reference**: [03-01](03-01-nested-move-lowering.md) §Testing Requirements
**Objective**: Add the ASan oracle and cover the copy/guard branches.

- [ ] 1.3.1 Add the ASan compile-and-run helper (`ASAN_OPTIONS=detect_leaks=0`) to a new implementation suite and cover the duplicable/type-parameter copy branches — `compiler/tests/partial_move_impl_test.rs`, `compiler/src/own/tests.rs`
- [ ] 1.3.2 Full phase verification — `make check`

**Deliverables**:
- All verification passing; the ST-1/ST-2/ST-3/ST-5 forms are ASan clean

**Verify**: `make check` (exit 0)

---

## Phase 2: Full acceptance and closeout

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Expected modification set**: `compiler/tests/partial_move_impl_test.rs`, `plans/00-roadmap.md`,
> `plans/partial-move-drop-coherence/99-execution-plan.md` (strict scope)
> **Lenses**: regression, documentation

### Step 2.1: Acceptance and documentation

**Reference**: [01-requirements.md](01-requirements.md) §Acceptance Criteria · AR-2, AR-5
**Objective**: Prove the whole feature and record completion.

- [ ] 2.1.1 Run the ASan matrix across ST-1, ST-2, ST-3, and ST-5 (generic and non-generic); confirm every form is ASan clean and frees once
- [ ] 2.1.2 Full acceptance run — `make verify`
- [ ] 2.1.3 Mark DEF-11 Fixed on `plans/00-roadmap.md`, set this plan's roadmap row to Done, and set PLAN-10 back to Executing (Phase 4 pending); update `AGENTS.md` only if a factual note changed

**Deliverables**:
- `make verify` green; roadmap and plan status updated

**Verify**: `make verify` (exit 0)

---

## Dependencies

```
Phase 1 (consume owned literal fields)
    ↓
Phase 2 (full acceptance)
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check` per task; `make verify` final)
3. ✅ No warnings/errors
4. ✅ No dead code — the consume rule has one owner in the ownership stage; the assignment emitter is untouched
5. ✅ Security N/A (AR-9) — the fix closes a double-free; no external input surface
6. ✅ Documentation updated — roadmap DEF-11 fixed; plan done; PLAN-10 Phase 4 resumes
7. ✅ Code reviewed (strict profile — independent review of the phase diff)
8. ✅ No `spec/` file modified
9. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
