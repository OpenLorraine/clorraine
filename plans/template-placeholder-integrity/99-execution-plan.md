# Execution Plan: Template Placeholder Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-11 17:25
> **Progress**: 28/37 tasks (76%)
> **CodeOps Artifact Schema**: 1

## Overview

Deliver issue #26 in three phases: the nullable-placeholder rule, the ordered
placeholder evaluation (function body, guarded positions, generic
instantiations, module initializers), and the full acceptance and closeout.
The technical specification is [03-02](03-02-ordered-evaluation.md); the
nullable rule is [03-01](03-01-nullable-placeholders.md); the expected
behavior is [07-testing-strategy.md](07-testing-strategy.md).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
|-------|-------|-------|
| 1 | Nullable and generic placeholder duties | 12 |
| 2 | Ordered placeholder evaluation | 16 |
| 3 | Acceptance and closeout | 9 |

**Total: 37 tasks across 3 phases**

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

## Phase 1: Nullable and Generic Placeholder Duties

> **Phase baseline tree**: `8b4bff2` plus the plan documents and roadmap row;
> the implementation tree is clean apart from those docs.
> **Lenses**: correctness, semantics, memory-safety
> **Reasoning**: medium — a localized ownership-check addition with a clear existing precedent

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-nullable-placeholders.md) · [07](07-testing-strategy.md) ST-1–ST-5, ST-22–ST-24, ST-26, ST-27, ST-32–ST-34 · AR-1, AR-2, AR-7
**Objective**: Write the nullable-placeholder oracle before any implementation.

- [x] 1.1.1 [spec-author] Create `compiler/tests/template_placeholder_integrity_spec_test.rs` with the harness and ST-1, ST-2, ST-3, ST-4, ST-5, ST-22, ST-23, ST-24, ST-26, ST-27, ST-32, ST-33, ST-34, ST-36, ST-37 — `compiler/tests/template_placeholder_integrity_spec_test.rs` ✅ (completed: 2026-11-11 15:21)
- [x] 1.1.2 Run the suite against the phase-start `clc` and record the red evidence for ST-1, ST-2, ST-4, ST-23, ST-26, ST-32, ST-33 (missing E10010/E10500; base crash, accepted program, or ICE) — verify: the failure log names the red cases ✅ (completed: 2026-11-11 15:21)
- [x] 1.1.3 Confirm ST-3, ST-5, ST-22, ST-24, ST-27, ST-34, ST-36 pass on the phase-start compiler (behavior guards) — verify: the suite output ✅ (completed: 2026-11-11 15:21)

### Step 1.2: Implementation

**Reference**: [03-01 §2, §3a](03-01-nullable-placeholders.md) · AR-1, AR-2, AR-7, AR-20
**Objective**: Apply the existing nullable/formattability rules in the function-body template walk and the per-instance generic duty.

- [x] 1.2.1 Add the `check_value_into(inner, &str, inner.span)` call to the `ExprKind::Template` arm in `eval`, and reject an opaque (`is_unknown` and not null-shaped) placeholder with E10500 in `check_template` — `compiler/src/own/expr.rs`, `compiler/src/sema/expr.rs` ✅ (completed: 2026-11-11 15:21)
- [x] 1.2.2 Add `RequiresTemplatePlaceholder` (formattability + nullability + conditional arms), its field/dedupe set/transfer in `own.rs`, its `ir.rs` re-export, the name/arm helper and record in `own/expr.rs`, the sema `formattable` exposure (`sema/mod.rs`), and the per-site check in `instance_check.rs` — `compiler/src/ir/mono.rs`, `compiler/src/ir.rs`, `compiler/src/own.rs`, `compiler/src/own/expr.rs`, `compiler/src/own/instance_check.rs`, `compiler/src/sema/mod.rs`, `compiler/src/sema/expr.rs` ✅ (completed: 2026-11-11 15:21)
- [x] 1.2.3 Re-run the spec suite; all nullable/formattability cases green — verify: the suite output ✅ (completed: 2026-11-11 15:21)
- [x] 1.2.4 Re-run the module-initializer case (`def_zero_null_spec_test::should_judge_module_initializer_call_and_template_arguments`) — verify: `cargo test -p clc --test def_zero_null_spec_test` ✅ (completed: 2026-11-11 15:21)

### Step 1.3: Impl Tests and Hardening

**Reference**: [07 §3](07-testing-strategy.md) IT-11 · AR-1, AR-16
**Objective**: Pin the diagnostic surface and harden the phase.

