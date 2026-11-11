# Execution Plan: Template Placeholder Residuals

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-11 22:25
> **Progress**: 12/38 tasks (32%)
> **CodeOps Artifact Schema**: 1

## Overview

Deliver issue #27 in three phases: the type-level null/conditional diagnostics,
the runtime and ownership residuals (zero state, discarded templates, for-step
cleanup, aggregate order), and the full acceptance and closeout. The technical
specifications are [03-01](03-01-type-diagnostics.md) and
[03-02](03-02-lowering-and-runtime.md); the expected behavior is
[07-testing-strategy.md](07-testing-strategy.md).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
|-------|-------|-------|
| 1 | Type diagnostics | 12 |
| 2 | Runtime and ownership residuals | 17 |
| 3 | Acceptance and closeout | 9 |

**Total: 38 tasks across 3 phases**

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
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented. Failure to keep the marks
> current means progress is invisible after crashes, context resets, or session handoffs.

---

## Phase 1: Type Diagnostics

> **Phase baseline tree**: `b6f12d8` plus the plan documents and roadmap row;
> the implementation tree is clean apart from those docs.
> **Lenses**: correctness, semantics
> **Reasoning**: medium — two localized type-stage rules with existing frozen precedents

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-type-diagnostics.md) · [07](07-testing-strategy.md) ST-1–ST-10, ST-24–ST-26, ST-28–ST-30 · AR-1, AR-2, AR-17, AR-23
**Objective**: Pin the null-placeholder rejection and the mixed-conditional nullability before any implementation.

- [x] 1.1.1 [spec-author] Create `compiler/tests/template_placeholder_residuals_spec_test.rs` with the harness and ST-1–ST-10, ST-24–ST-26, ST-28–ST-30 — `compiler/tests/template_placeholder_residuals_spec_test.rs` ✅ (completed: 2026-11-11 22:02)
- [x] 1.1.2 Run the suite against `b6f12d8` and record the red evidence for ST-1, ST-2, ST-5, ST-6, ST-9, ST-25, ST-26 — verify: the failure log names each red case ✅ (completed: 2026-11-11 22:02)
- [x] 1.1.3 Confirm ST-3, ST-4, ST-7, ST-8, ST-10, ST-24, ST-28, ST-29, ST-30 pass on `b6f12d8` (behavior guards) — verify: the suite output ✅ (completed: 2026-11-11 22:02)

### Step 1.2: Implementation

**Reference**: [03-01 §1–§2](03-01-type-diagnostics.md) · AR-1, AR-2, AR-11, AR-21, AR-23
**Objective**: Reject a null-shaped placeholder, keep the conditional's nullable flag, and type a null arm at a handle sibling as nullable.

- [x] 1.2.1 Add the null-shaped E10500 branch to `check_template` — `compiler/src/sema/expr.rs` ✅ (completed: 2026-11-11 22:02)
- [x] 1.2.2 Settle the conditional result's nullable flag from non-null-shaped arms and from a null arm at a handle/`Box` result — `compiler/src/types/mod.rs` ✅ (completed: 2026-11-11 22:02)
- [x] 1.2.3 Re-run the new spec suite; all P1 cases green — verify: the suite output ✅ (completed: 2026-11-11 22:02)
- [x] 1.2.4 Re-run the #26 spec/impl suites and `def_zero_null_spec_test`; #26 ST-34 and ST-37 must stay green — verify: `cargo test -p clc --test template_placeholder_integrity_spec_test --test template_placeholder_integrity_impl_test --test def_zero_null_spec_test` ✅ (completed: 2026-11-11 22:02)

### Step 1.3: Impl Tests and Hardening

**Reference**: [07 §3](07-testing-strategy.md) IT-1, IT-2 · AR-1, AR-2, AR-16
**Objective**: Pin the diagnostic surface and review the phase.

