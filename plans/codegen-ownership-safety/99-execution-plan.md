# Execution Plan: Codegen Ownership Safety

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-28 08:03
> **Progress**: 16/20 tasks (80%)
> **CodeOps Artifact Schema**: 1

## Overview

Fix the borrow-element alias double free (`DEF-13`) and the shadowing
miscompile (`DEF-14`), each with specification tests first, then the lowering
and emitter fixes, then implementation tests. Full details live in the
component documents and the testing strategy.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Borrow element alias (DEF-13) | 9 |
| 2 | Local identity (DEF-14) | 7 |
| 3 | Hardening and close-out | 4 |

**Total: 20 tasks across 3 phases**

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
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

---

## Phase 1: Borrow Element Alias (DEF-13)

> **Phase baseline tree**: _(recorded by the exec-plan skill from a temporary-index snapshot of committed, staged, unstaged, and untracked phase-start state)_
> **Lenses**: compiler and language

### Step 1.1: Specification Tests (RED)

**Reference**: [03-01](03-01-borrow-element-alias.md) §Overview · [07](07-testing-strategy.md) ST-1 – ST-7 · AR #6, AR #7, AR #8
**Objective**: Pin the element-alias behavior with failing run tests before the lowering changes.

- [x] 1.1.1 [spec-author] Write the DEF-13 specification cases ST-1 – ST-7 — `compiler/tests/codegen_ownership_spec_test.rs` ✅ (completed: 2026-09-28 07:32)
- [x] 1.1.2 Verify the RED phase: the new cases fail for the defect reason (double free/abort, wrong value, or C build failure), not a setup error; nested element loops are excluded (`DEF-17`) ✅ (completed: 2026-09-28 07:32)

### Step 1.2: Implementation

**Reference**: [03-01](03-01-borrow-element-alias.md) §Implementation Details · AR #2, AR #11
**Objective**: Alias borrow elements through a per-iteration `raw` temp and delete the copy-back machinery.

- [x] 1.2.1 Add `PlaceBase::DerefTemp { temp, ty }` with a variant unit test; implement rendering/typing (the impl suite pins the emitted shape) — `compiler/src/ir/place.rs`, `compiler/src/ir/tests.rs`, `compiler/src/codegen/expr.rs` ✅ (completed: 2026-09-28 07:36)
- [x] 1.2.2 Alias the borrow element through the temp address (refreshed each iteration) and mark the borrow binding used — `compiler/src/own/loops.rs` ✅ (completed: 2026-09-28 07:36)
- [x] 1.2.3 Delete `ForInWrite`, `set_for_in_write`, `store_for_in_element`, `run_for_in_write`, and the `break`/`continue` hooks — `compiler/src/own/flow.rs`, `compiler/src/own/loops.rs`, `compiler/src/own/statements.rs` ✅ (completed: 2026-09-28 07:36)
- [x] 1.2.4 Skip base-overridden local bindings (`kind == Local && base.is_none()`) in both `Function.locals` builders — `compiler/src/own.rs`, `compiler/src/own/lower.rs` ✅ (completed: 2026-09-28 07:36)
- [x] 1.2.5 Verify the GREEN phase: ST-1 – ST-7 pass and `make check` is green ✅ (completed: 2026-09-28 07:36)

### Step 1.3: Implementation Tests and Hardening

**Reference**: [07](07-testing-strategy.md) §Test Categories
**Objective**: Cover internals and the structural claims.

- [x] 1.3.1 Add implementation tests: no write-back and no spurious alias local; replacement read-back in one iteration; a slice iterable (`import { slice } from std.mem`); a name-collision and keyword-shadow case; a lambda-capture regression; a build-only freestanding compile of a borrow-element program; a run timeout in the suite helper — `compiler/tests/codegen_ownership_impl_test.rs` ✅ (completed: 2026-09-28 07:40)
- [x] 1.3.2 `make check` green after the new tests ✅ (completed: 2026-09-28 07:40)

**Deliverables**:
- [ ] Borrow elements alias the iterable element on every exit path
- [ ] The copy-in/write-back machinery is gone
- [ ] All verification passing

**Verify**: `make check`

---

## Phase 2: Local Identity (DEF-14)

> **Phase baseline tree**: _(recorded by the exec-plan skill from a temporary-index snapshot of committed, staged, unstaged, and untracked phase-start state)_
> **Lenses**: compiler and language

### Step 2.1: Specification Tests (RED)

**Reference**: [03-02](03-02-local-identity.md) §Overview · [07](07-testing-strategy.md) ST-8 – ST-15 · AR #3, AR #9
**Objective**: Pin shadowing behavior for every MOD-16 form before the identity change.

- [x] 2.1.1 [spec-author] Add the DEF-14 specification cases ST-8 – ST-15 — `compiler/tests/codegen_ownership_spec_test.rs` ✅ (completed: 2026-09-28 07:56)
- [x] 2.1.2 Verify the RED phase: the new cases fail for the defect reason (wrong value, segfault, or C build failure) ✅ (completed: 2026-09-28 07:56)

> **Runtime note (2.2.3):** `len` on an owned `String` silently yields `0`
> (a separate defect, filed as `DEF-19`). The ST-9/ST-10/ST-12 programs
> observe length through a `str` helper, which exercises the specification's
> implicit `String`→`str` widening; their expectations are unchanged.

### Step 2.2: Implementation

**Reference**: [03-02](03-02-local-identity.md) §Implementation Details · AR #3, AR #11
**Objective**: Give every binding a unique IR spelling and use it everywhere places and locals are built.

