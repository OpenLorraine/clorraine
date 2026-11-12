# Execution Plan: Template Placeholder Residuals III

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-12 10:50
> **Progress**: 35/35 tasks (100%)
> **CodeOps Artifact Schema**: 1

## Overview

Deliver issue #29 in three phases: the static judgments (alias/qualified
`text`, generic condition context, both-null decision, deferred null
comparison), the lowering/runtime residuals (null-safe cstring length,
fixed-array for step), and the full acceptance and closeout. The technical
specification is [03-implementation.md](03-implementation.md); the expected
behavior is [07-testing-strategy.md](07-testing-strategy.md).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
|-------|-------|-------|
| 1 | Static diagnoses and text spellings | 16 |
| 2 | Lowering and runtime residuals | 11 |
| 3 | Acceptance and closeout | 8 |

**Total: 35 tasks across 3 phases**

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

## Phase 1: Static Diagnoses and Text Spellings

> **Phase baseline tree**: `541ba27` plus the plan documents and roadmap row.
> **Lenses**: correctness, semantics, memory-safety, emission-integrity
> **Reasoning**: high — name resolution, condition parity, and deferred duties
> share the type stage, the ownership stage, and the instance audit

### Step 1.1: Specification Tests

**Reference**: [03 §1–§4](03-implementation.md) · [07](07-testing-strategy.md) ST-1–ST-40, ST-46–ST-49 · AR-1–AR-10
**Objective**: Pin the text spellings, the generic condition rule, the both-null decision, and the deferred null comparison before any implementation.

- [x] 1.1.1 [spec-author] Create `compiler/tests/template_placeholder_residuals_iii_spec_test.rs` with the harness (including `run_pair`) and ST-1–ST-40, ST-46–ST-49 — `compiler/tests/template_placeholder_residuals_iii_spec_test.rs`
- [x] 1.1.2 Run the suite against `541ba27` and record the red evidence for ST-1–ST-9, ST-12, ST-13, ST-16, ST-17, ST-19–ST-21, ST-33, ST-34, ST-46, ST-47, and ST-49 — verify: the failure log names each red case
- [x] 1.1.3 Confirm the guards pass on `541ba27` (ST-10, ST-11, ST-14, ST-15, ST-18, ST-22–ST-32, ST-35–ST-40, ST-48) — verify: the suite output

### Step 1.2: Implementation

**Reference**: [03 §1–§4](03-implementation.md) · AR-1–AR-10
**Objective**: Resolve the text spellings, carry the condition context, document the both-null decision, and defer the null comparison.

- [x] 1.2.1 Factor the compiler-known `text` branch into `check_text_call`; add `is_text_intrinsic_name` / `is_std_fmt_text_path` with the real-declaration gates — `compiler/src/sema/call.rs`
- [x] 1.2.2 Extend `is_builtin_text_call` for the alias/qualified spelling and canonicalize the lowered callee name to `$text` — `compiler/src/own/expr.rs`, `compiler/src/own/call.rs`
- [x] 1.2.3 Add the `condition` flag to `ConditionalArms`/`OpaqueConditional` and copy it in `check_module` — `compiler/src/sema/mod.rs`, `compiler/src/sema/ops.rs`
- [x] 1.2.4 Report E10218 for a non-bool generic condition settlement and skip its null-arm report — `compiler/src/own/instance_check.rs`
- [x] 1.2.5 Document the both-null decision in the `settle_null_arm` rustdoc; no settlement variant and no behavior change — `compiler/src/types/mod.rs`
- [x] 1.2.6 Add `OpaqueNullComparison`, drain it in the per-module loop, and audit it per instance — `compiler/src/sema/mod.rs`, `compiler/src/sema/ops.rs`, `compiler/src/own/instance_check.rs`
- [x] 1.2.7 Re-run the new spec suite; all Phase 1 cases green — verify: the suite output
- [x] 1.2.8 Re-run the #24/#28/#26/#27/#25 suites, `def_zero_null_spec_test`, `def_semantic_parity_integrity_spec_test`, and `checker_conformance_*` — verify: the suite output

### Step 1.3: Impl Tests and Hardening

**Reference**: [07 §3](07-testing-strategy.md) IT-1–IT-6, IT-10–IT-13 · AR-15
**Objective**: Pin the diagnostic surface and review the phase.

- [x] 1.3.1 Create `compiler/tests/template_placeholder_residuals_iii_impl_test.rs` with IT-1–IT-6 and IT-10–IT-13 — `compiler/tests/template_placeholder_residuals_iii_impl_test.rs`
- [x] 1.3.2 Create `plans/template-placeholder-residuals-iii/05-findings.md` as the findings ledger — `plans/template-placeholder-residuals-iii/05-findings.md`
- [x] 1.3.3 Run the phase review (correctness-reviewer, semantics-reviewer, memory-safety-auditor, emission-integrity-auditor) on the phase diff and record the verdicts — verify: the verdicts are recorded in `05-findings.md`
- [x] 1.3.4 Fix every critical/major finding and re-review the fix diff once — verify: the re-review verdict
- [x] 1.3.5 `make check` green — verify: `make check`

**Deliverables**:
- [x] Aliased and qualified `text` apply every intrinsic judgment
- [x] A user `std.fmt.text` declaration and a value head win over the intrinsic
- [x] A generic condition reports E10218-only for a non-bool settlement
- [x] The both-null conditional is documented and pinned as the null value
- [x] A null comparison against a non-null-capable instance is E10012