- [x] 1.3.1 Create `compiler/tests/template_placeholder_residuals_impl_test.rs` with IT-1 and IT-2 — `compiler/tests/template_placeholder_residuals_impl_test.rs` ✅ (completed: 2026-11-11 22:02)
- [x] 1.3.2 Create `plans/template-placeholder-residuals/05-findings.md` as the findings ledger — `plans/template-placeholder-residuals/05-findings.md` ✅ (completed: 2026-11-11 22:25)
- [x] 1.3.3 Run the phase review (correctness-reviewer, semantics-reviewer) on the phase diff and record the verdicts — verify: the verdicts are recorded in `05-findings.md` ✅ (completed: 2026-11-11 22:25)
- [x] 1.3.4 Fix every critical/major finding and re-review the fix diff once — verify: the re-review verdict ✅ (completed: 2026-11-11 22:25)
- [x] 1.3.5 `make check` green — verify: `make check` ✅ (completed: 2026-11-11 22:25)

**Deliverables**:
- [ ] A null-shaped placeholder is E10500 and writes no C
- [ ] A mixed conditional keeps its nullable flag; unnarrowed arms report E10010
- [ ] The #26 and #25 regression suites still pass

**Verify**: `make check`

---

## Phase 2: Runtime and Ownership Residuals

> **Phase baseline tree**: the Phase 1 head plus the Phase 1 tests; the
> implementation is clean apart from the plan and phase-1 files.
> **Lenses**: correctness, semantics, memory-safety, emission-integrity
> **Reasoning**: high — runtime, cleanup, and emission interact across the shims and the lowering

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-lowering-and-runtime.md) · [07](07-testing-strategy.md) ST-11–ST-23, ST-27 · AR-3–AR-6, AR-18, AR-19, AR-24
**Objective**: Pin the zero-state, discard/step, and aggregate-order behavior.

- [ ] 2.1.1 [spec-author] Add ST-11–ST-15 (zero-state placeholder, direct print, len, clone, text) — `compiler/tests/template_placeholder_residuals_spec_test.rs`
- [ ] 2.1.2 [spec-author] Add ST-16–ST-19 (discarded template, for-step template, owning call, continue edge) — `compiler/tests/template_placeholder_residuals_spec_test.rs`
- [ ] 2.1.3 [spec-author] Add ST-20–ST-23 and ST-27 (aggregate String field, fixed-array field, loop, nested, owning conditional field) — `compiler/tests/template_placeholder_residuals_spec_test.rs`
- [ ] 2.1.4 Run the suite against the Phase 1 head and record the red evidence; label the guards — verify: the failure log names each red case

### Step 2.2: Implementation

**Reference**: [03-02 §1–§3](03-02-lowering-and-runtime.md) · AR-3–AR-6, AR-8, AR-9, AR-12, AR-19
**Objective**: Make the zero state safe, release discarded templates, drain the step, and adopt an owning aggregate at its source position.

- [ ] 2.2.1 Make `cl_string_ptr`, `cl_string_len`, and `cl_string_copy` null-safe in the host shim — `compiler/runtime/shim_host.c`
- [ ] 2.2.2 Mirror the three helpers in the Amiga shim — `compiler/runtime/shim_hosted_amiga.c`
- [ ] 2.2.3 Bind an owned `Template` result in `lower_discarded` — `compiler/src/own/lower.rs`
- [ ] 2.2.4 Add the release floor and drain around each for-step expression in both step loops — `compiler/src/own/loops.rs`
- [ ] 2.2.5 Pin an owning non-fixed `StructLit` through `materialize_adopted_operand` — `compiler/src/own/lower.rs`
- [ ] 2.2.6 Add the no-pre-drop structured `StructLit` store to the `Statement::Adopt` arm — `compiler/src/codegen/function.rs`
- [ ] 2.2.7 Run the new spec suite; every case green — verify: `cargo test -p clc --test template_placeholder_residuals_spec_test`

### Step 2.3: Impl Tests and Hardening

**Reference**: [07 §3–§4](07-testing-strategy.md) IT-3–IT-10 · AR-3–AR-6, AR-22
**Objective**: Pin the emission and prove the sanitizer safety.

