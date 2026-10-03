# Execution Plan: Def Backlog Completion

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-03 19:34
> **Progress**: 31/38 tasks (82%)
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
| 1 | Generic-lend ownership (DEF-56, DEF-59, DEF-60, DEF-61) | 18 |
| 2 | Do-while condition release (DEF-62) | 7 |
| 3 | Closure C spelling (DEF-57, DEF-58) | 8 |
| 4 | Close-out | 5 |

**Total: 38 tasks across 4 phases** (no fabricated hour estimates — scope is bounded by the
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
- [x] 1.2.4 Add the guard-aware, chain-recursing index-materialization helper in
  `materialize_owned_argument`'s take block so the argument and the move-out share it, covering the
  discarded-call path — `own/lower.rs`, `own/statements.rs` ✅ (completed: 2026-10-03 13:25)
- [x] 1.2.5 Update the stale move-out pin to the corrected `__cl_tmp[i] = NULL` shape after the
  index-carrying move-out lands — `compiler/tests/residual_defect_completion_impl_test.rs`
  (AR-6, PF-002; the program migrations were applied with task 1.2.2) ✅ (completed: 2026-10-03 13:26)
- [x] 1.2.6 Move the store out of a statement-owned destination temporary instead of copying, and
  document the DEF-64 aggregate-literal limitation — `own/statements.rs` ✅ (completed: 2026-10-03 13:28; the limitation note lives on the aggregate-consume path in `own/lower.rs`)
- [x] 1.2.7 Move the return source in the no-ladder branch before the temporary releases —
  `codegen/cleanup.rs` ✅ (completed: 2026-10-03 13:29)
- [x] 1.2.8 Run the spec suite — green phase (a failing case means the implementation is wrong; do
  not edit the case) ✅ (completed: 2026-10-03 13:30)

> **Green-phase evidence (2026-10-03 13:30)**: `make check` exit 0 — all 39 suites green, including
> `def_backlog_completion_spec_test` at 18/18. The four migrated programs and the updated
> move-out pin stay green.

**Deliverables**:
- [ ] Duty machinery and the four fixes
- [ ] Spec suite green
- [ ] No `spec/` file changed

**Verify**: `make check`

---

### Step 1.3: Implementation Tests & Hardening

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) §Implementation Tests · [AR-12](00-ambiguity-register.md)
**Objective**: Pin emitted shapes, report counts, single evaluation, and re-emission.

- [x] 1.3.1 Write implementation tests: duty record and report dedupe; no duty for `take`/locals;
  `__cl_tmp[i] = NULL` for the take and store; return source-move ordering; one index
  materialization; constant-index byte-identical re-emission — `compiler/tests/def_backlog_completion_impl_test.rs` ✅ (completed: 2026-10-03 13:32)
- [x] 1.3.2 Full verification and determinism check — `make check` ✅ (completed: 2026-10-03 13:33)

> **Phase 1 verification evidence (2026-10-03 13:33)**: `make check` exit 0 — all 40 suites green
> (the specification suite at 18/18, the implementation suite at 7/7, and every pre-existing
> suite).

> **Phase 1 post-phase review (2026-10-03 13:36)**: one correctness reviewer ran on the phase diff;
> 7 findings, all ruled by the user: fix RV-101 (critical — the fixed-array row return bypassed the
> return move-out), RV-103 (major — the raw-element lend duty false negative), RV-104 (return index
> read once), RV-105 (audit docs), RV-106a (the guard-deferral test now asserts the assignment
> stays inside the guarded condition), and RV-107 (drop the redundant nullable clause). RV-102
> (major — slice-view element reads) is filed as DEF-66; RV-106b (the out-param write path) is an
> accepted limitation. The fixes are tasks 1.3.3 – 1.3.7 below.

- [x] 1.3.3 Fix the fixed-array row return: route a projected row through the byte-copy return core
  and pin it — `codegen/cleanup.rs`, `compiler/tests/def_backlog_completion_spec_test.rs` (RV-101) ✅ (completed: 2026-10-03 17:52)
- [x] 1.3.4 Fix the raw-element lend duty: the element projection sees through a raw pointer and
  records the pointee, with a rejection case — `own.rs`, `compiler/tests/def_backlog_completion_spec_test.rs` (RV-103) ✅ (completed: 2026-10-03 17:52)
- [x] 1.3.5 Read a returned element's run-time index once (materialize the chain on an owning
  return) and pin it — `own/statements.rs`, `compiler/tests/def_backlog_completion_impl_test.rs` (RV-104) ✅ (completed: 2026-10-03 17:52)
- [x] 1.3.6 Tighten the guard-deferral test and clean up the audit documentation and the redundant
  nullable clause — `compiler/tests/def_backlog_completion_impl_test.rs`, `own/instance_check.rs`,
  `own/place.rs` (RV-105, RV-106a, RV-107) ✅ (completed: 2026-10-03 17:52)