- [x] 2.2.1 Add `Binding.ir_name` and compute it deterministically in `declare`; initialize it in `declare_captures` — `compiler/src/own/flow.rs`, `compiler/src/own.rs`, `compiler/src/own/lower.rs` ✅ (completed: 2026-09-28 08:01)
- [x] 2.2.2 Spell places and both `Function.locals` builders with `ir_name`; update the IR documentation — `compiler/src/own.rs`, `compiler/src/own/lower.rs`, `compiler/src/ir.rs` ✅ (completed: 2026-09-28 08:01)
- [x] 2.2.3 Verify the GREEN phase: ST-8 – ST-15 pass and `make check` is green ✅ (completed: 2026-09-28 08:01)

### Step 2.3: Implementation Tests and Hardening

**Reference**: [07](07-testing-strategy.md) §Test Categories
**Objective**: Cover name uniqueness and determinism.

- [x] 2.3.1 Add the `declare` unit test and the determinism test (compile twice, identical emitted C) — `compiler/src/own.rs`, `compiler/tests/codegen_ownership_impl_test.rs` ✅ (completed: 2026-09-28 08:03)
- [x] 2.3.2 `make check` green after the new tests ✅ (completed: 2026-09-28 08:03)

**Deliverables**:
- [ ] Every MOD-16 shadowing form compiles and behaves correctly
- [ ] Diagnostics keep source names; emitted C names stay deterministic
- [ ] All verification passing

**Verify**: `make check`

---

## Phase 3: Hardening and Close-out

> **Phase baseline tree**: _(recorded by the exec-plan skill from a temporary-index snapshot of committed, staged, unstaged, and untracked phase-start state)_
> **Lenses**: compiler and language

### Step 3.1: Final Verification and Documentation

**Reference**: [00-index](00-index.md) §Minimum-Sufficient Baseline · [01](01-requirements.md) §Acceptance Criteria · AR #5
**Objective**: Prove the defects are gone on the host oracle and close the plan.

- [ ] 3.1.1 Run the host ASan probes per `07` §End-to-End Tests (plain loop, `return`-in-loop, struct write-through ST-3/ST-5, MOD-16, nested shadow, defer-local) and record the results on this line
- [ ] 3.1.2 `make verify` green (all hosted-amiga legs)
- [ ] 3.1.3 Update documentation: `AGENTS.md` test map, roadmap rows for `DEF-13`/`DEF-14` (and file `DEF-16` element-read materialization, `DEF-17` nested element loops, `DEF-18` move through a borrow element), register closure note — `AGENTS.md`, `plans/00-roadmap.md`
- [ ] 3.1.4 Close-out: acceptance criteria checked, plan marked Done

**Deliverables**:
- [ ] ASan hosts probes clean
- [ ] `make verify` green
- [ ] Documentation and roadmap synced

**Verify**: `make verify`

---

## Dependencies

```
Phase 1 (element alias)
    ↓
Phase 2 (local identity)
    ↓
Phase 3 (hardening and close-out)
```

---

## Quality Reviews

### Phase 1 — Borrow Element Alias (2026-09-28)

- Independent correctness review and an independent semantics review ran in
  parallel on the phase diff (baseline `bee8acd0`, through commit `6ed5a15`).
- RV-001 🟡 MINOR: the speculative element-loop pass kept an unused `let id`
  binding, emitting a compiler warning — fixed.
- RV-002 🟡 MINOR: `Binding.base`'s documentation named only lambda captures,
  not the new element alias — fixed.
- The semantics review reported no findings; differential ASan/LSan probes on
  every exit path, replacement, partial moves, defers, and sequential loops
  matched the baseline or improved on it.
- No 🔴/🟠 findings, so no re-review is required.

### Phase 2 — Local Identity (2026-09-28)

- Independent correctness review and an independent semantics review ran in
  parallel on the phase diff (baseline `b4270b97`, through commit `5954d89`).
- RV-001 🔴 CRITICAL: a shadowing initializer's ownership effect still resolved
  after the declaration, so the flow state attributed a move or loan to the new
  binding while the emitted store moved from the outer one. Confirmed repros: a
  valid self-move shadow was rejected; a shadowed move left the outer binding
  dangling (segfault); a moved `borrow` parameter was freed twice.
- **User ruling (2026-09-28 08:15):** fix per the review recommendation — resolve
  the initializer's ownership effect before the declaration, add ST-16 as the
  run-case oracle, and add unit tests for the rejection forms.
- RV-002 🟡 MINOR: a `(MOD-15)` requirement reference leaked into a source
  comment — fixed with the RV-001 follow-up.
- The semantics review confirmed `ir_name` uniqueness and determinism,
  diagnostics keep source names, and no other consumer keys by the wrong
  spelling.
- Re-review (2026-09-28 08:17, scoped to the fix diff `5954d89..821b8a1`):
  RV-001 resolved on every path; the assignment path is byte-identical and no
  new correctness defect was found. RV-003 🟡 MINOR (the ST-16 test name did
  not follow the behavior-when-condition shape) and RV-004 🟡 MINOR (a stale
  `bind_value` summary line) were fixed. A pre-existing diagnostic-only gap
  (`if (x != null) { let x: T = x; }` spuriously reports `E10010`) is filed as
  `DEF-20`.

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check` after every GREEN task, RED tasks failing for the defect reason; `make verify` at the end)
3. ✅ No warnings/errors from the emitted C
4. ✅ No dead code — the copy-in/write-back machinery is removed
5. ✅ Security hardened — not applicable (compiler-internal, AR #15)
6. ✅ Documentation updated — plan documents, `AGENTS.md`, roadmap
7. ✅ Code reviewed — per the repository's strict quality profile
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
