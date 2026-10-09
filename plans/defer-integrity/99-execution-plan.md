# Execution Plan: Defer Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-09 15:36
> **Progress**: 2/32 tasks (6%)
> **CodeOps Artifact Schema**: 1

## Overview

Fix the five filed `defer` defects (DEF-84 – DEF-88) with one shared deferred-copy emission
context plus a gated return-ordering change ([03-01](03-01-per-site-deferred-copies.md),
[03-02](03-02-bounded-closure-and-context.md), [03-03](03-03-deferred-return.md)). Each phase
follows the specification-first cycle: ST cases (red) → implementation (green) → implementation
tests, probes, and a phase review. Execution commit mode: **auto-commit** (user instruction) —
every verified task commits via the git-commit skill in push mode.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Per-site deferred copies (DEF-86, DEF-87) | 8 |
| 2 | Bounded closure and ambient context (DEF-84) | 9 |
| 3 | Deferred return continuation and ordering (DEF-85, DEF-88) | 9 |
| 4 | Plan-wide hardening and close-out | 6 |

**Total: 32 tasks across 4 phases** (no fabricated hour estimates — scope is bounded by the
task-size criteria in the make-plan quality checklist)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for progress.
> Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the Last Updated stamp
>    after EVERY task — never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is resumed
>    first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line.
>    The plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented. Failure to keep the marks
> current means progress is invisible after crashes, context resets, or session handoffs.

---

## Phase 1: Per-Site Deferred Copies (DEF-86, DEF-87)

> **Phase baseline tree**: `01b778fb8812c6e8e566f7c2cb0a5d3813f6ccf8`
> **Expected modification set**: `compiler/tests/defer_integrity_spec_test.rs`, `compiler/tests/defer_integrity_impl_test.rs`, `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/control.rs`, `compiler/src/own.rs`, `plans/defer-integrity/*` · Scope: `strict`
> **Lenses**: correctness, maintainability, standards (base lenses; profile-driven)
> **Reasoning**: medium — a focused emission-mechanism change with traced root causes

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-per-site-deferred-copies.md) §Proposed Changes · [AR-7, AR-10](00-ambiguity-register.md)
**Objective**: Register the every-edge behavior as specification tests before any emitter change.

- [x] 1.1.1 [spec-author] Write specification tests ST-1 – ST-4 in `compiler/tests/defer_integrity_spec_test.rs` ✅ (completed: 2026-10-09 15:36)
- [x] 1.1.2 Run the new suite and record the red phase (the fall-through site's empty copy; the suppressed outer copy; the early-return sibling; the multi-site counter) ✅ (completed: 2026-10-09 15:36)

**Red-phase evidence (2026-10-09 15:36)** — the new suite (`defer_integrity_spec_test`, four
tests) fails at the phase baseline exactly as designed: ST-1 exits 0 (expected 9 — the
fall-through site runs no defer); ST-2 exits 0 (expected 7 — the earlier returning defer never
fires); ST-3 exits 1 (expected 21 — the defer runs on at most one path); ST-4 exits 13 (expected
33 — one of three cleanup sites runs the defer). Run: `cargo test --test defer_integrity_spec_test`
→ 0 passed, 4 failed.

**Deliverables**:
- [x] ST-1 – ST-4 in the new spec suite
- [x] Red-phase evidence

**Verify**: `make check` (the new suite fails as expected)

### Step 1.2: Implementation

**Reference**: [03-01](03-01-per-site-deferred-copies.md) §Proposed Changes
**Objective**: Emit one copy per cleanup activation with a copy-local printed set and label
namespace, and keep the deferred effects checked on every edge.

- [ ] 1.2.1 Emit one copy per cleanup activation: copy-local printed set, per-root copy counter, deterministic label suffix (first copy unchanged) — `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/control.rs`
- [ ] 1.2.2 Keep the deferred entries available across exit edges with an active-entry guard (per-edge effect checking) — `compiler/src/own.rs`
- [ ] 1.2.3 Run the spec suite and record the green phase

**Deliverables**:
- [ ] Per-site copies with stable labels
- [ ] Per-edge effect checking
- [ ] ST-1 – ST-4 green

**Verify**: `make check`

### Step 1.3: Implementation Tests & Hardening

**Reference**: [03-01](03-01-per-site-deferred-copies.md) §Testing Requirements
**Objective**: Pin the copy shapes and prove the memory behavior; review the phase.

- [ ] 1.3.1 Write implementation tests for the per-site copy shape, the label spelling (first copy unchanged, later copies suffixed), and byte-identical re-emission — `compiler/tests/defer_integrity_impl_test.rs`
- [ ] 1.3.2 Run the host-ASan probes for ST-1 – ST-4 and `make check`
- [ ] 1.3.3 Dispatch the phase review (correctness + memory-safety + emission-integrity; blast-radius and test-integrity steps) and resolve findings; one bounded re-review of a fix diff

**Deliverables**:
- [ ] Implementation suite for the copy shapes
- [ ] Clean probe matrix
- [ ] Phase review with no open critical/major finding

**Verify**: `make check`

---

## Phase 2: Bounded Closure and Ambient Context (DEF-84)

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: correctness, maintainability, standards; semantics (defer exit edges are a language-rule surface)
> **Reasoning**: high — control-flow edge classification across the copy boundary

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-bounded-closure-and-context.md) §Proposed Changes · [AR-6](00-ambiguity-register.md)
**Objective**: Pin the strict-build and loop-edge behavior of `continue`/`break` inside a defer.

