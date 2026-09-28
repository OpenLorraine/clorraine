# Execution Plan: Element Loop Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-28 10:44
> **Progress**: 5/16 tasks (31%)
> **CodeOps Artifact Schema**: 1

## Overview

Fix DEF-16 (owned-element read double free) and DEF-17 (nested `for` step
mis-selection). Both fixes are small and independent; each phase follows the
specification-first ordering (spec tests → red → implement → green → impl tests →
verify).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Element read ownership (DEF-16) | 6 |
| 2 | Nested loop step (DEF-17) | 6 |
| 3 | Hardening and close-out | 4 |

**Total: 16 tasks across 3 phases**

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
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

---

## Phase 1: Element Read Ownership (DEF-16)

> **Phase baseline tree**: `01154d4c435edd58be8bbf84d7fe52a9746790e6`
> **Expected modifications**: `compiler/tests/element_loop_integrity_spec_test.rs`, `compiler/tests/element_loop_integrity_impl_test.rs`, `compiler/src/own/lower.rs`, `99-execution-plan.md`, `plans/00-roadmap.md`
> **Scope mode**: strict
> **Lenses**: correctness, maintainability, standards (strict profile — independent review of the phase diff)

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-element-read-ownership.md) §Behavior Matrix · [07](07-testing-strategy.md) ST-1 – ST-13 · AR #6, AR #9, AR #13
**Objective**: Write the DEF-16 run and rejection oracle before any implementation.

- [x] 1.1.1 [spec-author] Write the DEF-16 specification cases ST-1 – ST-13 in `compiler/tests/element_loop_integrity_spec_test.rs`, with a bounded run helper and a rejection helper ✅ (completed: 2026-09-28 10:40)
- [x] 1.1.2 Verify the RED phase: ST-1 – ST-8 fail with the double-free abort, and ST-9 – ST-13 pass pre-fix (rejections, regression guard, and the existing `E11211` trap); record per-case evidence on this line. Any deviation is an oracle or test defect: stop and record it, do not adjust a case to match ✅ (completed: 2026-09-28 10:39) — red evidence: ST-1 – ST-8 abort with `free(): double free detected in tcache 2` (exit 134); ST-9/ST-10 `E10026`; ST-11 exit 3; ST-12 exit 2; ST-13 `E11211` trap

**Deliverables**:
- The suite exists and compiles
- Every DEF-16 case's red-phase state is recorded

**Verify**: `make check`

### Step 1.2: Implementation

**Reference**: [03-01](03-01-element-read-ownership.md) §Proposed Changes · AR #6, AR #12
**Objective**: Make sub-object reads of owned containers lend instead of materializing.

- [x] 1.2.1 Add the `projects_from_place` predicate and use it in `materialize_owned_argument` — `compiler/src/own/lower.rs` ✅ (completed: 2026-09-28 10:41)
- [x] 1.2.2 Verify the GREEN phase: ST-1 – ST-13 pass and `make check` is green ✅ (completed: 2026-09-28 10:42) — green evidence: 13/13 spec cases pass; `make check` → 711 tests passed, spec check OK (27 files, 104 compile-fail tests)

**Deliverables**:
- Owned element reads no longer bind a scope temporary
- Ownerless temporaries still materialize and release once

**Verify**: `make check`

### Step 1.3: Implementation Tests & Hardening

**Reference**: [07](07-testing-strategy.md) §Implementation Tests · AR #10
**Objective**: Pin the emitted shape and cover internals.

- [x] 1.3.1 Write the element-read shape tests in `compiler/tests/element_loop_integrity_impl_test.rs` (no `__cl_owned` binding for an element argument; the owned-temporary path still binds and releases; the placeholder assertion pins the element-read shape: the concat result keeps its single owned binding while the element itself is not bound) ✅ (completed: 2026-09-28 10:44)
- [ ] 1.3.2 Run the full gate: `make check` green and the workspace builds warning-free

**Deliverables**:
- Shape tests pass
- All verification passing

**Verify**: `make check`

---

