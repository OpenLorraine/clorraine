# Execution Plan: Defer Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-09 18:18
> **Progress**: 29/32 tasks (91%)
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
> **Expected modification set**: `compiler/tests/defer_integrity_spec_test.rs`, `compiler/tests/defer_integrity_impl_test.rs`, `compiler/src/codegen.rs`, `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/control.rs`, `compiler/src/codegen/function.rs`, `compiler/src/own.rs`, `compiler/src/own/statements.rs`, `plans/defer-integrity/*` · Scope: `strict`
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

- [x] 1.2.1 Emit one copy per cleanup activation: copy-local printed set, per-root copy counter, deterministic label suffix (first copy unchanged) — `compiler/src/codegen.rs`, `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/control.rs`, `compiler/src/codegen/function.rs` ✅ (completed: 2026-10-09 15:43)
- [x] 1.2.2 Keep the deferred entries available across exit edges with an active-entry guard (per-edge effect checking) — `compiler/src/own.rs`, `compiler/src/own/statements.rs` ✅ (completed: 2026-10-09 15:43)
- [x] 1.2.3 Run the spec suite and record the green phase ✅ (completed: 2026-10-09 15:43)

**Green-phase evidence (2026-10-09 15:43)** — the spec suite passes 4/4 (the four cases exit
9/7/21/33 with the sanitizer clean); the full workspace is green with no regressions. Two runtime
discoveries were resolved: the ST-3 expectation program was corrected to make the counter
observable (AR-15, runtime); and the checker's per-edge effect check needed
`close_scope_after_edge` so a block that ended in `return`/`break`/`continue` does not re-run the
scope it already left (the first implementation produced a spurious `E10021` on two NLL spec
cases; the corrected version is green).

**Deliverables**:
- [x] Per-site copies with stable labels
- [x] Per-edge effect checking
- [x] ST-1 – ST-4 green

**Verify**: `make check`

### Step 1.3: Implementation Tests & Hardening

**Reference**: [03-01](03-01-per-site-deferred-copies.md) §Testing Requirements
**Objective**: Pin the copy shapes and prove the memory behavior; review the phase.

- [x] 1.3.1 Write implementation tests for the per-site copy shape, the label spelling (first copy unchanged, later copies suffixed), and byte-identical re-emission — `compiler/tests/defer_integrity_impl_test.rs` ✅ (completed: 2026-10-09 15:44)
- [x] 1.3.2 Run the host-ASan probes for ST-1 – ST-4 and `make check` ✅ (completed: 2026-10-09 15:44)
- [x] 1.3.3 Dispatch the phase review (correctness + memory-safety + emission-integrity; blast-radius and test-integrity steps) and resolve findings; one bounded re-review of a fix diff ✅ (completed: 2026-10-09 16:27)