- [x] 1.3.7 Full verification of the review fixes and the bounded fix-diff re-review — `make check` ✅ (completed: 2026-10-03 17:58; the re-review verified all six fixes and, on its one allowed pass, found the RV-201 evaluation-order regression)
- [x] 1.3.8 Fix the re-review's evaluation-order regression: fill an eager projection base before
  pinning the index on the take, store, and return paths, and pin the order — `own/lower.rs`,
  `own/statements.rs`, `compiler/tests/def_backlog_completion_spec_test.rs` (RV-201) ✅ (completed: 2026-10-03 17:58)

> **Phase 1 review outcome (2026-10-03 17:58)**: all 7 findings resolved; the bounded re-review
> verified RV-101/103/104/105/106a/107 and found RV-201 (index pinned before the base) plus RV-202
> (whitespace). RV-201 is fixed (base filled first on take/store/return; ST-30 pins the order) and
> RV-202 is fixed; no third review pass per the profile cap. `make check` exit 0 — 40 suites
> (spec 21/21, impl 8/8). DEF-66 records the filed slice-view residual.

**Deliverables**:
- [ ] Implementation suite for Phase 1
- [ ] All verification passing

**Verify**: `make check`

---

## Phase 2: Do-while condition release (DEF-62)

> **Phase baseline tree**: dbc3cee (the phase-start commit; the review diff uses `git diff dbc3cee..HEAD`)

### Step 2.1: Specification Tests

**Reference**: [03-03](03-03-do-while-condition-release.md) · [AR-4](00-ambiguity-register.md)
**Objective**: Pin the early-return release before implementation.

- [x] 2.1.1 [spec-author] Write specification tests from ST-12 – ST-15 — append to `compiler/tests/def_backlog_completion_spec_test.rs` ✅ (completed: 2026-10-03 18:30)
- [x] 2.1.2 Run the spec suite and record the red phase: ST-12 – ST-13 and ST-15 leak (captured with
  the host AddressSanitizer probe; each case pins an exit code); ST-14 passes pre-implementation
  (justified: it pins unchanged fall-through and break behavior) ✅ (completed: 2026-10-03 18:30)

> **Red-phase evidence (2026-10-03 18:30)**: the documented host AddressSanitizer probe is the red
> oracle, because the in-container full-suite `RDC_ASAN` run is timing-sensitive in this environment
> (its 30-second timeouts vary between runs while the host probe is deterministic). Host results:
> ST-12 → 20 bytes leaked in 2 allocations; ST-13 → 40 bytes in 4; ST-15 → 40 bytes in 4; ST-14's
> fall-through program exits 3 and its break program exits 2 with no sanitizer report. Plain
> `make check` is green with the new cases (`All 40 suites`), because a plain run cannot observe the
> leaks; the ownership-checked runs are the red signal. A pre-existing ownerless-borrow leak found
> while auditing the ST-30 order case is filed as DEF-67 and the order program now binds its
> results (AR-14).

**Deliverables**:
- [ ] Specification cases ST-12 – ST-15
- [ ] Red-phase evidence recorded

**Verify**: `make check`

---

### Step 2.2: Implementation

**Reference**: [03-03](03-03-do-while-condition-release.md) §Proposed Changes
**Objective**: Attach the condition's materialized releases to the body's early returns.

- [x] 2.2.1 Add the builder mutator that prepends release places (newest-first, deduped) to the
  return terminators reachable from the loop body, and call it in `check_do_while` with the
  checkpoint-sliced releases registered while lowering the condition; reuse `liveness::successors`
  for reachability and record the release events — `compiler/src/ir.rs`, `own/loops.rs` ✅ (completed: 2026-10-03 18:33)
- [x] 2.2.2 Run the spec suite — green phase ✅ (completed: 2026-10-03 18:33)

> **Green-phase evidence (2026-10-03 18:33)**: `make check` exit 0 — all 40 suites green, including
> the specification suite's five do-while cases. The host AddressSanitizer probe now reports no leak
> for the early-return program (exit 2), the two-temporary condition (exit 3), and the nested loops
> (exit 2), while the fall-through (exit 3) and break (exit 2) programs stay clean. The emitted
> early return carries the condition temporary's null-safe release before `return`.

**Deliverables**:
- [ ] Release append implemented
- [ ] Spec suite green

**Verify**: `make check`

---

### Step 2.3: Implementation Tests & Hardening

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) §Implementation Tests
**Objective**: Pin the appended release shape and the nested-loop behavior.

- [x] 2.3.1 Write implementation tests: the return arm carries the condition release; nested
  do-while returns carry both loops' releases; no release is appended outside the body; re-emission
  is byte-identical — `compiler/tests/def_backlog_completion_impl_test.rs` ✅ (completed: 2026-10-03 18:34)
