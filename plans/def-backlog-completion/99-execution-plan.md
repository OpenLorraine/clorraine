# Execution Plan: Def Backlog Completion

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-03 13:23
> **Progress**: 5/31 tasks (16%)
> **CodeOps Artifact Schema**: 1

## Overview

Close the seven open defects DEF-56 – DEF-62 in four phases: generic-lend ownership, do-while
condition release, closure C spelling, and close-out. Each implementation phase follows the
specification-first ordering (spec tests → red phase → implement → green phase → impl tests →
verify) from `07-testing-strategy.md`.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Generic-lend ownership (DEF-56, DEF-59, DEF-60, DEF-61) | 12 |
| 2 | Do-while condition release (DEF-62) | 6 |
| 3 | Closure C spelling (DEF-57, DEF-58) | 8 |
| 4 | Close-out | 5 |

**Total: 31 tasks across 4 phases** (no fabricated hour estimates — scope is bounded by the
task-size criteria: each task is independently testable and reviewable)

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
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is resumed first,
>    else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line. The
>    plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented. Failure to keep the marks current
> means progress is invisible after crashes, context resets, or session handoffs.

---

## Phase 1: Generic-lend ownership (DEF-56, DEF-59, DEF-60, DEF-61)

> **Phase baseline tree**: ad2fff6aa69f25efce9928ea5190adb7c9f3b0a6

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-generic-lend-duties.md) · [03-02](03-02-move-out-integrity.md) · [AR-1, AR-2, AR-5, AR-9, AR-10](00-ambiguity-register.md)
**Objective**: Pin the four defects' required behavior before any implementation.

- [x] 1.1.1 [spec-author] Write specification tests from ST-1 – ST-11 and ST-20 – ST-26 — `compiler/tests/def_backlog_completion_spec_test.rs` ✅ (completed: 2026-10-03 11:04)
- [x] 1.1.2 Run the spec suite and record the red phase: the acceptance cases fail with `E10034`
  (ST-1/2/4, ST-21 – ST-23, ST-26); the double-free cases abort where the shape reaches `free`
  (DEF-60); the use-after-free and leak cases are captured with the host AddressSanitizer probe
  described in [07-testing-strategy.md](07-testing-strategy.md) (ST-9/10, ST-20, ST-25); the
  duplicable, constant-index, exemption, and regression cases pass pre-implementation (justified:
  they pin unchanged behavior) ✅ (completed: 2026-10-03 11:06)

> **Red-phase evidence (2026-10-03 11:06)**: `cargo test --test def_backlog_completion_spec_test`
> → `5 passed; 13 failed`. Pass (pinned behavior): ST-3, ST-5, ST-6, ST-11, ST-24. Fail
> (acceptance red): ST-1, ST-2, ST-4 (accepted instead of `E10034`), ST-7, ST-8, ST-25 (double
> free), ST-9, ST-10 (use-after-free / wrong result), ST-20 (copy without the zeroing move),
> ST-21, ST-22, ST-23 (accepted instead of `E10034`), ST-26 (one `E10020`, no `E10034`).

**Deliverables**:
- [ ] Specification suite with ST-1 – ST-11 and ST-20 – ST-26
- [ ] Red-phase evidence recorded in this plan

**Verify**: `make check` (the new suite compiles; expected failures recorded)

---

### Step 1.2: Implementation

**Reference**: [03-01](03-01-generic-lend-duties.md) §Proposed Changes · [03-02](03-02-move-out-integrity.md) §Proposed Changes
**Objective**: Reject the generic lend escape; make the move-out index-correct and the store/return
element reads move before release.

- [x] 1.2.1 Add the duty record, collection, and post-substitution `E10034` audit with one report
  per site, no param-mention early skip, the `BindingKind::Param` predicate, and the `borrow T?`
  exemption — `ir/mono.rs`, `own.rs`, `own/instance_check.rs` ✅ (completed: 2026-10-03 13:18)
- [x] 1.2.2 Record the lend duty at the center: `mark_moved_if_owning` takes the move span and
  records a `RequiresNotLend` for a lend-parameter root (field or element projection), deriving the
  parameter and type internally; add the span at the eleven call sites — `own/place.rs`,
  `own/statements.rs`, `own/call_check.rs`, `own/expr.rs` ✅ (completed: 2026-10-03 13:22; the four
  colliding test programs were migrated with this task as a mechanical correction)
- [x] 1.2.3 Add `Projection::ElementTemp`; map a materialized index in `value_place`; add the
  explicit arms at every exhaustive match (`own/expr.rs`, `own/place.rs`, `own.rs`) and render it
  through `index_text` — `ir/place.rs`, `own/lower.rs`, `own/expr.rs`, `own/place.rs`, `own.rs`,
  `codegen/expr.rs` ✅ (completed: 2026-10-03 13:23)
