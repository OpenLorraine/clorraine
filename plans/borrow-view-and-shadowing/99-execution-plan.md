# Execution Plan: Borrow-View Moves and Shadowed Narrowing

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-29 13:19
> **Progress**: 15/16 tasks (94%)
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
- [x] 1.1.2 Run the spec suite and record the red phase — ST-1 – ST-5 accepted pre-fix; ST-6 – ST-8 and ST-13 pass as guards ✅ (completed: 2026-09-29 13:04)

**Deliverables**:
- [x] Spec suite with the ST-1 – ST-8 and ST-13 cases and the shared scratch/run helpers
- [x] Red-phase evidence recorded (suite output)

> **Red phase (recorded 2026-09-29 13:04)**: `cargo test --test borrow_view_and_shadowing_spec_test` → 6 passed / 5 failed. The five rejection cases fail as predicted (ST-1 – ST-5: the compiler accepts every move form); ST-6 – ST-8 and ST-13 pass as guards.

**Verify**: `make -C docker exec CMD="cargo test --test borrow_view_and_shadowing_spec_test"`

### Step 1.2: Implementation

**Reference**: [03-01 §Proposed Changes](03-01-borrow-view-moves.md) · AR-3, AR-7, AR-9
**Objective**: Reject an ownership transfer through a borrow element view at the shared move decision.

- [x] 1.2.1 Guard `move_place` against a `BindingKind::Local` binding with `Mode::Borrow`, reporting the `E10026` registry row — `compiler/src/own/place.rs` ✅ (completed: 2026-09-29 13:04)
- [x] 1.2.2 Run the spec suite and record the green phase — ST-1 – ST-8 and ST-13 pass ✅ (completed: 2026-09-29 13:04)

**Deliverables**:
- [x] Every move form through the view reports `E10026`; the guards stay accepted
- [x] No regressions; no warnings

> **Green phase (recorded 2026-09-29 13:04)**: `cargo test --test borrow_view_and_shadowing_spec_test` → 11 passed / 0 failed. Every rejection case reports `E10026` and every guard case still passes.

**Verify**: `make -C docker exec CMD="cargo test --test borrow_view_and_shadowing_spec_test"`

### Step 1.3: Implementation Tests & Hardening

**Reference**: [07 §Test Categories](07-testing-strategy.md) · AR-9, AR-13
**Objective**: Pin the rejection's single report and the unchanged guard paths.

- [x] 1.3.1 Add implementation tests: single-report at a view move, the borrow-parameter `E10034` path unchanged, and unit coverage in `own::tests` for the view rejection and the copy-form binding — `compiler/tests/borrow_view_and_shadowing_impl_test.rs`, `compiler/src/own/tests.rs` ✅ (completed: 2026-09-29 13:05)
- [x] 1.3.2 Full verification — `make check` ✅ (completed: 2026-09-29 13:05)

**Deliverables**:
- [x] Implementation tests pass
- [x] No regressions; no warnings

> **Full verification (recorded 2026-09-29 13:05)**: `make check` → spec check plus the full Rust workspace suite, all green (351 lib tests, every spec/impl suite passing).

> **Phase 1 review (recorded 2026-09-29 13:08)**: independent correctness/maintainability/standards review of the phase diff (baseline tree `0a6ac18`). No critical or major findings; no correctness findings in the guard or tests. One minor bookkeeping finding RV-001 (the Step 1.2 "no regressions" deliverable box was left unchecked after `make check` passed) — resolved by checking it. The reviewer independently re-ran both new suites, the `own::tests` unit tests, and `make check`, and probed `target/debug/clc` directly (single report per transfer, `E10034` intact, writes/lends accepted).

**Verify**: `make check`

---

## Phase 2: Shadowed narrowing

> **Phase baseline tree**: 80779131e34df9e3845b1c064a0a5ce3cc5ac90f
> **Expected modification set**: `compiler/tests/borrow_view_and_shadowing_spec_test.rs`, `compiler/tests/borrow_view_and_shadowing_impl_test.rs`, `compiler/src/own/statements.rs`, `compiler/src/own/tests.rs`, `plans/borrow-view-and-shadowing/99-execution-plan.md`
> **Scope mode**: strict
> **Lenses**: correctness, maintainability, standards (strict profile — independent review of the phase diff)

### Step 2.1: Specification Tests

**Reference**: [03-02 §Proposed Changes](03-02-shadowed-narrowing.md) · AR-5, AR-6, AR-13
**Objective**: Pin the shadowing acceptance and the unchanged null rules.

- [x] 2.1.1 [spec-author] Add the shadowed-narrowing specification tests to the suite (ST-9 – ST-12, ST-14) — `compiler/tests/borrow_view_and_shadowing_spec_test.rs` ✅ (completed: 2026-09-29 13:12)
- [x] 2.1.2 Run the spec suite and record the red phase — ST-9, ST-10, and ST-14 report `E10010` pre-fix; ST-11 and ST-12 pass as guards ✅ (completed: 2026-09-29 13:12)

**Deliverables**:
- [x] ST-9 – ST-12 and ST-14 in the spec suite
- [x] Red-phase evidence recorded

