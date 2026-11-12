# Execution Plan: Template Placeholder Residuals II

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-12 04:05
> **Progress**: 16/38 tasks (42%)
> **CodeOps Artifact Schema**: 1

## Overview

Deliver issue #28 in three phases: the static diagnoses (explicit `text`,
bare null operand, null-arm parity), the lowering/runtime residuals
(for-step assignment, for-init effects, aggregate contract, panic guard), and
the full acceptance and closeout. The technical specifications are
[03-01](03-01-static-diagnoses.md) and [03-02](03-02-lowering-and-runtime.md);
the expected behavior is [07-testing-strategy.md](07-testing-strategy.md).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
|-------|-------|-------|
| 1 | Static diagnoses | 16 |
| 2 | Lowering and runtime residuals | 14 |
| 3 | Acceptance and closeout | 8 |

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

## Phase 1: Static Diagnoses

> **Phase baseline tree**: `bb3956c` plus PLAN-33 (`text-arity`, executed first) and the plan documents and roadmap row.
> **Lenses**: correctness, semantics
> **Reasoning**: high — formattability, null-arm settlement, and generic parity
> share the type stage and the instance audit

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-static-diagnoses.md) · [07](07-testing-strategy.md) ST-1–ST-17, ST-26–ST-31 · AR-1–AR-9, AR-18, AR-20, AR-21
**Objective**: Pin `text` formattability, the bare null operand, and the null-arm parity before any implementation.

- [x] 1.1.1 [spec-author] Create `compiler/tests/template_placeholder_residuals_ii_spec_test.rs` with the harness and ST-1–ST-17, ST-26–ST-31, ST-33–ST-38 — `compiler/tests/template_placeholder_residuals_ii_spec_test.rs` ✅ (completed: 2026-11-12 03:17)
- [x] 1.1.2 Run the suite against `bb3956c` and record the red evidence for ST-1, ST-2, ST-3, ST-5, ST-7, ST-9, ST-11, ST-12, ST-13, ST-14, ST-15, ST-28, ST-33, ST-36 — verify: the failure log names each red case ✅ (completed: 2026-11-12 03:17)
- [x] 1.1.3 Confirm ST-4, ST-6, ST-8, ST-10, ST-16, ST-17, ST-26, ST-27, ST-29, ST-30, ST-31, ST-34 pass on `bb3956c` (behavior guards) — verify: the suite output ✅ (completed: 2026-11-12 03:17)

### Step 1.2: Implementation

**Reference**: [03-01 §1–§3](03-01-static-diagnoses.md) · AR-1–AR-9, AR-20, AR-21
**Objective**: Reject a non-formattable `text` argument (including an opaque projection), a bare null operand, and a scalar null arm; make the generic null arm match the concrete twin.

- [x] 1.2.1 Add the concrete null/non-formattable/opaque-projection E10500 branch to the compiler-known `text` call — `compiler/src/sema/call.rs` ✅ (completed: 2026-11-12 03:17)
- [x] 1.2.2 Apply the nullable-use E10010 rule to a formattable `text` argument, and reuse the existing deferred placeholder duty for a generic argument — `compiler/src/own/expr.rs` ✅ (completed: 2026-11-12 03:17)
- [x] 1.2.3 Remove the literal-null exclusion from the binary null-operand rule — `compiler/src/sema/ops.rs` ✅ (completed: 2026-11-12 03:17)
- [x] 1.2.4 Add the shared `settle_null_arm` and `accepts_bare_null` helpers (with unit tests) and the concrete no-expected-type E10012 report — `compiler/src/types/mod.rs`, `compiler/src/sema/ops.rs` ✅ (completed: 2026-11-12 03:17)
- [x] 1.2.5 Apply the settled null-arm rule in `null_conditional_report` and the placeholder requirement audit; the placeholder duty owns E10010 while the instance conversion duty owns E10012 — `compiler/src/own/instance_check.rs` ✅ (completed: 2026-11-12 03:17)
- [x] 1.2.6 Skip a binary argument conditional whose site already reported, so `(c ? 1 : null) + 1` reports E10012 once — `compiler/src/sema/ops.rs` ✅ (completed: 2026-11-12 03:17)
- [x] 1.2.7 Re-run the new spec suite; all Phase 1 cases green — verify: the suite output ✅ (completed: 2026-11-12 03:17)
- [x] 1.2.8 Re-run the #26/#27 suites, `def_zero_null_spec_test`, and `def_semantic_parity_integrity_spec_test`; #26 ST-37 stays green — verify: `cargo test -p clc --test template_placeholder_integrity_spec_test --test template_placeholder_integrity_impl_test --test template_placeholder_residuals_spec_test --test template_placeholder_residuals_impl_test --test def_zero_null_spec_test --test def_semantic_parity_integrity_spec_test` ✅ (completed: 2026-11-12 03:17)