- [ ] 2.1.1 [spec-author] Write specification tests ST-5 – ST-7 in `compiler/tests/defer_integrity_spec_test.rs`
- [ ] 2.1.2 Run the new suite and record the red phase (the undefined `__cl_b2` label; the swallowed loop; the break variant baseline)

**Deliverables**:
- [ ] ST-5 – ST-7 in the spec suite
- [ ] Red-phase evidence

**Verify**: `make check` (the new cases fail as expected)

### Step 2.2: Implementation

**Reference**: [03-02](03-02-bounded-closure-and-context.md) §Proposed Changes
**Objective**: Give the copy its own region and the ambient flow context; guard rung-hosted
out-edges.

- [ ] 2.2.1 Record the deferred block's own block range at lowering and bound the copy to it — `compiler/src/own/statements.rs`, `compiler/src/ir.rs`, `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/control.rs`
- [ ] 2.2.2 Thread the ambient flow context through the cleanup chain so out-edges classify as `continue`/`break` — `compiler/src/codegen/control.rs`, `compiler/src/codegen/cleanup.rs`
- [ ] 2.2.3 Add the inline-only guard for scopes whose deferred regions contain break/continue out-edges, and run the rung-hosted sibling verification probe — `compiler/src/codegen/cleanup.rs`
- [ ] 2.2.4 Run the spec suite and record the green phase

**Deliverables**:
- [ ] Bounded copy with ambient context
- [ ] Rung guard and its probe evidence
- [ ] ST-5 – ST-7 green

**Verify**: `make check`

### Step 2.3: Implementation Tests & Hardening

**Reference**: [03-02](03-02-bounded-closure-and-context.md) §Testing Requirements
**Objective**: Pin the bounded copy, the edge spelling, and the guard; review the phase.

- [ ] 2.3.1 Write implementation tests for the bounded copy (no outer block inside it), the `continue;`/`break;` spelling, and byte-identical re-emission — `compiler/tests/defer_integrity_impl_test.rs`
- [ ] 2.3.2 Run the host-ASan and strict-build probes for ST-5 – ST-7, the rung guard probe, and `make check`
- [ ] 2.3.3 Dispatch the phase review (correctness + memory-safety + emission-integrity + semantics) and resolve findings; one bounded re-review of a fix diff

**Deliverables**:
- [ ] Implementation suite for the bounded copy
- [ ] Clean probe matrix (including the rung sibling outcome)
- [ ] Phase review with no open critical/major finding

**Verify**: `make check`

---

## Phase 3: Deferred Return Continuation and Ordering (DEF-85, DEF-88)

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: correctness, maintainability, standards; semantics (evaluation order and cleanup order)
> **Reasoning**: high — the continuation interacts with the ladder, nested re-entry, and return forms

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-deferred-return.md) §Proposed Changes · [AR-5, AR-8](00-ambiguity-register.md)
**Objective**: Pin the deferred-return values, the remaining-actions behavior, and the
value-before-cleanup ordering.

- [ ] 3.1.1 [spec-author] Write specification tests ST-8 – ST-14 in `compiler/tests/defer_integrity_spec_test.rs`
- [ ] 3.1.2 Run the new suite and record the red phase (the self-loop hangs; the segfaults)

**Deliverables**:
- [ ] ST-8 – ST-14 in the spec suite
- [ ] Red-phase evidence