- [x] 2.3.2 Full verification — `make check` ✅ (completed: 2026-10-03 18:34)
- [x] 2.3.3 Tighten the fall-through case so the post-loop return is asserted directly —
  `compiler/tests/def_backlog_completion_impl_test.rs` (RV-302) ✅ (completed: 2026-10-03 19:28)

> **Phase 2 verification evidence (2026-10-03 18:34)**: `make check` exit 0 — all 40 suites green
> (the specification suite at 25/25, the implementation suite at 12/12, and every pre-existing
> suite). The four new implementation cases pin the early return's release, the nested return's
> newest-first pair, the unappended fall-through shape, and byte-identical re-emission.

> **Phase 2 post-phase review (2026-10-03 19:28)**: one correctness reviewer ran on the phase diff
> (`dbc3cee..aac2699`); no critical findings. RV-301 (major — a `return` inside a `defer` block
> registered in the body still leaks the condition temporary, because deferred blocks are not CFG
> successors) is pre-existing and outside the approved reachability mechanism; the user ruled it
> filed as DEF-68. RV-302 (minor — the fall-through case's post-loop return was only indirectly
> asserted) is fixed by task 2.3.3. The reviewer verified the append targets body returns exactly,
> the newest-first order and dedupe, the inert-but-safe release events, zero-initialized
> temporaries, and that `while`/`for`/guard paths are unaffected.

**Deliverables**:
- [ ] Implementation suite for Phase 2
- [ ] All verification passing

**Verify**: `make check`

---

## Phase 3: Closure C spelling (DEF-57, DEF-58)

> **Phase baseline tree**: a316203 (the phase-start commit; the review diff uses `git diff a316203..HEAD`)

### Step 3.1: Specification Tests

**Reference**: [03-04](03-04-closure-c-spelling.md) · [AR-3](00-ambiguity-register.md)
**Objective**: Pin the callable-bound closure spelling and the zero-argument call before implementation.

- [x] 3.1.1 [spec-author] Write specification tests from ST-16 – ST-19 and ST-27 — append to `compiler/tests/def_backlog_completion_spec_test.rs` ✅ (completed: 2026-10-03 19:29)
- [x] 3.1.2 Run the spec suite and record the red phase: ST-16/17 fail the C build (`ClClosure_0`
  unknown); ST-27 emits one collapsed symbol for two enclosing instances; ST-19 fails the C build
  (trailing comma); ST-18 passes pre-implementation (justified: it pins the unchanged non-generic
  spelling) ✅ (completed: 2026-10-03 19:29)

> **Red-phase evidence (2026-10-03 19:29)**: `cargo test --test def_backlog_completion_spec_test`
> → 4 failed of the 5 new cases. ST-16 and ST-27: `unknown type name 'ClClosure_0'; did you mean
> 'ClClosure_0__i32'?` (the callee's recorded argument and the call site disagree). ST-17:
> `cl_apply__String_closure_0` undeclared (the per-enclosing symbols do not match the collapsed
> callee declaration). ST-19: `expected expression before ')' token` from the trailing separator in
> `((f).fn((f).env, ))`. ST-18 passes: the non-generic caller keeps `ClClosure_0` everywhere.

**Deliverables**:
- [ ] Specification cases ST-16 – ST-19 and ST-27
- [ ] Red-phase evidence recorded

**Verify**: `make check`

---

### Step 3.2: Implementation

**Reference**: [03-04](03-04-closure-c-spelling.md) §Proposed Changes
**Objective**: Make the call site and the callee agree, and fix the empty argument list.

- [x] 3.2.1 Relocate `type_key` and its slice helpers to the stage-neutral `types` module; include
  the closure instance in `type_key`'s closure arm and in `dedupe_arg` — `compiler/src/types/`,
  `codegen/types.rs`, `codegen/calls.rs`, `ir/mono.rs` ✅ (completed: 2026-10-03 19:34)
- [x] 3.2.2 Record the enclosing lifted specializations before walking the body, then stamp the
  enclosing instance key on recorded closure type arguments that have a lifted specialization for
  that instance — `own/generics.rs` ✅ (completed: 2026-10-03 19:34)
- [x] 3.2.3 Omit the separator for an empty closure argument list — `codegen/expr.rs` ✅ (completed: 2026-10-03 19:34)
- [x] 3.2.4 Run the spec suite — green phase ✅ (completed: 2026-10-03 19:34)

> **Green-phase evidence (2026-10-03 19:34)**: `cargo test --test def_backlog_completion_spec_test`
> → 30/30 green, and `make check` exit 0 — all 40 suites. The five closure cases run with their
> pinned exits (`41`, `44`, `42`, `5`, `30`); the two closure-only enclosing specializations emit
> `cl_consume__closure_0__i32` and `cl_consume__closure_0__String` as distinct symbols, and the
> zero-argument call renders `((f).fn((f).env))`. Task 3.2.2 also skips generic templates in the
> first walk, because a closure-only argument has no type parameter to mark it unresolved.

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