**Verify**: `make check`

---

## Phase 2: Lowering and Runtime Residuals

> **Phase baseline tree**: the Phase 1 head plus the Phase 1 tests.
> **Lenses**: correctness, semantics, memory-safety, emission-integrity
> **Reasoning**: medium — three small runtime guards and one loop-shape predicate

### Step 2.1: Specification Tests

**Reference**: [03 §5–§6](03-implementation.md) · [07](07-testing-strategy.md) ST-41–ST-45 · AR-11, AR-12
**Objective**: Pin the null cstring and the fixed-array step behavior.

- [x] 2.1.1 [spec-author] Add ST-41–ST-45 (cstring panic, non-empty cstring panic, step array call/member/literal) — `compiler/tests/template_placeholder_residuals_iii_spec_test.rs`
- [x] 2.1.2 Run the suite against the Phase 1 head and record the red evidence for ST-41, ST-43, ST-44; ST-42/ST-45 are guards — verify: the failure log names each red case

### Step 2.2: Implementation

**Reference**: [03 §5–§6](03-implementation.md) · AR-11, AR-12
**Objective**: Make the cstring length null-safe and force the flat array step.

- [x] 2.2.1 Guard `cl_cstring_len` against NULL in the host, Amiga, and freestanding shims — `compiler/runtime/shim_host.c`, `shim_hosted_amiga.c`, `shim_freestanding.c`
- [x] 2.2.2 Force the flat loop for a for-step `Assign` whose target is a fixed array — `compiler/src/own/loops.rs`
- [x] 2.2.3 Run the new spec suite; every case green — verify: the suite output
- [x] 2.2.4 Re-run the #24/#28/#26/#27/#25 suites and the nullability suites — verify: the suite output

### Step 2.3: Impl Tests and Hardening

**Reference**: [07 §3–§4](07-testing-strategy.md) IT-7–IT-9 · AR-11, AR-12
**Objective**: Pin the shim guard and the step shape, and prove sanitizer safety.

- [x] 2.3.1 Add IT-7–IT-9 to the impl suite — `compiler/tests/template_placeholder_residuals_iii_impl_test.rs`
- [x] 2.3.2 Run `make asan` and `make ubsan`; both new suites are discovered and clean — verify: the runner summaries
- [x] 2.3.3 Run the phase review (correctness, semantics, memory-safety, emission-integrity) on the phase diff and record the verdicts — verify: the verdicts are recorded in `05-findings.md`
- [x] 2.3.4 Fix every critical/major finding and re-review the fix diff once — verify: the re-review verdict
- [x] 2.3.5 `make check` green — verify: `make check`

**Deliverables**:
- [x] A null cstring panics cleanly
- [x] A fixed-array for-step assignment uses the flat store
- [x] Non-owning and array-literal step shapes stay unchanged

**Verify**: `make check`

---

## Phase 3: Acceptance and Closeout

> **Phase baseline tree**: the Phase 2 head; no implementation change remains.
> **Lenses**: correctness, memory-safety, emission-integrity, performance
> **Reasoning**: medium — full-gate execution and evidence recording, no new behavior

### Step 3.1: Full Gate

**Reference**: [07 §4–§5](07-testing-strategy.md) · AR-14
**Objective**: Prove the complete acceptance surface and the performance budget.

- [x] 3.1.1 `make asan` all suites green — verify: the runner's summary
- [x] 3.1.2 `make ubsan` all suites green — verify: the runner's summary
- [x] 3.1.3 `make bench-base COMMIT=541ba27`, `make bench`, `make bench-compare` green with equal outputs and the 1% guard — verify: the comparison output
- [x] 3.1.4 `make bench-shape` green — verify: the four shape legs
- [x] 3.1.5 `make verify` all legs green — verify: `verify: all legs passed`

### Step 3.2: Closeout

**Reference**: [01 §Acceptance](01-requirements.md) · AR-13, AR-14
**Objective**: Record the evidence and sync the project state.

- [x] 3.2.1 Finalize `plans/template-placeholder-residuals-iii/05-findings.md` with the baseline/head metrics, red evidence, review outcomes, and the gate results — `plans/template-placeholder-residuals-iii/05-findings.md`
- [x] 3.2.2 Update the PLAN-34 row in `plans/00-roadmap.md` to Done and correct the progress counters — `plans/00-roadmap.md`
- [x] 3.2.3 Close issue #29 with the delivered evidence — GitHub issue #29 via `gh issue close --comment`

**Deliverables**:
- [x] The full gate is green
- [x] The findings ledger and roadmap record the outcome
- [x] Issue #29 is closed with evidence

**Verify**: `make verify` and the recorded gate outputs

---

## Dependencies

```
Phase 1 (static)
    ↓
Phase 2 (runtime/lowering)
    ↓
Phase 3 (acceptance)
```

Phase 2 is behaviorally independent of Phase 1, but it follows Phase 1 so each
review sees a focused diff and the full gate runs on a settled tree.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ R1–R10 verified by the ST/IT cases
3. ✅ `make check`, `make asan`, `make ubsan`, `make verify`, `make bench-shape`, and `make bench-compare` green
4. ✅ All scored outputs equal; no scored metric regression above 1%
5. ✅ No critical/major review finding open
6. ✅ The findings ledger, roadmap, and issue #29 reflect the evidence
7. ✅ No dead code; the new branches are documented and used
8. ✅ No spec, stdlib, or ABI change