- [x] 1.3.1 Create `compiler/tests/template_placeholder_integrity_impl_test.rs` with IT-11 and IT-12 — `compiler/tests/template_placeholder_integrity_impl_test.rs` ✅ (completed: 2026-11-11 15:21)
- [x] 1.3.2 Create `plans/template-placeholder-integrity/05-findings.md` as the findings ledger — `plans/template-placeholder-integrity/05-findings.md` ✅ (completed: 2026-11-11 15:21)
- [x] 1.3.3 Run the phase review (correctness-reviewer, semantics-reviewer, memory-safety-auditor) on the phase diff and record the verdicts — verify: the verdicts are recorded in `05-findings.md` ✅ (completed: 2026-11-11 15:45)
- [x] 1.3.4 Fix every critical/major finding and re-review the fix diff once — verify: the re-review verdict ✅ (completed: 2026-11-11 15:45)
- [x] 1.3.5 `make check` green — verify: `make check` ✅ (completed: 2026-11-11 15:45)

**Deliverables**:
- [ ] The function-body nullable rule matches the module path
- [ ] The new spec/impl cases are green and the phase review is clean

**Verify**: `make check`

---

## Phase 2: Ordered Placeholder Evaluation

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: correctness, semantics, memory-safety, emission-integrity
> **Reasoning**: high — ownership, deferred fills, and emission order interact across two lowering paths

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-ordered-evaluation.md) · [07](07-testing-strategy.md) ST-6–ST-21, ST-25 · AR-3–AR-11
**Objective**: Pin once-only, source-order evaluation and the copy-before-move rule in every evaluation position.

- [x] 2.1.1 [spec-author] Add ST-6–ST-14 (body call count, take sibling, view over move, reverse-order guard, scalar order, generic String/scalar) — `compiler/tests/template_placeholder_integrity_spec_test.rs` ✅ (completed: 2026-11-11 16:05)
- [x] 2.1.2 [spec-author] Add ST-15–ST-21, ST-25, ST-28–ST-31, ST-35, ST-38–ST-41 (eager `?:`, short-circuit, loop condition, module initializer, literal-only, skipped-arm, String source, projected member, conditional view, static-str member, index trap, literal order, adopted loop field, pinned guard) — `compiler/tests/template_placeholder_integrity_spec_test.rs` ✅ (completed: 2026-11-11 16:05)
- [x] 2.1.3 Run the suite against the phase-start `clc`; record the red evidence for every fix case and label the guards — verify: the failure log names each red case ✅ (completed: 2026-11-11 16:05)

### Step 2.2: Implementation

**Reference**: [03-02 §3–§8](03-02-ordered-evaluation.md) · AR-3–AR-12
**Objective**: Lower each string-like placeholder copy at its source position and keep pure scalar-only templates unchanged.

- [x] 2.2.1 Make `expression_has_effects` `pub(super)` for the sibling lowering module; update its call site — `compiler/src/own/call.rs` ✅ (completed: 2026-11-11 16:05)
- [x] 2.2.2 Add the `owned` provenance flag to `ValuePiece::Placeholder` and update every `ValuePiece` walk to carry it — `compiler/src/ir.rs` and its 29 match sites ✅ (completed: 2026-11-11 16:05)
- [x] 2.2.3 Replace the template placeholder mapping in `lower_value` with the ordered walk and the three materialization helpers — `compiler/src/own/lower.rs` ✅ (completed: 2026-11-11 16:05)
- [x] 2.2.4 Add the `$text` early arm to `render_call`; adopt only an `owned` String part and keep the raw String/str copy — `compiler/src/codegen/expr.rs` ✅ (completed: 2026-11-11 16:05)
- [x] 2.2.5 Add the ordered once-only walk to `materialize_init_value`'s Template arm using the pre-walk `init_operand_is_pure` scan, and retain the copy local — `compiler/src/codegen/program.rs` ✅ (completed: 2026-11-11 16:05)
- [x] 2.2.6 Run the new suite on host; every case green — verify: `cargo test -p clc --test template_placeholder_integrity_spec_test` ✅ (completed: 2026-11-11 16:05)

### Step 2.3: Impl Tests and Hardening

**Reference**: [07 §3](07-testing-strategy.md) IT-1–IT-10 · AR-8–AR-12
**Objective**: Pin the intended emission, prove the sanitizer safety, and review the phase.