> **Red phase (recorded 2026-09-29 13:12)**: `cargo test --test borrow_view_and_shadowing_spec_test` → 13 passed / 3 failed. The three shadowing cases fail as predicted (ST-9, ST-10, ST-14: spurious `E10010` on the initializer); ST-11 and ST-12 pass as guards.

**Verify**: `make -C docker exec CMD="cargo test --test borrow_view_and_shadowing_spec_test"`

### Step 2.2: Implementation

**Reference**: [03-02 §Proposed Changes](03-02-shadowed-narrowing.md) · AR-5, AR-6
**Objective**: Run the initializer's null check before the shadowing declaration.

- [x] 2.2.1 Move `check_value_into` before `declare_local` in `check_let` and `check_const` — `compiler/src/own/statements.rs` ✅ (completed: 2026-09-29 13:12)
- [x] 2.2.2 Run the spec suite and record the green phase — ST-9 – ST-12 and ST-14 pass ✅ (completed: 2026-09-29 13:12)

**Deliverables**:
- [x] The shadowing `let` and `const` forms compile; genuine null uses keep `E10010`
- [x] No regressions; no warnings

> **Green phase (recorded 2026-09-29 13:12)**: `cargo test --test borrow_view_and_shadowing_spec_test` → 16 passed / 0 failed. The `let` and `const` shadowing forms now compile; the un-narrowed use still reports `E10010`.

**Verify**: `make -C docker exec CMD="cargo test --test borrow_view_and_shadowing_spec_test"`

### Step 2.3: Implementation Tests & Hardening

**Reference**: [07 §Test Categories](07-testing-strategy.md) · AR-5, AR-13
**Objective**: Pin the diagnostic shape and the post-declaration usability.

- [x] 2.3.1 Add implementation tests: a single `E10010` report when one is due, the shadowing binding usable after its declaration, and the `E10012` null rule unchanged — `compiler/tests/borrow_view_and_shadowing_impl_test.rs` ✅ (completed: 2026-09-29 13:13)
- [x] 2.3.2 Full verification — `make check` ✅ (completed: 2026-09-29 13:13)

**Deliverables**:
- [x] Implementation tests pass
- [x] No regressions; no warnings

> **Full verification (recorded 2026-09-29 13:13)**: `make check` → spec check plus the full Rust workspace suite, all green (30 suites ok, no failures).

> **Phase 2 review (recorded 2026-09-29 13:17)**: independent correctness/maintainability/standards review of the phase diff (baseline tree `8077913`). No critical or major findings; the reviewer confirmed the reorder changes only the name the initializer resolves to, that the `if let` restructuring is equivalent, and that diagnostics order and IR are unchanged (probed boundary cases: un-narrowed shadowing initializer, field-path narrowing, nested shadowing, missing outer binding). Two minor findings, both resolved: RV-001 (the changed path lacked a guard case — added `bvsimp7`, a shadowing initializer reading an un-narrowed outer binding, expecting exactly one `E10010`; plus the `bvsimp4` rationale corrected) and RV-002 (the Step 2.2 "no regressions" box was unchecked after `make check` passed — checked).

**Verify**: `make check`

---

## Phase 3: Close-out: acceptance, documentation

> **Phase baseline tree**: deb6baa4138093c3f2499d92a383b36d38875b4c
> **Expected modification set**: `AGENTS.md`, `plans/00-roadmap.md`, `plans/borrow-view-and-shadowing/01-requirements.md`, `plans/borrow-view-and-shadowing/99-execution-plan.md`
> **Scope mode**: strict
> **Lenses**: correctness, maintainability, standards (strict profile — docs and evidence phase)

### Step 3.1: Evidence and Close-out

**Reference**: [01 §Acceptance Criteria](01-requirements.md) · AR-8, AR-10, AR-12
**Objective**: Run the full acceptance gate and close the records.

- [x] 3.1.1 Full acceptance — `make verify` (every hosted-amiga and freestanding leg) ✅ (completed: 2026-09-29 13:19)
- [x] 3.1.2 Update documentation: `AGENTS.md` test map (both new suites), roadmap rows for `DEF-18`/`DEF-20` (done + plan link) — `AGENTS.md`, `plans/00-roadmap.md` ✅ (completed: 2026-09-29 13:19)
- [x] 3.1.3 Check every acceptance criterion in `01-requirements.md`; set this plan's status Complete and the progress header to 16/16; final `make check` ✅ (completed: 2026-09-29 13:19)
- [ ] 3.1.4 Phase review evidence and plan summary — commit per the active commit mode

**Deliverables**:
- [x] `make verify` all legs passed
- [x] Documentation and roadmap synced; acceptance criteria checked

> **Acceptance check (recorded 2026-09-29 13:19)**: every requirement R1–R4 traces to passing spec cases (R1: ST-1 – ST-5; R2: ST-9, ST-14; R3: ST-10; R4: ST-6 – ST-8, ST-11 – ST-13); `make check` green with no warnings and `make verify` all legs passed; the roadmap rows are Done with the plan link; `AGENTS.md` carries both suites; final `make check` green (30 suites ok).

> **Full acceptance (recorded 2026-09-29 13:19)**: `make verify` → `check` plus every hosted-amiga cross-toolchain leg, the vbcc build-only leg, the FS-UAE programs, and the six runtime-fault programs; `verify: all legs passed`.

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