### Step 1.3: Impl Tests and Hardening

**Reference**: [07 §3](07-testing-strategy.md) IT-1–IT-6, IT-13, IT-14 · AR-1–AR-9, AR-16, AR-20, AR-21
**Objective**: Pin the diagnostic surface and review the phase.

- [x] 1.3.1 Create `compiler/tests/template_placeholder_residuals_ii_impl_test.rs` with IT-1–IT-6, IT-13–IT-15 — `compiler/tests/template_placeholder_residuals_ii_impl_test.rs` ✅ (completed: 2026-11-12 03:17)
- [x] 1.3.2 Create `plans/template-placeholder-residuals-ii/05-findings.md` as the findings ledger — `plans/template-placeholder-residuals-ii/05-findings.md` ✅ (completed: 2026-11-12 03:17)
- [x] 1.3.3 Run the phase review (correctness-reviewer, semantics-reviewer, emission-integrity-auditor) on the phase diff and record the verdicts — verify: the verdicts are recorded in `05-findings.md` ✅ (completed: 2026-11-12 04:05)
- [x] 1.3.4 Fix every critical/major finding and re-review the fix diff once — verify: the re-review verdict ✅ (completed: 2026-11-12 04:05)
- [x] 1.3.5 `make check` green — verify: `make check` ✅ (completed: 2026-11-12 04:05)

**Deliverables**:
- [ ] `text(null)` and a non-formattable `text` argument are E10500
- [ ] A bare null operand outside `==`/`!=` is E10012
- [ ] A generic null arm matches the concrete twin; a scalar null arm is E10012

**Verify**: `make check`

---

## Phase 2: Lowering and Runtime Residuals

> **Phase baseline tree**: the Phase 1 head plus the Phase 1 tests.
> **Lenses**: correctness, semantics, memory-safety, emission-integrity
> **Reasoning**: high — loop shapes, release cleanup, and the panic runtime interact

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-lowering-and-runtime.md) · [07](07-testing-strategy.md) ST-18–ST-25, ST-32 · AR-10–AR-13
**Objective**: Pin the for-step, for-init, aggregate-order, and panic behavior.

- [ ] 2.1.1 [spec-author] Add ST-18–ST-21 (for-step owning assignment, continue edge, for-init template, for-init owning call) — `compiler/tests/template_placeholder_residuals_ii_spec_test.rs`
- [ ] 2.1.2 [spec-author] Add ST-22–ST-25 and ST-32 (same/differing aggregate order, panic null view, panic zero-state String, increment init) — `compiler/tests/template_placeholder_residuals_ii_spec_test.rs`
- [ ] 2.1.3 Run the suite against the Phase 1 head and record the red evidence for ST-18, ST-19, ST-20, ST-21, ST-24, ST-25, ST-32; ST-22/ST-23 are guards — verify: the failure log names each red case

### Step 2.2: Implementation

**Reference**: [03-02 §1–§4](03-02-lowering-and-runtime.md) · AR-10–AR-13
**Objective**: Release the old for-step value, run for-init effects, document and pin the aggregate contract, and guard the panic write.

- [ ] 2.2.1 Reject a for-step `Assign` whose target may own, forcing the flat loop form — `compiler/src/own/loops.rs`
- [ ] 2.2.2 Lower each for-init expression through `check_expr_statement` with a release floor and drain — `compiler/src/own/loops.rs`
- [ ] 2.2.3 Guard the zero-length message `fwrite` in both panic shims — `compiler/runtime/shim_host.c`, `compiler/runtime/shim_hosted_amiga.c`
- [ ] 2.2.4 Document the supported-toolchain initializer-order contract in the two codegen comments — `compiler/src/codegen/guards.rs`, `compiler/src/codegen/function.rs`
- [ ] 2.2.5 Run the new spec suite; every case green — verify: `cargo test -p clc --test template_placeholder_residuals_ii_spec_test`
- [ ] 2.2.6 Re-run the #26/#27/#25 suites and the nullability suites — verify: the suite output

