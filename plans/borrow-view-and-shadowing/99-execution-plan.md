# Execution Plan: Borrow-View Moves and Shadowed Narrowing

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-29 13:05
> **Progress**: 2/16 tasks (13%)
> **CodeOps Artifact Schema**: 1

## Overview

Reject ownership transfers through a `for (borrow e in xs)` element view with
`E10026` (`DEF-18`), and resolve a shadowing declaration's initializer against
the outer narrowed binding so the spurious `E10010` disappears (`DEF-20`).
Each feature phase follows the mandatory specification-first ordering; the
close-out phase runs the full acceptance gate and the documentation/roadmap
updates.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Borrow-view moves | 6 |
| 2 | Shadowed narrowing | 6 |
| 3 | Close-out: acceptance, documentation | 4 |

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
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the
>    Last Updated stamp after EVERY task — never batch updates. Only `[x]`
>    counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]`
>    task is resumed first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append
>    `Blocked: <short reason>` on the same line. The plan lifecycle is `Ready`,
>    `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

---

## Phase 1: Borrow-view moves

> **Phase baseline tree**: 0a6ac189aad54ff74e74ce9ae2ec73fc2b1acb37
> **Expected modification set**: `compiler/tests/borrow_view_and_shadowing_spec_test.rs`, `compiler/tests/borrow_view_and_shadowing_impl_test.rs`, `compiler/src/own/place.rs`, `compiler/src/own/statements.rs`, `compiler/src/own/tests.rs`, `plans/borrow-view-and-shadowing/99-execution-plan.md`
> **Scope mode**: strict
> **Lenses**: correctness, maintainability, standards (strict profile — independent review of the phase diff)

### Step 1.1: Specification Tests

**Reference**: [03-01 §Proposed Changes](03-01-borrow-view-moves.md) · AR-3, AR-4, AR-9, AR-13
**Objective**: Pin the view-move rejection and the guard forms before any implementation.

- [x] 1.1.1 [spec-author] Write specification tests from the 07 ST cases — `compiler/tests/borrow_view_and_shadowing_spec_test.rs` (ST-1 – ST-8, ST-13) ✅ (completed: 2026-09-29 13:04)
- [x] 1.1.2 Run the spec suite and record the red phase — ST-1 – ST-5 accepted pre-fix; ST-6 – ST-8 and ST-13 pass as guards ✅ (completed: 2026-09-29 13:05)

**Deliverables**:
- [x] Spec suite with the ST-1 – ST-8 and ST-13 cases and the shared scratch/run helpers
- [x] Red-phase evidence recorded (suite output)

> **Red phase (recorded 2026-09-29 13:04)**: `cargo test --test borrow_view_and_shadowing_spec_test` → 6 passed / 5 failed. The five rejection cases fail as predicted (ST-1 – ST-5: the compiler accepts every move form); ST-6 – ST-8 and ST-13 pass as guards.

**Verify**: `make -C docker exec CMD="cargo test --test borrow_view_and_shadowing_spec_test"`

### Step 1.2: Implementation

**Reference**: [03-01 §Proposed Changes](03-01-borrow-view-moves.md) · AR-3, AR-7, AR-9
**Objective**: Reject an ownership transfer through a borrow element view at the shared move decision.

- [ ] 1.2.1 Guard `move_place` against a `BindingKind::Local` binding with `Mode::Borrow`, reporting the `E10026` registry row — `compiler/src/own/place.rs`
- [ ] 1.2.2 Run the spec suite and record the green phase — ST-1 – ST-8 and ST-13 pass

**Deliverables**:
- [ ] Every move form through the view reports `E10026`; the guards stay accepted
- [ ] No regressions; no warnings

**Verify**: `make -C docker exec CMD="cargo test --test borrow_view_and_shadowing_spec_test"`

### Step 1.3: Implementation Tests & Hardening

**Reference**: [07 §Test Categories](07-testing-strategy.md) · AR-9, AR-13
**Objective**: Pin the rejection's single report and the unchanged guard paths.