**Implementation-test evidence (2026-10-09 15:44)** — the three implementation cases pass: the
three-site program spells the deferred increment exactly three times; the label-bearing deferred
copy keeps the plain first-copy labels and suffixes the second copy (`__cl_b5__2:`); and two
emissions are byte-identical. Host-ASan runs (the specification suite's memory cases) are clean,
and `make check` is green. The three finding-regression cases were added with the fixes.

**Phase 1 review findings and fixes (2026-10-09 16:27)** — three reviewers on the phase diff
(correctness, memory-safety, emission-integrity). All findings resolved; the bounded re-review
(effectiveness of the label fix + correctness of the per-edge entry lifetime) confirmed the three
primary findings fixed and surfaced two more, which were fixed immediately with direct evidence
(full workspace + `make check` green; the repros build strict and run correctly in the host and
freestanding profiles). Per the protocol cap, no third review pass was run; the pulled-forward
mechanism is covered by Phase 2's own review.

| ID | Severity | Finding | Resolution |
| -- | -------- | ------- | ---------- |
| PE-001 | 🔴 CRITICAL | `{ defer { break; } }` emitted a duplicate C label: a per-site copy still walked the unbounded successor closure and re-emitted the break target block the main walk prints. | ✅ Fixed structurally by the bounded copy + ambient flow context (AR-16): the deferred copy walks only its own block range and prints `break;`/`continue;` for loop edges. |
| RV-001 | 🟠 MAJOR | `check_switch` and `check_defer_exit` closed a terminated scope with `pop_scope()`, re-running the cleanup the edge already ran and rejecting a valid program with a spurious `E10021`. | ✅ Fixed: both sites use `close_scope_after_edge`; the `dupdiag3`-shape case is accepted and runs (pinned by the impl suite). |
| OBS-2 | 🟠 MAJOR | `run_scope_cleanup` stopped its depth scan at the first deeper entry, so a shallower scope's defers were never effect-checked (missed `E10020`). | ✅ Fixed: the scan collects every matching-depth entry; the moved-value case is rejected again (pinned by the impl suite). |
| PE-002 | 🟠 MAJOR | A guard-stepped `for` (plain branch shape, no ambient break target) with a deferred `break`/`continue` emitted a namespaced `goto` to a label the main walk owns and prints plain. | ✅ Fixed: only blocks inside the active copy's own range take the copy namespace; out-of-range targets keep the main walk's plain spelling (pinned by an impl case; host and freestanding strict builds pass). |
| PE-003 | 🟡 MINOR | An unused `region_start` binding introduced a compiler warning. | ✅ Fixed (removed); the build is warning-free. |
| PRE-1 | 🟡 pre-existing | A `break` out of a non-loop scope that owns cleanup takes a rung while the edge always inlines, emitting an unused `__cl_cN` label (byte-identical at the phase baseline). | Reported; out of this phase's scope. |

**Deliverables**:
- [x] Implementation suite for the copy shapes
- [x] Clean probe matrix
- [x] Phase review with no open critical/major finding

**Verify**: `make check`

---

## Phase 2: Bounded Closure and Ambient Context (DEF-84)

> **Phase baseline tree**: _(pulled forward during Phase 1's fix cycle — see the note below; the phase-start snapshot was taken at Phase 1's start per AR-16)_
> **Expected modification set**: `compiler/tests/defer_integrity_spec_test.rs`, `compiler/tests/defer_integrity_impl_test.rs`, `compiler/src/own/statements.rs`, `compiler/src/ir.rs`, `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/control.rs`, `compiler/src/codegen/function.rs`, `compiler/src/codegen.rs`, `plans/defer-integrity/*` · Scope: `strict`
> **Lenses**: correctness, maintainability, standards; semantics (defer exit edges are a language-rule surface)
> **Reasoning**: high — control-flow edge classification across the copy boundary

> **Pull-forward note (AR-16, 2026-10-09 16:27)**: the Phase 1 review's PE-001 required the bounded
> copy and the ambient flow context to be correct, so Steps 2.1 and 2.2 executed inside Phase 1's
> fix cycle; their evidence is recorded here. Step 2.3 (implementation tests, probes, phase review)
> remains Phase 2's own work.

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-bounded-closure-and-context.md) §Proposed Changes · [AR-6, AR-16](00-ambiguity-register.md)
**Objective**: Pin the strict-build and loop-edge behavior of `continue`/`break` inside a defer.

- [x] 2.1.1 [spec-author] Write specification tests ST-5 – ST-7 in `compiler/tests/defer_integrity_spec_test.rs` ✅ (completed: 2026-10-09 16:27)
- [x] 2.1.2 Run the new suite and record the red phase (the undefined `__cl_b2` label; the swallowed loop; the break variant baseline) ✅ (completed: 2026-10-09 16:27)

**Red-phase evidence (pre-mechanism head `cb0ec2f`)** — ST-5 (do-while continue) hangs; ST-6 (while
continue) already passed as a justified green pin (the per-site copies from Phase 1 fixed that
shape); ST-7 (deferred break) exited 11 with a duplicate `__cl_b3` build failure. All three pass at
the mechanism head (3 / 3 / 22).

**Deliverables**:
- [x] ST-5 – ST-7 in the spec suite
- [x] Red-phase evidence

**Verify**: `make check` (the new cases fail as expected)

### Step 2.2: Implementation

**Reference**: [03-02](03-02-bounded-closure-and-context.md) §Proposed Changes
**Objective**: Give the copy its own region and the ambient flow context; guard rung-hosted
out-edges.