- [ ] 1.2.4 Add the guard-aware, chain-recursing index-materialization helper in
  `materialize_owned_argument`'s take block so the argument and the move-out share it, covering the
  discarded-call path — `own/lower.rs`, `own/statements.rs`
- [ ] 1.2.5 Update the stale move-out pin to the corrected `__cl_tmp[i] = NULL` shape after the
  index-carrying move-out lands — `compiler/tests/residual_defect_completion_impl_test.rs`
  (AR-6, PF-002; the program migrations were applied with task 1.2.2)
- [ ] 1.2.6 Move the store out of a statement-owned destination temporary instead of copying, and
  document the DEF-64 aggregate-literal limitation — `own/statements.rs`
- [ ] 1.2.7 Move the return source in the no-ladder branch before the temporary releases —
  `codegen/cleanup.rs`
- [ ] 1.2.8 Run the spec suite — green phase (a failing case means the implementation is wrong; do
  not edit the case)

**Deliverables**:
- [ ] Duty machinery and the four fixes
- [ ] Spec suite green
- [ ] No `spec/` file changed

**Verify**: `make check`

---

### Step 1.3: Implementation Tests & Hardening

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) §Implementation Tests · [AR-12](00-ambiguity-register.md)
**Objective**: Pin emitted shapes, report counts, single evaluation, and re-emission.

- [ ] 1.3.1 Write implementation tests: duty record and report dedupe; no duty for `take`/locals;
  `__cl_tmp[i] = NULL` for the take and store; return source-move ordering; one index
  materialization; constant-index byte-identical re-emission — `compiler/tests/def_backlog_completion_impl_test.rs`
- [ ] 1.3.2 Full verification and determinism check — `make check`

**Deliverables**:
- [ ] Implementation suite for Phase 1
- [ ] All verification passing

**Verify**: `make check`

---

## Phase 2: Do-while condition release (DEF-62)

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_

### Step 2.1: Specification Tests

**Reference**: [03-03](03-03-do-while-condition-release.md) · [AR-4](00-ambiguity-register.md)
**Objective**: Pin the early-return release before implementation.

- [ ] 2.1.1 [spec-author] Write specification tests from ST-12 – ST-15 — append to `compiler/tests/def_backlog_completion_spec_test.rs`
- [ ] 2.1.2 Run the spec suite and record the red phase: ST-12 – ST-13 and ST-15 leak (captured with
  the host AddressSanitizer probe; each case pins an exit code); ST-14 passes pre-implementation
  (justified: it pins unchanged fall-through and break behavior)

**Deliverables**:
- [ ] Specification cases ST-12 – ST-15
- [ ] Red-phase evidence recorded

**Verify**: `make check`

---

### Step 2.2: Implementation

**Reference**: [03-03](03-03-do-while-condition-release.md) §Proposed Changes
**Objective**: Attach the condition's materialized releases to the body's early returns.

- [ ] 2.2.1 Add the builder mutator that prepends release places (newest-first, deduped) to the
  return terminators reachable from the loop body, and call it in `check_do_while` with the
  checkpoint-sliced releases registered while lowering the condition; reuse `liveness::successors`
  for reachability and record the release events — `compiler/src/ir.rs`, `own/loops.rs`
- [ ] 2.2.2 Run the spec suite — green phase

**Deliverables**:
- [ ] Release append implemented
- [ ] Spec suite green

**Verify**: `make check`

---

### Step 2.3: Implementation Tests & Hardening

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) §Implementation Tests
**Objective**: Pin the appended release shape and the nested-loop behavior.

- [ ] 2.3.1 Write implementation tests: the return arm carries the condition release; nested
  do-while returns carry both loops' releases; no release is appended outside the body; re-emission
  is byte-identical — `compiler/tests/def_backlog_completion_impl_test.rs`
- [ ] 2.3.2 Full verification — `make check`

**Deliverables**:
- [ ] Implementation suite for Phase 2
- [ ] All verification passing

**Verify**: `make check`

---

## Phase 3: Closure C spelling (DEF-57, DEF-58)

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_

### Step 3.1: Specification Tests

**Reference**: [03-04](03-04-closure-c-spelling.md) · [AR-3](00-ambiguity-register.md)
**Objective**: Pin the callable-bound closure spelling and the zero-argument call before implementation.

