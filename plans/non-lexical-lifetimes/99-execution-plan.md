# Execution Plan: Non-Lexical Lifetimes (NLL v2)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-27 09:04
> **Progress**: 2/22 tasks (9%)
> **CodeOps Artifact Schema**: 1

## Overview

Replace the version-1 lexical loan model with non-lexical lifetimes: record
ordered events during the existing ownership walk, compute last-use regions on
the CFG the walk already builds, decide `E10022`/`E10031`/`E10032`/`E10030`
from the regions, and delete the lexical state. The frozen chapters are amended
to describe v2, and the one flipping golden is rewritten (AR-2, AR-4, AR-10).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Specification tests and frozen-spec coherence | 5 |
| 2 | Liveness implementation | 10 |
| 3 | Implementation tests and hardening | 4 |
| 4 | Close-out | 3 |

**Total: 22 tasks across 4 phases** (no fabricated hour estimates — scope is
bounded by the task-size criteria in the make-plan quality checklist)

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
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>`
>    on the same line.

---

## Phase 1: Specification Tests and Frozen-Spec Coherence

> **Phase baseline tree**: 93a099a8c079a4b42bfbbd716707f5e63b1dab85
> **Scope mode**: strict · **Expected modifications**: `compiler/tests/non_lexical_lifetimes_spec_test.rs` (new), `spec/16-borrow-rules.md`, `spec/lineage.md`, `spec/decisions.md`, `spec/03-ownership.md`, `spec/tests/compile-fail/E10031_view_widening_mutation.clo`, `plans/non-lexical-lifetimes/99-execution-plan.md`
> **Lenses**: compiler and language

### Step 1.1: Specification Tests

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) ST-1 – ST-16 · AR-3, AR-5, AR-7, AR-8, AR-11
**Objective**: Express the v2 last-use behavior as failing tests before any checker change.

- [x] 1.1.1 [spec-author] Write the specification tests from ST-1 – ST-16 — `compiler/tests/non_lexical_lifetimes_spec_test.rs` ✅ (completed: 2026-09-27 09:04)
- [x] 1.1.2 Run the spec file and confirm the red set: ST-1, ST-2, ST-3, ST-8, ST-10, ST-12, ST-14 fail; ST-4 – ST-7, ST-9, ST-11, ST-13, ST-15 already pass; record the evidence ✅ (completed: 2026-09-27 09:04) — observed: 7 red (ST-1/2/3/8/10/12/14) + 9 green; ST-16 is already accepted by v1 (same-scope `defer` use runs before the release), so it is a regression guard, not a flipping case. `make check`: `spec/check.sh` OK (27 files, 104 compile-fail tests), every existing suite green, new suite 9 passed / 7 failed as intended.

**Deliverables**:
- The 16 ST cases exist in the dedicated spec file
- Red-phase evidence recorded in the task line

**Verify**: `make check` (expected: the new acceptance cases red, the existing suite green)

### Step 1.2: Frozen-Spec Coherence

**Reference**: [03-03-spec-amendment.md](03-03-spec-amendment.md) · AR-2, AR-11
**Objective**: Make the frozen chapters describe v2 and keep every golden presenting a still-rejected shape.

- [ ] 1.2.1 Apply the chapter edits of 03-03 — `spec/16-borrow-rules.md`, `spec/lineage.md`, `spec/decisions.md`, `spec/03-ownership.md`
- [ ] 1.2.2 Rewrite the flipping golden per 03-03 — `spec/tests/compile-fail/E10031_view_widening_mutation.clo`
- [ ] 1.2.3 Run the spec oracle and the compile-fail suite for the rewritten golden, and confirm the amended chapters pass while the golden still reports `E10031`

**Deliverables**:
- The frozen spec describes last-use regions; no other golden changed
- The rewritten golden still reports `E10031`

**Verify**: `./spec/check.sh`, plus `cargo test --test compile_fail_spec_test` in the SDK container for the rewritten golden (the new NLL spec tests stay red until Phase 2)

---

## Phase 2: Liveness Implementation

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: compiler and language

### Step 2.1: Event Recording and the Region Pass

**Reference**: [03-01-liveness-model.md](03-01-liveness-model.md), [03-02-checker-integration.md](03-02-checker-integration.md) §Recording hooks, §Post-pass · AR-3 – AR-10
**Objective**: Record the events, compute the regions, decide the four rules, and remove the lexical state.

- [ ] 2.1.1 Add the event types and checker state (`events`, `next_loan`, `next_defer`, `current_defer`; `LoanId`/`DeferId`/`Position`/`LoanEvent`/`LoanEventKind`/`AccessKind`; `Binding.loan`) — `compiler/src/own/liveness.rs`, `compiler/src/own/flow.rs`, `compiler/src/own.rs`
- [ ] 2.1.2 Add `view_loan`; create or share a loan in `bind_value`; record `Request` in `record_view_loan` and drop its inline `E10032` — `compiler/src/own/place.rs`, `compiler/src/own/statements.rs`
- [ ] 2.1.3 Record uses: `read_place`, writes through views, `check_return`, and call arguments — `compiler/src/own/place.rs`, `compiler/src/own/statements.rs`, `compiler/src/own/call_check.rs`
- [ ] 2.1.4 Record owner accesses and replace the inline `E10022`/`E10031` checks — `compiler/src/own/place.rs`
- [ ] 2.1.5 Record releases and defer edges; tag every event recorded while a deferred block is checked with its defer id; swap the log around `lift_lambda` — `compiler/src/own.rs`, `compiler/src/own/statements.rs`, `compiler/src/own/lower.rs`
- [ ] 2.1.6 Implement `successors`/`regions`/`check` (including the deferred-event resolution at each defer's exit edges) and wire the post-pass into the function check — `compiler/src/own/liveness.rs`, `compiler/src/own.rs`
- [ ] 2.1.7 Remove the lexical `Loans` set, `FlowState.loans`, the merge union, `end_scope`, and the obsolete unit tests — `compiler/src/borrow.rs`, `compiler/src/own/flow.rs`, `compiler/src/own.rs`
- [ ] 2.1.8 Remove the inline conflict rules and the lexical `E10030` per 03-02 §Removal — `compiler/src/own/statements.rs`, `compiler/src/own/place.rs`, `compiler/src/own/call_check.rs`
- [ ] 2.1.9 Run `make check` and confirm the Phase 1 spec tests pass (green phase)

**Deliverables**:
- Every hook of 03-02 records the specified event; no hook is missed
- Regions decide `E10022`/`E10031`/`E10032`/`E10030` in a deterministic post-pass
- Every event recorded inside a defer is evaluated at that defer's exit edges, never at a discarded position
- No lexical loan state remains

**Verify**: `make check`

### Step 2.2: Expectation Sweep

**Reference**: [03-02](03-02-checker-integration.md) §Error Handling · AR-11
**Objective**: Reconcile every pre-existing test whose expectation the model changes.

- [ ] 2.2.1 Sweep the suite for flipped or cascaded expectations; rewrite each to still pin its rule and record every change (the known flip set is one golden, already handled in 1.2.2)

**Deliverables**:
- Every rewrite listed in the task line, with its rule preserved
- No spec test expectation weakened without an AR-11 record

**Verify**: `make check`

---

## Phase 3: Implementation Tests and Hardening

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: compiler and language

### Step 3.1: Implementation Tests

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) §Implementation Tests · [03-02](03-02-checker-integration.md) §Testing Requirements
**Objective**: Cover the region internals and the accepted programs at runtime.

- [ ] 3.1.1 Add liveness unit tests: no-use empty region, branch-only use, use after the conflict, loop back edge, alias sharing, release reachability, report-once — `compiler/src/own/liveness.rs`
- [ ] 3.1.2 Add the implementation tests: ST-1/ST-10 accepted programs build and run; a conflict reports exactly once; two runs produce identical diagnostics — `compiler/tests/non_lexical_lifetimes_impl_test.rs`
- [ ] 3.1.3 Run the phase gate (`make check`)

### Step 3.2: Full Acceptance

- [ ] 3.2.1 Run the full acceptance gate (`make verify`) and record the result

**Deliverables**:
- Unit and implementation tests pass
- `make verify` green on every leg

**Verify**: `make check`, then `make verify`

---

## Phase 4: Close-out

> **Phase baseline tree**: _(recorded by the exec-plan skill)_

### Step 4.1: Traceability and Roadmap

**Reference**: [01-requirements.md](01-requirements.md) R1 – R9 · AR-11, AR-13
**Objective**: Record the close-out evidence and mark PLAN-12 done.

- [ ] 4.1.1 Trace R1 – R9 to the passing ST cases and record the acceptance evidence in this document
- [ ] 4.1.2 Set the `PLAN-12` row on `plans/00-roadmap.md` to Done and link it; update `AGENTS.md` only if a durable fact changed
- [ ] 4.1.3 Final phase gate (`make check`) and close-out commit

**Deliverables**:
- The traceability section covers every requirement
- The roadmap shows PLAN-12 done

**Verify**: `make check`

---

## Dependencies

```
Phase 1 (spec tests + spec coherence)
    ↓
Phase 2 (implementation)
    ↓
Phase 3 (impl tests + full acceptance)
    ↓
Phase 4 (close-out)
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check` after every task; `make verify` before completion)
3. ✅ No warnings/errors
4. ✅ No dead code — the lexical loan state is removed and no unused parameter, function, or module remains
5. ✅ Safety preserved — accepted programs stay sound; no new rejection was added; every rewrite is recorded (AR-11)
6. ✅ Documentation updated — the amended chapters, the plan state, and the roadmap
7. ✅ Code reviewed (strict profile — independent review of the phase diff)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