- [x] 2.2.1 Record the deferred block's own block range at lowering and bound the copy to it — `compiler/src/own/statements.rs`, `compiler/src/ir.rs`, `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/control.rs` ✅ (completed: 2026-10-09 16:27)
- [x] 2.2.2 Thread the ambient flow context through the cleanup chain so out-edges classify as `continue`/`break` — `compiler/src/codegen/control.rs`, `compiler/src/codegen/cleanup.rs` ✅ (completed: 2026-10-09 16:27)
- [x] 2.2.3 Add the inline-only guard for scopes whose deferred regions contain break/continue out-edges, and run the rung-hosted sibling verification probe — `compiler/src/codegen/cleanup.rs` ✅ (completed: 2026-10-09 16:27)
- [x] 2.2.4 Run the spec suite and record the green phase ✅ (completed: 2026-10-09 16:27)

**Green-phase evidence (2026-10-09 16:27)** — ST-5/6/7 green; the guard probe (a switch-arm defer
with a `break`) builds and runs; the guard-step `for` repro (PE-002) builds strict in the host and
freestanding profiles and runs correctly; `make check` green.

**Deliverables**:
- [x] Bounded copy with ambient context
- [x] Rung guard and its probe evidence
- [x] ST-5 – ST-7 green

**Verify**: `make check`

### Step 2.3: Implementation Tests & Hardening

**Reference**: [03-02](03-02-bounded-closure-and-context.md) §Testing Requirements
**Objective**: Pin the bounded copy, the edge spelling, and the guard; review the phase.

- [x] 2.3.1 Write implementation tests for the bounded copy (no outer block inside it), the `continue;`/`break;` spelling, and byte-identical re-emission — `compiler/tests/defer_integrity_impl_test.rs` ✅ (completed: 2026-10-09 16:41)
- [x] 2.3.2 Run the host-ASan and strict-build probes for ST-5 – ST-7, the rung guard probe, and `make check` ✅ (completed: 2026-10-09 16:41)
- [x] 2.3.3 Dispatch the phase review (correctness + memory-safety + emission-integrity + semantics) and resolve findings; one bounded re-review of a fix diff ✅ (completed: 2026-10-09 16:41)

**Phase 2 review findings and fixes (2026-10-09 16:41)** — the memory-safety and semantics
reviewers covered the pulled-forward mechanism (the correctness and emission lenses were covered by
the Phase 1 fix re-review on the same diff). The memory audit is clean across the loop forms,
owned locals, nested defers, moves, and the guard-forced inline scopes. The semantics review found
one critical ordering defect, fixed immediately:

| ID | Severity | Finding | Resolution |
| -- | -------- | ------- | ---------- |
| SR-001 | 🔴 CRITICAL | The rung guard inverted the cleanup order when an inner scope kept a rung: a guard-excluded outer scope ran inline before the inner rung, and its deferred `break` preempted the rung jump, skipping the inner cleanup and return store. | ✅ Fixed: an inline scope that owns cleanup flushes the edge's inner rungs inline, innermost first; the ladder emits only rungs an edge (or the chain) reaches. Pinned by the impl suite; the SR-001 shape answers 111. |
| (side effect) | — | The pre-existing PRE-1 unused-rung label (`break` out of a non-loop scope owning cleanup) is naturally resolved by the used-rung emission: the unreachable rung is no longer emitted. | ✅ Verified: the shape builds strict. |
| (regression found during the fix) | 🔴 | The first flush attempt fired for action-less outer scopes, inlining inner rungs everywhere and reintroducing the store-before-release bug (`ai2` and the array suite failed). | ✅ Fixed: the flush runs only for scopes that own cleanup; the full workspace is green. |

**Deliverables**:
- [x] Implementation suite for the bounded copy
- [x] Clean probe matrix (including the rung sibling outcome)
- [x] Phase review with no open critical/major finding

**Verify**: `make check`

---

## Phase 3: Deferred Return Continuation and Ordering (DEF-85, DEF-88)