- [ ] 3.1.1 [spec-author] Write specification tests from ST-16 – ST-19 and ST-27 — append to `compiler/tests/def_backlog_completion_spec_test.rs`
- [ ] 3.1.2 Run the spec suite and record the red phase: ST-16/17 fail the C build (`ClClosure_0`
  unknown); ST-27 emits one collapsed symbol for two enclosing instances; ST-19 fails the C build
  (trailing comma); ST-18 passes pre-implementation (justified: it pins the unchanged non-generic
  spelling)

**Deliverables**:
- [ ] Specification cases ST-16 – ST-19 and ST-27
- [ ] Red-phase evidence recorded

**Verify**: `make check`

---

### Step 3.2: Implementation

**Reference**: [03-04](03-04-closure-c-spelling.md) §Proposed Changes
**Objective**: Make the call site and the callee agree, and fix the empty argument list.

- [ ] 3.2.1 Relocate `type_key` and its slice helpers to the stage-neutral `types` module; include
  the closure instance in `type_key`'s closure arm and in `dedupe_arg` — `compiler/src/types/`,
  `codegen/types.rs`, `codegen/calls.rs`, `ir/mono.rs`
- [ ] 3.2.2 Record the enclosing lifted specializations before walking the body, then stamp the
  enclosing instance key on recorded closure type arguments that have a lifted specialization for
  that instance — `own/generics.rs`
- [ ] 3.2.3 Omit the separator for an empty closure argument list — `codegen/expr.rs`
- [ ] 3.2.4 Run the spec suite — green phase

**Deliverables**:
- [ ] Key relocation, stamping, and renderer fix
- [ ] Spec suite green
- [ ] `own` does not import `codegen`

**Verify**: `make check`

---

### Step 3.3: Implementation Tests & Hardening

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) §Implementation Tests
**Objective**: Pin the emitted symbol names and the empty call text.

- [ ] 3.3.1 Write implementation tests: both enclosing specializations' struct and symbol names;
  the non-generic `ClClosure_0` spelling; the `((f).fn((f).env))` text; byte-identical re-emission
  — `compiler/tests/def_backlog_completion_impl_test.rs`
- [ ] 3.3.2 Full verification — `make check`

**Deliverables**:
- [ ] Implementation suite for Phase 3
- [ ] All verification passing

**Verify**: `make check`

---

## Phase 4: Close-out

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_

### Step 4.1: Acceptance, Probes, Docs, and Summary

**Reference**: [01-requirements.md](01-requirements.md) §Acceptance Criteria · [AR-6, AR-7, AR-10, AR-12](00-ambiguity-register.md)
**Objective**: Prove the plan against the acceptance criteria and sync the project records.

- [ ] 4.1.1 Full cross-toolchain verification — `make verify`
- [ ] 4.1.2 Build and run the close-out probe runner: five AddressSanitizer run probes (DEF-57,
  DEF-58, DEF-59, DEF-60, DEF-62) and two rejection probes (DEF-56, DEF-61: `E10034`, no C
  written), each with an expected signature, a `timeout`, and
  `ASAN_OPTIONS=detect_leaks=1:symbolize=0:handle_segv=0` for determinism —
  `build/def-backlog-probes/run-def-backlog-completion.sh` (gitignored)
- [ ] 4.1.3 Sync documentation: add both suites to the `AGENTS.md` test map; mark DEF-56 – DEF-62
  Done; verify the DEF-63 and DEF-64 backlog rows and the corrected DEF-60 remedy — `AGENTS.md`,
  `plans/00-roadmap.md`
- [ ] 4.1.4 Check every acceptance criterion in [01-requirements.md](01-requirements.md); set the
  plan status Complete and the progress header to 31/31
- [ ] 4.1.5 Phase review evidence and plan summary — commit per the active commit mode (git-commit
  skill in push mode)

**Deliverables**:
- [ ] `make verify` all legs passed
- [ ] Five ASan run probes clean and two rejection probes correct
- [ ] Documentation and roadmap synced; acceptance criteria checked

**Verify**: `make verify` and the probe runner

---

## Dependencies

```
Phase 1 (generic lend + move-out)
    ↓
Phase 2 (do-while release)
    ↓
Phase 3 (closure spelling)
    ↓
Phase 4 (close-out)
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check` per phase; `make verify` at close-out)
3. ✅ No warnings/errors; `spec/` unchanged
4. ✅ No dead code — no unused parameters, functions, or modules
5. ✅ Memory safety hardened — five reproductions run with AddressSanitizer and leak detection with
   no double free, leak, or use-after-free, and the two rejection reproductions prove `E10034`
   with no C emitted
6. ✅ Documentation updated (`AGENTS.md`, roadmap rows; DEF-63/DEF-64 verified filed; DEF-60 remedy
   corrected)
7. ✅ Code reviewed (post-phase reviews per the quality profile)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