### Step 2.3: Impl Tests and Hardening

**Reference**: [07 §3–§4](07-testing-strategy.md) IT-7–IT-12 · AR-10, AR-12, AR-13
**Objective**: Pin the loop shapes and panic guard, and prove the sanitizer safety.

- [ ] 2.3.1 Add IT-7–IT-12 to the impl suite — `compiler/tests/template_placeholder_residuals_ii_impl_test.rs`
- [ ] 2.3.2 Run `make asan` and `make ubsan`; the new suite is discovered and reports no ASan/UBSan error and no leak — verify: the runner summaries
- [ ] 2.3.3 Run the phase review (correctness, semantics, memory-safety, emission-integrity) on the phase diff and record the verdicts — verify: the verdicts are recorded in `05-findings.md`
- [ ] 2.3.4 Fix every critical/major finding and re-review the fix diff once — verify: the re-review verdict
- [ ] 2.3.5 `make check` green — verify: `make check`

**Deliverables**:
- [ ] A for-step owning assignment is leak-free on the fall-through and `continue` edges
- [ ] A for-init template or call runs once and releases its result
- [ ] The aggregate order contract is documented and pinned
- [ ] A zero-length panic is clean on host; the Amiga body is parity-pinned and compiled

**Verify**: `make check`

---

## Phase 3: Acceptance and Closeout

> **Phase baseline tree**: the Phase 2 head; no implementation change remains.
> **Lenses**: correctness, memory-safety, emission-integrity, performance
> **Reasoning**: medium — full-gate execution and evidence recording, no new behavior

### Step 3.1: Full Gate

**Reference**: [07 §4–§5](07-testing-strategy.md) · AR-17
**Objective**: Prove the complete acceptance surface and the performance budget.

- [ ] 3.1.1 `make asan` all suites green — verify: the runner's summary
- [ ] 3.1.2 `make ubsan` all suites green — verify: the runner's summary
- [ ] 3.1.3 `make bench-base COMMIT=bb3956c`, `make bench`, `make bench-compare` green with equal outputs and the 1% guard — verify: the comparison output
- [ ] 3.1.4 `make bench-shape` green — verify: the four shape legs
- [ ] 3.1.5 `make verify` all legs green — verify: `verify: all legs passed`

### Step 3.2: Closeout

**Reference**: [01 §Acceptance](01-requirements.md) · AR-14, AR-17
**Objective**: Record the evidence and sync the project state.

- [ ] 3.2.1 Finalize `plans/template-placeholder-residuals-ii/05-findings.md` (PLAN-33, the `text` arity mini-plan, executes first because it edits the same `text` branch) with the baseline/head metrics, red evidence, review outcomes, and the gate results — `plans/template-placeholder-residuals-ii/05-findings.md`
- [ ] 3.2.2 Update the existing PLAN-32 row in `plans/00-roadmap.md` to Done and correct the progress counters — `plans/00-roadmap.md`
- [ ] 3.2.3 Close issue #28 with the delivered evidence and the #24 note — GitHub issue #28 via `gh issue close --comment`

**Deliverables**:
- [ ] The full gate is green
- [ ] The findings ledger and roadmap record the outcome
- [ ] Issue #28 is closed with evidence

**Verify**: `make verify` and the recorded gate outputs

---

## Dependencies

```
Phase 1 (static diagnoses)
    ↓
Phase 2 (lowering and runtime)
    ↓
Phase 3 (acceptance and closeout)
```

PLAN-33 (`text-arity`) executes first because it edits the same `text` branch.
Phase 2 is behaviorally independent of Phase 1, but it follows Phase 1 so each
review sees a focused diff and the full gate runs on a tree whose static
diagnoses are already settled.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ R1–R11 verified by the ST cases
3. ✅ `make check`, `make asan`, `make ubsan`, `make verify`, `make bench-shape`, and `make bench-compare` green
4. ✅ All scored outputs equal; no scored metric regression above 1%
5. ✅ No critical/major review finding open
6. ✅ The findings ledger, roadmap, and issue #28 reflect the evidence
7. ✅ No dead code; the new branches are documented and used
8. ✅ No spec, stdlib, or ABI change