> **Phase baseline tree**: `e29a748840171b7602130d840e84c4854c5c73a6`
> **Expected modification set**: `compiler/tests/defer_integrity_spec_test.rs`, `compiler/tests/defer_integrity_impl_test.rs`, `compiler/src/codegen.rs`, `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/control.rs`, `plans/defer-integrity/*` · Scope: `strict`
> **Lenses**: correctness, maintainability, standards; semantics (evaluation order and cleanup order)
> **Reasoning**: high — the continuation interacts with the ladder, nested re-entry, and return forms

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-deferred-return.md) §Proposed Changes · [AR-5, AR-8](00-ambiguity-register.md)
**Objective**: Pin the deferred-return values, the remaining-actions behavior, and the
value-before-cleanup ordering.

- [x] 3.1.1 [spec-author] Write specification tests ST-8 – ST-14 in `compiler/tests/defer_integrity_spec_test.rs` ✅ (completed: 2026-10-09 16:52)
- [x] 3.1.2 Run the new suite and record the red phase (the self-loop hangs; the segfaults) ✅ (completed: 2026-10-09 16:52)

**Red-phase evidence (2026-10-09 16:52, baseline `e29a7488`)** — ST-8/9/10/11/12 hang (the
deferred return's self-loop; isolated re-runs confirm), ST-13/14 segfault (the drop prints before
the value read; both reproduced outside the harness). The seven pre-existing tests stay green
(7 passed; 7 failed).

**Deliverables**:
- [x] ST-8 – ST-14 in the spec suite
- [x] Red-phase evidence

**Verify**: `make check` (the new cases fail as expected)

### Step 3.2: Implementation

**Reference**: [03-03](03-03-deferred-return.md) §Proposed Changes
**Objective**: Continue the running cleanup on a deferred return; reorder value returns.