- [ ] 1.3.1 Add implementation tests: single-report at a view move, the borrow-parameter `E10034` path unchanged, and unit coverage in `own::tests` for the view rejection and the copy-form binding — `compiler/tests/borrow_view_and_shadowing_impl_test.rs`, `compiler/src/own/tests.rs`
- [ ] 1.3.2 Full verification — `make check`

**Deliverables**:
- [ ] Implementation tests pass
- [ ] No regressions; no warnings

**Verify**: `make check`

---

## Phase 2: Shadowed narrowing

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness, maintainability, standards (strict profile — independent review of the phase diff)

### Step 2.1: Specification Tests

**Reference**: [03-02 §Proposed Changes](03-02-shadowed-narrowing.md) · AR-5, AR-6, AR-13
**Objective**: Pin the shadowing acceptance and the unchanged null rules.

- [ ] 2.1.1 [spec-author] Add the shadowed-narrowing specification tests to the suite (ST-9 – ST-12, ST-14) — `compiler/tests/borrow_view_and_shadowing_spec_test.rs`
- [ ] 2.1.2 Run the spec suite and record the red phase — ST-9, ST-10, and ST-14 report `E10010` pre-fix; ST-11 and ST-12 pass as guards

**Deliverables**:
- [ ] ST-9 – ST-12 and ST-14 in the spec suite
- [ ] Red-phase evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test borrow_view_and_shadowing_spec_test"`

### Step 2.2: Implementation

**Reference**: [03-02 §Proposed Changes](03-02-shadowed-narrowing.md) · AR-5, AR-6
**Objective**: Run the initializer's null check before the shadowing declaration.

- [ ] 2.2.1 Move `check_value_into` before `declare_local` in `check_let` and `check_const` — `compiler/src/own/statements.rs`
- [ ] 2.2.2 Run the spec suite and record the green phase — ST-9 – ST-12 and ST-14 pass

**Deliverables**:
- [ ] The shadowing `let` and `const` forms compile; genuine null uses keep `E10010`
- [ ] No regressions; no warnings

**Verify**: `make -C docker exec CMD="cargo test --test borrow_view_and_shadowing_spec_test"`

### Step 2.3: Implementation Tests & Hardening

**Reference**: [07 §Test Categories](07-testing-strategy.md) · AR-5, AR-13
**Objective**: Pin the diagnostic shape and the post-declaration usability.

- [ ] 2.3.1 Add implementation tests: a single `E10010` report when one is due, the shadowing binding usable after its declaration, and the `E10012` null rule unchanged — `compiler/tests/borrow_view_and_shadowing_impl_test.rs`
- [ ] 2.3.2 Full verification — `make check`

**Deliverables**:
- [ ] Implementation tests pass
- [ ] No regressions; no warnings

**Verify**: `make check`

---

## Phase 3: Close-out: acceptance, documentation

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness, maintainability, standards (strict profile — docs and evidence phase)

### Step 3.1: Evidence and Close-out

**Reference**: [01 §Acceptance Criteria](01-requirements.md) · AR-8, AR-10, AR-12
**Objective**: Run the full acceptance gate and close the records.

- [ ] 3.1.1 Full acceptance — `make verify` (every hosted-amiga and freestanding leg)
- [ ] 3.1.2 Update documentation: `AGENTS.md` test map (both new suites), roadmap rows for `DEF-18`/`DEF-20` (done + plan link) — `AGENTS.md`, `plans/00-roadmap.md`
- [ ] 3.1.3 Check every acceptance criterion in `01-requirements.md`; set this plan's status Complete and the progress header to 16/16; final `make check`
- [ ] 3.1.4 Phase review evidence and plan summary — commit per the active commit mode

**Deliverables**:
- [ ] `make verify` all legs passed
- [ ] Documentation and roadmap synced; acceptance criteria checked

**Verify**: `make verify`

---

## Dependencies

```
Phase 1 (borrow-view moves)
    ↓
Phase 2 (shadowed narrowing)
    ↓
Phase 3 (close-out)
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check`, `make verify`)
3. ✅ No warnings/errors
4. ✅ No dead code — no unused parameters, functions, classes, or modules
5. ✅ Security reviewed — compiler-internal change with no input, secret, or data path (AR-11)
6. ✅ Documentation updated
7. ✅ Code reviewed (strict-profile phase review)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