**Verify**: `make check` (the new cases fail as expected)

### Step 3.2: Implementation

**Reference**: [03-03](03-03-deferred-return.md) §Proposed Changes
**Objective**: Continue the running cleanup on a deferred return; reorder value returns.

- [ ] 3.2.1 Add the running-cleanup frame and the continuation label for returning copies (returns inside a copy jump to the continuation) — `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/control.rs`
- [ ] 3.2.2 Nested re-entry into a running scope emits only the actions older than the frame's position — `compiler/src/codegen/cleanup.rs`
- [ ] 3.2.3 For value returns, store the value (and run the terminator's releases) before the trailing inline cleanup, on every return form — `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/control.rs`
- [ ] 3.2.4 Run the spec suite and record the green phase

**Deliverables**:
- [ ] Continuation and remaining-actions rule
- [ ] Value-before-cleanup ordering
- [ ] ST-8 – ST-14 green

**Verify**: `make check`

### Step 3.3: Implementation Tests & Hardening

**Reference**: [03-03](03-03-deferred-return.md) §Testing Requirements
**Objective**: Pin the continuation, the action order, and the return ordering; review the phase.

- [ ] 3.3.1 Write implementation tests for the continuation shape, the remaining-actions order, the buffered-cleanup order, and byte-identical re-emission — `compiler/tests/defer_integrity_impl_test.rs`
- [ ] 3.3.2 Run the host-ASan matrix (release counts and side effects exactly once per path) and `make check`
- [ ] 3.3.3 Dispatch the phase review (correctness + memory-safety + emission-integrity + semantics) and resolve findings; one bounded re-review of a fix diff

**Deliverables**:
- [ ] Implementation suite for the return family
- [ ] Clean probe matrix
- [ ] Phase review with no open critical/major finding

**Verify**: `make check`

---

## Phase 4: Plan-wide Hardening and Close-out

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: all four project review roles over the plan diff
> **Reasoning**: medium — sweeps and the final review

### Step 4.1: Plan-wide Sweeps

**Reference**: [00-index](00-index.md) §Minimum-Sufficient Baseline · [AR-10, AR-13](00-ambiguity-register.md)
**Objective**: Prove the plan-wide emission and memory behavior.

- [ ] 4.1.1 Base-vs-head emission sweep over the corpus in every profile and the suite programs; classify every diff against the design and confirm defer-free programs are byte-identical
- [ ] 4.1.2 Plan-wide host-ASan matrix over all changed shapes and `make verify` (all legs)

**Deliverables**:
- [ ] Emission sweep with every diff traced
- [ ] Memory sweep clean
- [ ] `make verify: all legs passed`

**Verify**: `make verify`

### Step 4.2: Final Review

**Reference**: [00-index](00-index.md) · all three component docs
**Objective**: Independent review of the whole plan diff.

- [ ] 4.2.1 Dispatch the final plan review (correctness + memory-safety + emission-integrity + semantics over the plan diff) and resolve findings
- [ ] 4.2.2 Bounded re-review of any fix diff

**Deliverables**:
- [ ] Final review with no open critical/major finding

**Verify**: `make check`

### Step 4.3: Close-out

**Reference**: [01-requirements.md](01-requirements.md) §Acceptance Criteria
**Objective**: Record the outcome and sync the project records.

- [ ] 4.3.1 Update the AGENTS.md test-naming map with the new suites and sync the roadmap (DEF-84 – DEF-88 to Done per the roadmap skill)
- [ ] 4.3.2 Record the final evidence, clean the session's temporary artifacts, and present the summary

**Deliverables**:
- [ ] Docs and roadmap updated
- [ ] Final evidence recorded

**Verify**: `make verify`

---

## Dependencies

```
Phase 1 (per-site copies)
    ↓
Phase 2 (bounded closure + context, reuses the copy activation)
    ↓
Phase 3 (continuation + ordering, reuses the frames and copies)
    ↓
Phase 4 (plan-wide hardening and close-out)
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check` per phase; `make verify` at the end)
3. ✅ No warnings/errors (strict host build on every accepted unit)
4. ✅ No dead code — no unused parameters, functions, or modules
5. ✅ Security: N/A (no input path); memory safety verified by host AddressSanitizer
6. ✅ Documentation updated (AGENTS.md test map, roadmap)
7. ✅ Code reviewed (phase reviews plus the final plan review)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