- [x] 2.3.1 Add IT-1–IT-10 and IT-13 to the impl suite (IT-11/IT-12 already exist from Phase 1) — `compiler/tests/template_placeholder_integrity_impl_test.rs` ✅ (completed: 2026-11-11 16:05)
- [x] 2.3.2 Update the intended #25 emission pins for the String/str part shape — `compiler/tests/template_direct_formatting_impl_test.rs` ✅ (completed: 2026-11-11 16:05)
- [x] 2.3.3 Run the impl suite and the #25 suites green — verify: `cargo test -p clc --test template_placeholder_integrity_impl_test --test template_direct_formatting_impl_test --test template_direct_formatting_spec_test` ✅ (completed: 2026-11-11 16:05)
- [x] 2.3.4 Run the new spec suite under ASan, including the `String`-returning-call-in-`&&` case; no report and no leak — verify: `RDC_ASAN=1 cargo test -p clc --test template_placeholder_integrity_spec_test` ✅ (completed: 2026-11-11 16:05)
- [x] 2.3.5 Run the phase review (correctness, semantics, memory-safety, emission-integrity) on the phase diff — verify: the verdicts are recorded in `05-findings.md` ✅ (completed: 2026-11-11 16:35)
- [x] 2.3.6 Fix every critical/major finding and re-review the fix diff once — verify: the re-review verdict ✅ (completed: 2026-11-11 17:25)
- [x] 2.3.7 `make check` green — verify: `make check` ✅ (completed: 2026-11-11 16:35)

**Deliverables**:
- [ ] Every placeholder evaluates once in source order in all positions
- [ ] The copy precedes the move in the body, guarded, generic, and initializer paths
- [ ] The phase review is clean of critical/major findings

**Verify**: `make check`

---

## Phase 3: Acceptance and Closeout

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: correctness, memory-safety, emission-integrity, performance
> **Reasoning**: medium — full-gate execution and evidence recording, no new behavior

### Step 3.1: Full Gate

**Reference**: [07 §4–§5](07-testing-strategy.md) · AR-16
**Objective**: Prove the complete acceptance surface and the performance budget.

- [ ] 3.1.1 `make asan` all suites green — verify: the runner's summary
- [ ] 3.1.2 `make ubsan` all suites green — verify: the runner's summary
- [ ] 3.1.3 `make bench-base COMMIT=8b4bff2`, `make bench`, `make bench-compare` green with equal outputs and the 1% guard — verify: the comparison output
- [ ] 3.1.4 `make bench-shape` green — verify: the four shape legs
- [ ] 3.1.5 `make verify` all legs green — verify: `verify: all legs passed`

### Step 3.2: Closeout

**Reference**: [01 §Acceptance](01-requirements.md) · AR-16
**Objective**: Record the evidence and sync the project state.

- [ ] 3.2.1 Write `plans/template-placeholder-integrity/05-findings.md` with the baseline/head metrics, red evidence, review outcomes, and the gate results — `plans/template-placeholder-integrity/05-findings.md`
- [ ] 3.2.2 Update the existing PLAN-30 row in `plans/00-roadmap.md` to Done and correct the progress counters — `plans/00-roadmap.md`
- [ ] 3.2.3 Close issue #26 with the delivered evidence and the residual note (AR-18) — GitHub issue #26 via `gh issue close --comment`
- [ ] 3.2.4 File the follow-up issue for the `$"{null}"` placeholder ICE (AR-21) — GitHub via `gh issue create`

**Deliverables**:
- [ ] The full gate is green
- [ ] The findings ledger and roadmap record the outcome
- [ ] Issue #26 is closed with evidence

**Verify**: `make verify` and the recorded gate outputs

---

## Dependencies

```
Phase 1 (nullable rule)
    ↓
Phase 2 (ordered evaluation)
    ↓
Phase 3 (acceptance and closeout)
```

Phase 2's implementation depends on Phase 1 only for the shared spec-suite
file; the lowering changes are independent but are executed in order so the
phase reviews see a focused diff.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ R1–R8 verified by the ST cases
3. ✅ `make check`, `make asan`, `make ubsan`, `make verify`, `make bench-shape`, and `make bench-compare` green
4. ✅ All scored outputs equal; no scored metric regression above 1%
5. ✅ No critical/major review finding open
6. ✅ The findings ledger, roadmap, and issue #26 reflect the evidence
7. ✅ No dead code; the new helpers are documented and used
8. ✅ No spec, shim, or ABI change