## Phase 2: Nested Loop Step (DEF-17)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness, maintainability, standards (strict profile — independent review of the phase diff)

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-nested-loop-step.md) · [07](07-testing-strategy.md) ST-14 – ST-21 · AR #3, AR #8
**Objective**: Extend the oracle with the nested-loop cases before the emitter change.

- [ ] 2.1.1 [spec-author] Add the DEF-17 specification cases ST-14 – ST-21 to `compiler/tests/element_loop_integrity_spec_test.rs`
- [ ] 2.1.2 Verify the RED phase: ST-14 – ST-18 and ST-21 fail with the bounded-run timeout and ST-19/ST-20 pass; record per-case evidence on this line

**Deliverables**:
- Every DEF-17 case's red-phase state is recorded

**Verify**: `make check`

### Step 2.2: Implementation

**Reference**: [03-02](03-02-nested-loop-step.md) §Proposed Changes · AR #7, AR #8
**Objective**: Carry the step block in the IR shape and delete the search.

- [ ] 2.2.1 Add `step: Option<BlockId>` to `BranchShape::Loop`, set it in `own/loops.rs` at all three construction sites, consume it in `emit_loop`, and delete `for_step_block` — `compiler/src/ir/control.rs`, `compiler/src/own/loops.rs`, `compiler/src/codegen/control.rs`
- [ ] 2.2.2 Verify the GREEN phase: ST-14 – ST-21 pass and `make check` is green

**Deliverables**:
- Nested `for` loops terminate with their own step
- Non-nested emitted C is unchanged

**Verify**: `make check`

### Step 2.3: Implementation Tests & Hardening

**Reference**: [07](07-testing-strategy.md) §Implementation Tests · AR #7
**Objective**: Pin the nested-loop emitted shape and determinism.

- [ ] 2.3.1 Extend `compiler/tests/element_loop_integrity_impl_test.rs`: the inner `for` header carries its own increment, no dead step label or duplicate loop remains, and two emissions are byte-identical
- [ ] 2.3.2 Run the full gate: `make check` green and the workspace builds warning-free

**Deliverables**:
- Shape and determinism tests pass
- All verification passing

**Verify**: `make check`

---

## Phase 3: Hardening and Close-out

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness, maintainability, standards (strict profile — docs-only phase; auditors skipped as not applicable)

### Step 3.1: End-to-End Evidence and Close-out

**Reference**: [07](07-testing-strategy.md) §End-to-End Tests · [01](01-requirements.md) §Acceptance Criteria
**Objective**: Prove memory safety end to end and close the plan.

- [ ] 3.1.1 Host AddressSanitizer probes over the fixed programs (every DEF-16 form and the DEF-17 nested loops); record the exits and report counts on this line
- [ ] 3.1.2 `make verify` green (all hosted-amiga legs)
- [ ] 3.1.3 Update documentation: `AGENTS.md` test map, roadmap rows for `DEF-16`/`DEF-17` (and confirm `DEF-21` is filed, as recorded in the roadmap during planning), register closure note — `AGENTS.md`, `plans/00-roadmap.md`
- [ ] 3.1.4 Close-out: acceptance criteria checked, plan marked Done

**Deliverables**:
- ASan probes clean
- `make verify` green
- Documentation and roadmap synced

**Verify**: `make check`, then `make verify`

---

## Dependencies

```
Phase 1 (element read ownership)
    ↓
Phase 2 (nested loop step)
    ↓
Phase 3 (hardening and close-out)
```

Phase 2 is independent of Phase 1 in code; the order follows the plan and keeps
one suite growing spec-first. Phase 3 requires both implementations.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check` after every task; `make verify` before completion)
3. ✅ No warnings/errors
4. ✅ No dead code — `for_step_block` and its search are deleted; no unused parameter, function, or module remains
5. ✅ Safety preserved — every DEF-16 reproducer is ASan-clean; nested loops terminate; no new rejection was added
6. ✅ Documentation updated — the `AGENTS.md` test map, the plan state, and the roadmap
7. ✅ Code reviewed (strict profile — independent review of each phase diff)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