- [x] 3.2.1 Add the running-cleanup frame and the continuation for returning copies (returns inside a copy continue the running cleanup) — `compiler/src/codegen.rs`, `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/control.rs` ✅ (completed: 2026-10-09 17:07)
- [x] 3.2.2 Nested re-entry into a running scope emits only the actions older than the frame's position — `compiler/src/codegen/cleanup.rs` ✅ (completed: 2026-10-09 17:07)
- [x] 3.2.3 For value returns, store the value (and run the terminator's releases) before the trailing inline cleanup, on every return form — `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/control.rs` ✅ (completed: 2026-10-09 17:07)
- [x] 3.2.4 Run the spec suite and record the green phase ✅ (completed: 2026-10-09 17:07)

**Green-phase evidence (2026-10-09 17:07)** — ST-8 – ST-14 all green (spec 14/14; impl 9/9 at
implementation time, 12/12 after the Step 3.3 pins); the full workspace is green; `make check` is
green. The continuation is realized **label-free** (AR-17): the running scope's `ScopeExit` is a
nested re-entry that emits only the remaining actions, and a value return's inline cleanup is
buffered past the value store. A runtime discovery during the green run: an empty deferred-cleanup
emission added a lone `#line` directive and broke the byte pin
`def_backlog_closeout_impl_test.rs:171`; skipping scopes with no remaining actions restored the
pinned bytes.

**Deliverables**:
- [x] Continuation and remaining-actions rule
- [x] Value-before-cleanup ordering
- [x] ST-8 – ST-14 green

**Verify**: `make check`

### Step 3.3: Implementation Tests & Hardening

**Reference**: [03-03](03-03-deferred-return.md) §Testing Requirements
**Objective**: Pin the continuation, the action order, and the return ordering; review the phase.

- [x] 3.3.1 Write implementation tests for the continuation shape, the remaining-actions order, the buffered-cleanup order, and byte-identical re-emission — `compiler/tests/defer_integrity_impl_test.rs` ✅ (completed: 2026-10-09 17:07)
- [x] 3.3.2 Run the host-ASan matrix (release counts and side effects exactly once per path) and `make check` ✅ (completed: 2026-10-09 17:07)
- [x] 3.3.3 Dispatch the phase review (correctness + memory-safety + emission-integrity + semantics) and resolve findings; one bounded re-review of a fix diff ✅ (completed: 2026-10-09 17:31)

**Implementation-test evidence (2026-10-09 17:07)** — three new emission cases pin the store-before-
drop order, the remaining-defer-after-return order, and the outer-rung jump for a nested deferred
return (impl 12/12). Direct host-ASan probes confirm the nested-rung release is clean, and
`make check` is green.

**Phase 3 review findings and fixes (2026-10-09 17:25)** — four reviewers over the phase diff
(`e29a7488..402b94e`). The emission audit is clean (20 intended diffs, every one classified;
determinism 930/930; no duplicate/unused labels). Three reviewers independently found the same
phase-introduced defect, fixed immediately; two bookkeeping findings fixed; several pre-existing
observations reported. Per the protocol cap, one bounded re-review follows this fix.

| ID | Severity | Finding | Resolution |
| -- | -------- | ------- | ---------- |
| RV-001 / SA-001 / SR-001 | 🟠 MAJOR | A deferred return that overrides a pending **owned** return value leaked the superseded value (`__cl_ret` overwritten without releasing the earlier owner); newly reachable because the base hangs. Reproduced for `String`, growable arrays, fixed arrays, structs, moves, and nested cases. | ✅ Fixed: a deferred store now releases the pending result first (`emit_drop_expr` on the live result; gated to deferred copies so ordinary returns stay byte-identical). ASan-clean probes; ST-15 spec case added; impl pin added. |
| RV-002 | 🟡 MINOR | Plan bookkeeping: a duplicate `3.3.3` task line; stale progress count; an imprecise "byte pin" citation. | ✅ Fixed (duplicate removed; progress corrected; citation reworded). |
| RV-003 | 🟡 MINOR | Register bookkeeping: the AR-17 row was glued to AR-16 by a literal `\n`. | ✅ Fixed (separate rows). |
| PRE (emission) | 🟡 pre-existing | Two base-identical strict-build failures (`codegen_run_spec_test__036` unused-but-set array; `ownership_drop_spec_test__019` `RawConsume` coercion). | Reported; out of this plan's scope (backlog decision). |
| OBS (semantics) | — | Pre-existing exponential emit time for many returning defers (base-identical) and emitted-C growth from the nested remaining actions. | Reported; the code-size note is bounded by the per-site copy model (AR-4/AR-17); no behavior defect. |

**Fix evidence (2026-10-09 17:25)** — the leak shapes are ASan clean (String override: exit 1; fixed
array: exit 1); defer suites 15 spec + 13 impl; full workspace green; `make check` green.

**Bounded re-review (2026-10-09 17:31)** — the memory-safety re-review of the fix diff confirms the
leak is resolved across every reproduced form (`String`, growable `Array<i32>`/`Array<String>`,
fixed `String[2]` including conditional/call forms, structs, moves, call temporaries, two-defer
pairs, loop edges, generic `String` substitution) with values unchanged; no double or invalid free
(the release nulls the result, and the guarded-not-taken path keeps the body value); ordinary
non-deferred returns and the host corpus are byte-identical; the fixed-array release addresses the
same `__cl_ret[i]` elements the store writes. One pre-existing, base-identical observation reported
(a growable-slice `len` const-folds to zero) — out of scope.

**Deliverables**:
- [x] Implementation suite for the return family
- [x] Clean probe matrix
- [x] Phase review with no open critical/major finding

**Verify**: `make check`

---

## Phase 4: Plan-wide Hardening and Close-out

> **Phase baseline tree**: `d4f530cc2ed614a9a36cb78b6ab1661241575b0e`
> **Expected modification set**: `compiler/src/**`, `compiler/tests/**`, `AGENTS.md`, `plans/00-roadmap.md`, `plans/defer-integrity/*` · Scope: `strict`
> **Lenses**: all four project review roles over the plan diff
> **Reasoning**: medium — sweeps and the final review

### Step 4.1: Plan-wide Sweeps

**Reference**: [00-index](00-index.md) §Minimum-Sufficient Baseline · [AR-10, AR-13](00-ambiguity-register.md)
**Objective**: Prove the plan-wide emission and memory behavior.

- [x] 4.1.1 Base-vs-head emission sweep over the corpus in every profile and the suite programs; classify every diff against the design and confirm defer-free programs are byte-identical ✅ (completed: 2026-10-09 17:57)
- [x] 4.1.2 Plan-wide host-ASan matrix over all changed shapes and `make verify` (all legs) ✅ (completed: 2026-10-09 17:57)

**Sweep results and fixes (2026-10-09 17:57)** — the emission sweep compared the plan baseline
`01b778fb` with the head over 737 emissions (corpus × 3 profiles + 892 inline suite programs):
every diff traced, 690 are the lone-`#line` cleanup, all 185 rejections byte-identical, strict
builds clean, determinism 930/930 — one MINOR finding (PE-001, below). The memory sweep ran the
plan-wide ASan matrix over every changed shape (multi-site loop/switch/nested defers with owned
locals; all deferred-return forms; guarded-not-taken returns; superseded `String`/growable/fixed/
struct/move/call-temp results; ordering root-cause loops) — one plan-introduced MAJOR leak
(SA-001, below) and one plan-introduced false rejection (`E10021`, below); the rest clean.

| ID | Severity | Finding | Resolution |
| -- | -------- | ------- | ---------- |
| SA-001 | 🟠 MAJOR | A deferred return's stored value, preempted by an older deferred `break`/`continue`, was overwritten by an ordinary store without release (newly reachable). | ✅ Fixed: a per-pass flag set by a deferred result store gates the ordinary store's release (AR-18); ST-16 + impl14 added; `String` and continue-preempt probes ASan clean. |
| `E10021` | 🟠 MAJOR | An older defer reading an owned local after a newer returning defer was falsely rejected (`restore_enclosing_state` restored only strictly-enclosing scopes). | ✅ Fixed: the running scope's dropped bindings are restored too (AR-19); impl15 added; the shape is accepted and answers 7. |
| PE-001 | 🟡 MINOR | Growable-array temporary suffixes renumbered in defer-free programs (the planning pass skipped the ladder's name allocations). | ✅ Fixed: the planning pass walks every rung (AR-20); the minimal repro emits the base suffixes; the real-pass emission is unchanged. |
| PRE (emission) | 🟡 pre-existing | Two base-identical strict-build failures (unused-but-set array; `RawConsume` coercion) and a base-identical deferred-break supersede leak (PRE-1). | Reported; out of scope (backlog decision). |

**Deliverables**:
- [x] Emission sweep with every diff traced
- [x] Memory sweep clean
- [x] `make verify: all legs passed`

**Verify**: `make verify`

### Step 4.2: Final Review

**Reference**: [00-index](00-index.md) · all three component docs
**Objective**: Independent review of the whole plan diff.

- [x] 4.2.1 Dispatch the final plan review (correctness + memory-safety + emission-integrity + semantics over the plan diff) and resolve findings ✅ (completed: 2026-10-09 18:18)
- [~] 4.2.2 Bounded re-review of any fix diff ⏳ (implemented: 2026-10-09 18:18)

**Final review results and fixes (2026-10-09 18:18)** — four reviewers over the whole-plan diff
(`01b778fb..fd923cb`, 3266 lines). No critical or major finding: correctness clean (blast radius,
two-pass determinism, test integrity confirmed), memory clean at HEAD (the SA-001 and E10021 fixes
verified end to end; one pre-existing base-identical leak reported), emission clean (PE-001
verified fixed — the 13 renumbered programs match the base bytes; the `deferred_result_store`
release appears in exactly the four superseding programs; defer-free code unchanged), semantics
consistent (acceptance, order, values) with one minor finding:

| ID | Severity | Finding | Resolution |
| -- | -------- | ------- | ---------- |
| SR-001 | 🟡 MINOR | A deferred block re-checked on every edge and through nested continuations reported the same diagnostic once per check (`dupedges` 3×, `v3` 2×). | ✅ Fixed: only the first check that can report emits diagnostics (a per-registration seen-set, AR-21); `dupedges` 1×, `v3` 1× (the single report closes a base false negative once); pinned by an implementation case. |
| RV-001 | 🟡 MINOR | Plan records said the first copy keeps the plain label spelling and that defer-free programs are byte-identical, contradicting the landed AR-16 suffixing and the accepted lone-`#line` cleanup. | ✅ Fixed: the register, 03-01, 07, and R6 wording now record the supersession and the accepted exception. |
| RV-002 | 🟡 MINOR | The PE-001 stabilization had no regression pin. | ✅ Fixed: an implementation case pins the base local-name numbering for a ladder drop. |
| PRE (memory) | 🟠 pre-existing | A deferred `break`/`continue` preempting a pending **ordinary** owned return leaks the abandoned value (base-identical at the plan baseline). | Reported; out of scope (backlog decision). |

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