- [ ] 2.3.1 Add IT-3–IT-10 to the impl suite — `compiler/tests/template_placeholder_residuals_impl_test.rs`
- [ ] 2.3.2 Run the #26 and #25 suites; confirm IT-9/IT-10 pins are unchanged and update only an intended pin — verify: `cargo test -p clc --test template_placeholder_integrity_spec_test --test template_placeholder_integrity_impl_test --test template_direct_formatting_spec_test --test template_direct_formatting_impl_test`
- [ ] 2.3.3 Run the new spec suite under ASan; no report and no leak — verify: `RDC_ASAN=1 cargo test -p clc --test template_placeholder_residuals_spec_test`
- [ ] 2.3.4 Run the phase review (correctness, semantics, memory-safety, emission-integrity) on the phase diff and record the verdicts — verify: the verdicts are recorded in `05-findings.md`
- [ ] 2.3.5 Fix every critical/major finding and re-review the fix diff once — verify: the re-review verdict
- [ ] 2.3.6 `make check` green — verify: `make check`

**Deliverables**:
- [ ] The zero state formats as an empty String on host and Amiga
- [ ] A discarded template and a for-step template are leak-free
- [ ] An owning aggregate field evaluates before a later placeholder copy, with no double free

**Verify**: `make check`

---

## Phase 3: Acceptance and Closeout

> **Phase baseline tree**: the Phase 2 head; no implementation change remains.
> **Lenses**: correctness, memory-safety, emission-integrity, performance
> **Reasoning**: medium — full-gate execution and evidence recording, no new behavior

### Step 3.1: Full Gate

**Reference**: [07 §4–§5](07-testing-strategy.md) · AR-16, AR-22
**Objective**: Prove the complete acceptance surface and the performance budget.

- [ ] 3.1.1 `make asan` all suites green — verify: the runner's summary
- [ ] 3.1.2 `make ubsan` all suites green — verify: the runner's summary
- [ ] 3.1.3 `make bench-base COMMIT=b6f12d8`, `make bench`, `make bench-compare` green with equal outputs and the 1% guard — verify: the comparison output
- [ ] 3.1.4 `make bench-shape` green — verify: the four shape legs
- [ ] 3.1.5 `make verify` all legs green — verify: `verify: all legs passed`

### Step 3.2: Closeout

**Reference**: [01 §Acceptance](01-requirements.md) · AR-10, AR-16
**Objective**: Record the evidence and sync the project state.

- [ ] 3.2.1 Finalize `plans/template-placeholder-residuals/05-findings.md` with the acceptance metrics, review outcomes, and the gate results — `plans/template-placeholder-residuals/05-findings.md`
- [ ] 3.2.2 Update the existing PLAN-31 row in `plans/00-roadmap.md` to Done and correct the progress counters — `plans/00-roadmap.md`
- [ ] 3.2.3 Close issue #27 with the delivered evidence and the residual note — GitHub issue #27 via `gh issue close --comment`
- [ ] 3.2.4 File the follow-up issue for the explicit `text(null)` ICE (AR-10) — GitHub via `gh issue create`

**Deliverables**:
- [ ] The full gate is green
- [ ] The findings ledger and roadmap record the outcome
- [ ] Issue #27 is closed with evidence
- [ ] The `text(null)` follow-up is recorded

**Verify**: `make verify` and the recorded gate outputs

---

## Dependencies

```
Phase 1 (type diagnostics)
    ↓
Phase 2 (runtime and ownership residuals)
    ↓
Phase 3 (acceptance and closeout)
```

Phase 2's zero-state tests depend on Phase 1 only for the shared spec-suite
file; the lowering and runtime changes are independent but are executed after
Phase 1 so each review sees a focused diff.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ R1–R9 verified by the ST cases
3. ✅ `make check`, `make asan`, `make ubsan`, `make verify`, `make bench-shape`, and `make bench-compare` green
4. ✅ All scored outputs equal; no scored metric regression above 1%
5. ✅ No critical/major review finding open
6. ✅ The findings ledger, roadmap, and issue #27 reflect the evidence
7. ✅ No dead code; the new branches are documented and used
8. ✅ No spec, stdlib, or ABI change
