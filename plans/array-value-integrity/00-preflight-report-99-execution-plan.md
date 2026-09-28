# Preflight Report: Array Value Integrity — Execution Plan

> **Status**: ✅ PREFLIGHT PASSED — all 7 findings resolved (2 major, 3 minor, 2 observation); iteration 2 verified
> **Iteration**: 2 (re-scan after accepted fixes)
> **Audit target**: `plans/array-value-integrity/99-execution-plan.md` (single plan document, uncommitted; pre-fix content SHA-256 `63e741b27314dc7021460a50e7a32012896e41916cc0e3cf3aee41a0848b06bd`)
> **Context documents** (read, not audited): `00-index.md`, `00-ambiguity-register.md`, `01-requirements.md`, `02-current-state.md`, `03-01-multi-extent-element-loops.md`, `03-02-array-value-projections.md`, `07-testing-strategy.md`, `plans/00-roadmap.md`, `00-preflight-report.md`
> **Modification set** (confirmed by the user for this fix round): the audit target plus `07-testing-strategy.md` and `03-02-array-value-projections.md`
> **Scope**: Strict (no `--explore-scope`); targeted single-document audit.
> **Codebase Grounded**: 18 source/test/spec files read; ~30 references mapped; 11 live probes with `target/debug/clc` plus the host C compiler
> **Last Updated**: 2026-09-28 17:50

> **REVIEW INDEPENDENCE**: The plan set was created in an earlier session and reviewed
> by a same-session preflight (`00-preflight-report.md`). This scan is a fresh session
> with no creation history, and it re-verified claims with live probes. That earlier
> report's Next Step 3 recommended exactly this fresh-session check.

## Codebase Context Summary

**Repository:** The Clorraine compiler (Rust workspace, std-only; `clc` + `clorraine` driver)
**Tech Stack:** Rust (compiler), C99 (one emitted translation unit plus the profile shim), Clorraine sources; host tests compile emitted C with `-Wall -Wextra -Werror`
**Architecture:** `lex → parse → load → resolve → sema → own → ir → codegen`. `own` owns typing/lowering/ownership; `codegen` renders C. Fixed arrays are `TyKind::Array { element, extents }`; by-value fixed-array returns use a hidden trailing out-parameter. `len` is rendered in `codegen/calls.rs`; element-loop bounds call the same intrinsic (`own/loops.rs:489`). Fixed-array constant folding already uses the outermost extent (`compiler/src/consts/mod.rs:554`).

**Key files examined:** `compiler/src/codegen/calls.rs`, `codegen/function.rs`, `codegen/guards.rs`, `codegen/types.rs`, `codegen/drop.rs`, `compiler/src/own/lower.rs`, `own/loops.rs`, `own/place.rs`, `own/expr.rs`, `own/statements.rs`, `own/call_check.rs`, `own/call.rs`, `compiler/src/sema/stmt.rs`, `compiler/src/types/mod.rs`, `types/table.rs`, `consts/mod.rs`, `compiler/tests/element_loop_integrity_spec_test.rs`, `ownership_drop_impl_test.rs`, `spec/03`, `spec/04`, `spec/08`, `spec/14`.

**Probe evidence (host profile, 2026-09-28):**

| Probe | Program | Result |
| ----- | ------- | ------ |
| p5 | ST-26 shape `s = dup(borrow s);` then `len(s[0]) == 3` | clc accepts; C builds; runs with exit 1 (`len` is 0) |
| p6 | `len(stringOf("hello")) == 5` | exit 1 (`len` is 0) |
| p7 | `len(b[1]) == 3` for `b: String[2]` | exit 1 (`len` is 0) |
| p8 | ST-25 shape `s = mk();` then `len(s[1]) == 2` | exit 1 (`len` is 0) |
| p1 | ST-30 shape `h.s = a[0];` | clc accepts, no `E10026` |
| p2 | `let s: String = a[0];` | `E10026` today |
| p3 | ST-29 shape `h.s = mk()[0];` | clc accepts, no `E10026` |
| p9 | `for (row in m)` over `i32[2][3]` | `E10209` today |
| p10 | `for (borrow row in m)` over `i32[2][3]` | C rejects the emitted C (`i32 *` from `i32 (*)[3]`) |
| p11 | `let b: i32[2] = a;` | C rejects (`assignment to expression with array type`) |

### Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|-----------------|
| 1 | Ambiguities | 0 | — |
| 2 | Implicit Assumptions | 0 | — |
| 3 | Logical Contradictions | 0 | — |
| 4 | Completeness Gaps | 1 | 🟠 |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 0 | — |
| 7 | Testability | 1 | 🟡 |
| 8 | Security Blind Spots | 0 | — |
| 9 | Edge Cases | 0 | — |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 2 | 🟠 |
| 12 | Consistency | 3 | 🟡 |
| 13 | Codebase Alignment | (shared) | 🟠 |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | — |
| MAJOR | 2 | all resolved (iteration 2 verified) |
| MINOR | 3 | all resolved |
| OBSERVATION | 2 | all resolved |

---

## Findings

### PF-001: Phase 1 cannot reach its green phase — four Phase 1 cases need the Phase 3 `len(String)` fix [🟠 MAJOR]

**Dimension:** 11 Ordering & Sequencing · 3 Logical Contradictions · 4 Completeness Gaps
**Location:** `99-execution-plan.md` task 1.2.2 (`:81`), task 1.1.2 (`:67`), task 3.2.3 (`:181`); task 3.1.2 (`:166`); `07-testing-strategy.md` red-phase table (`:98`)
**Codebase Evidence:** `codegen/calls.rs:145-167` renders `len`; an owned `String` handle falls to `_ => "0"` (`:166`). ST-5 (`07:45`), ST-7 (`:47`), ST-25 (`:50`), and ST-26 (`:51`) all assert `len` on an owned `String` element (`len(b[1]) == 3`, `len(s[1]) == 2`, `len(s[0]) == 3`). The String arm is implemented only in Phase 3 task 3.2.3, after Phase 1's green gate. Probes p5/p6/p7/p8: all four shapes run with the wrong value pre-fix (exit 1, `len` = 0); p5's source builds and runs rather than failing the C compiler.
**The Problem:** Task 1.2.2 claims "ST-1 – ST-9, ST-25, ST-26 pass" at the end of Phase 1, and task 1.1.2/`07` record ST-25/ST-26 as passing pre-fix. Both are false: the four cases fail on the `len` value until task 3.2.3 lands. Under `07`'s own rule ("a deviation is an oracle or test defect, never a reason to change a case"), the executor is directed to stop at Phase 1, and the Phase 1 gate cannot pass. The Dependencies diagram does not show this Phase 1 → Phase 3 dependency.
**Related:** `00-preflight-report.md` PF-001 added ST-25/ST-26 and extended task 1.2.1 but did not check their `len(String)` dependency — a same-session blind spot this fresh scan surfaces.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A (recommended) | Move the `len(String)` arm into Phase 1 task 1.2.1 (which already edits the same `len` match for the outer-extent arm); move ST-22 into Phase 1's spec batch so its red phase precedes the implementation; remove/repurpose Phase 3 task 3.2.3; correct the red/green text in `99` and `07` | Phase 1 green becomes reachable; one coherent `len` change; removes the duplicated outer-extent arm in 3.2.3 | Edits to `99` and `07`; Phase 1 absorbs a small part of DEF-19's fix; ST-22's red evidence moves to Phase 1 |
| B | Keep the implementation order; move ST-5, ST-7, ST-25, ST-26 from Phase 1's spec batch into Phase 3's; Phase 1 green covers ST-1–ST-4, ST-8, ST-9 only | Keeps defect isolation by phase | The only spec-level `String[2]` run/release oracles slip to Phase 3; task 2.2.2's "ST-1 – ST-15 pass" and the Phase 2 red text also need edits |
| C | Keep the order; correct only the red/green expectations and accept the four cases as failing until Phase 3 | Smallest wording change | Breaks the non-negotiable green-phase gate; `make check` at task 1.3.2 runs the failing suite |

**Recommendation:** Option A — the `len` arms are one match; implementing both in Phase 1 makes every Phase 1 case pass and removes the duplicate assignment in task 3.2.3. This is also the challenger's independent pick.

**Confidence:** High — the probes and the phase schedule are unambiguous. Confidence would drop if the project requires strict per-defect commit isolation (then Option B with mandatory release probes in Phase 3).
**Challenger:** converged.

**User Decision:** Resolved — User accepted recommendation: Option A. Applied: task 1.2.1 now owns both `len` arms (outer extent and `String` byte count); ST-22 moved into the Phase 1 batch (tasks 1.1.1/1.1.2/1.2.2, `07` Phase 1 table); Phase 3 task 3.2.3 (`len` arms) removed and the Phase 3 tasks renumbered (24 total); red split corrected to "ST-22, ST-25, ST-26 fail on the `len` value" in `99` and `07`; task 4.1.4 progress target 24/24.

---

### PF-002: ST-30 is not covered by the documented `E10026` mechanism [🟠 MAJOR]

**Dimension:** 4 Completeness Gaps · 13 Codebase Alignment
**Location:** `99-execution-plan.md` task 3.2.2 (`:180`), task 3.3.1 (`:195`), task 3.2.4 (`:182`); `03-02-array-value-projections.md` helper contract (`:52-61`) and site table (`:88-93`); `07-testing-strategy.md` ST-30 (`:80`)
**Codebase Evidence:** `is_ownerless_element_move` is documented to be true only for a call-rooted index chain ("a local, parameter, or out-parameter root is a place the move checker already guards"). For `h.s = a[0]` with local root `a`, `place_target` returns `Some(Local(a), [Element])` (`own/place.rs:575-579`), so both the predicate and the site condition "no place source" are false. But the field-target assignment path (`own/expr.rs::check_assign` → `emit_assigned` → `emit_store` → `prepare_store`, `own/statements.rs:223-245`) never applies a binding effect, so nothing calls `move_target`/`move_place` (`own/place.rs:217-229`) and no `E10026` is recorded. Probe p1 confirms `h.s = a[0];` is accepted with no diagnostic today.
**The Problem:** The plan promises ST-30 ("`h.s = a[0];` → E10026") and task 3.2.2 says "include field-target assignment", but the specified predicate cannot produce that diagnostic. Implementing the plan as written leaves ST-30 red at task 3.2.4; the executor must invent a second mechanism (or over-broaden the predicate, risking a double report for `let s: String = a[0];`, which already reports E10026 through `move_target`). The claim that field-target assignment "reaches the same store preparation" is true only in the sense that `prepare_store` runs; it does not record the move.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A (recommended) | At projected-target assignments, after `self.eval(value)`, call the existing `consume_owned_place` (`own/expr.rs:48`) — the same consume point that already yields E10026 for element projections and records legal top-level field moves. Keep `is_ownerless_element_move` for ownerless call roots | Reuses the existing move rule; ST-30 works with no bespoke condition; no double report (ownerless values have no place target, and whole-place targets keep their existing `bind_value` path) | Also records top-level field moves on this path (`h.s = a.field`), a behavior change beyond R4's element-move scope; could surface new E10020/E10025 in programs that currently under-report |
| B | Add a narrow element-only condition at the projected-target assignment sites: report E10026 when the value is a non-duplicable element projection rooted in a place | Strictly stays inside R4; no field-move behavior change | Bespoke condition duplicating `move_place`'s element branch; leaves top-level field moves unrecorded |
| C | Drop ST-30 / accept `h.s = a[0]` as a move | No plan change | Violates R4, `spec/03` §4, and `spec/07` STR-12 (`let e = xs[0]; // E10026: element move`); not viable |

**Recommendation:** Option A — the challenger's refinement: reuse the existing consume point instead of inventing a condition, and update `03-02`'s helper/site table plus tasks 3.2.2/3.3.1. If the user wants to keep strictly inside R4 and leave field-move recording untouched, choose Option B.
**Confidence:** High that the documented mechanism misses ST-30; Med that Option A's field-recording side effect is acceptable without an explicit scope note.
**Challenger:** converged (refined Option A from a bespoke condition to `consume_owned_place`).

**User Decision:** Resolved — User accepted recommendation: Option A. Applied: task 3.2.2 now consumes place values at projected targets through `consume_owned_place`; `03-02` Proposed Changes item 4, the helper contract, the site table (new `check_assign` row), and the error table state the two mechanisms and the partial-move recording side effect; the Phase 3 deliverable names projected-target assignment.

---

### PF-003: Task 3.2.2 instructs writing spec cases during implementation [🟡 MINOR]

**Dimension:** 11 Ordering & Sequencing · 7 Testability
**Location:** `99-execution-plan.md` task 3.2.2 (`:180`); `03-02` "Proposed Changes" item 4 (context)
**Codebase Evidence:** The spec-first ordering is non-negotiable (`_shared/spec-first-ordering.md`): spec tests precede implementation, and a plan must not combine spec tests and implementation in one task. ST-27 – ST-30 are already authored in task 3.1.1 (`99:165`) and run red in 3.1.2.
**The Problem:** "…include field-target assignment and add the matching spec cases" asks the implementation task to add spec cases. Literally followed, it authors expectations after the implementation exists — the prohibited direction — and duplicates 3.1.1's batch.
**Options:** (single viable) Delete "and add the matching spec cases" and, if coverage needs extending after PF-002's mechanism is chosen, add the missing case to task 3.1.1's batch before implementation.
**Recommendation:** Delete the phrase; keep `is_ownerless_element_move` unit coverage in task 3.3.1 (implementation tests, which may follow the code).
**Confidence:** High.

**User Decision:** Resolved — User accepted recommendation. Applied: "add the matching spec cases" removed from task 3.2.2; the unit coverage stays in 3.3.1.

---

### PF-004: Task references resolve to a stale coverage table [🟡 MINOR]

**Dimension:** 12 Consistency · 5 Dependency Issues
**Location:** `99-execution-plan.md` tasks 1.3.1/2.3.1/3.3.1 reference `07 §Test Categories`; `07-testing-strategy.md` spec-suite row (`:106`)
**Codebase Evidence:** `07:106` still reads `ST-1 – ST-24` for `array_value_integrity_spec_test.rs`, while tasks 1.1.1, 2.1.1, and 3.1.1 add ST-1 – ST-9, ST-25, ST-26; ST-10 – ST-15; ST-16 – ST-24, ST-27 – ST-30. The earlier preflight recorded this as fixed ("ST-27 – ST-30 missing from `07`; fixed") but the summary row was not updated.
**The Problem:** The executor reading the referenced table sees a narrower coverage claim than the tasks require.
**Options:** (single viable) Correct the row to `ST-1 – ST-30` (or list the ranges) in `07`. The fix is in a context document, so it requires the user's confirmation of the expanded modification set.
**Recommendation:** Fix `07:106` when applying the PF-001/PF-002 edits (same review pass).
**Confidence:** High.

**User Decision:** Resolved — User accepted recommendation. Applied: `07` spec-suite row now reads `ST-1 – ST-30`.

---

### PF-005: The "counting-deleter" probe mechanism is undefined [🟡 MINOR]

**Dimension:** 7 Testability · 1 Ambiguities
**Location:** `99-execution-plan.md` task 1.3.1 (`:94`) and task 4.1.1 (`:216`); ST-25 (`07:50`) and the `07` E2E table (`:126`)
**Codebase Evidence:** Host tests compile emitted C and run it; existing release oracles use `run_clean`/no-double-free (`ownership_drop_impl_test.rs:110`) or count deleter call sites in the emitted C (`:414`). With `ASAN_OPTIONS=detect_leaks=0` (task 4.1.1), a *missing* release leaks invisibly; a runtime release count has no established helper, and the repo has no counting-deleter pattern.
**The Problem:** "Counting-deleter release programs" and "shows the old elements released exactly once" do not define how the count is observed (emitted-C call-site count vs. instrumented runtime counter). The executor cannot tell whether the planned evidence proves what the wording claims.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A (recommended) | Define the probe as a static count of deleter call sites in the emitted C per case (the pattern that already exists in `ownership_drop_impl_test.rs`) plus run-clean, and reword ST-25/`07` from "released exactly once" to the evidence actually produced | Smallest change; matches repo convention; feasible under `detect_leaks=0` | Does not prove runtime execution count, only that one release site exists |
| B | Build a runtime counting deleter (test-only C support or custom handle) | Proves the runtime count | New test harness/support surface; a complexity escalation requiring explicit approval |

**Recommendation:** Option A.
**Confidence:** High on the gap; Medium on whether Option A satisfies the author's intent.
**Challenger:** not applicable (minor).

**User Decision:** Resolved — User accepted recommendation: Option A. Applied: `99` tasks 1.3.1/4.1.1 and `07` ST-25, the implementation-test description, and the E2E table now define the probe as an emitted-C deleter call-site count plus run-clean.

---

### PF-006: `Lenses` lines use names outside the quality-profile enum [🔵 OBSERVATION]

**Dimension:** 12 Consistency
**Location:** `99-execution-plan.md` phase headers (`:59`, `:108`, `:158`, `:209`)
**Codebase Evidence:** `_shared/quality-profile.md` defines the lens enum (`correctness`, `maintainability`, `standards`, `security`, `perf`, `api-surface`, `concurrency`). The lines name "compatibility" and "compiler and language lens"; `compatibility` maps to `api-surface`. No structured quality profile exists in this flat-layout repo, so the line is informational only. The newest completed plan (`element-loop-integrity`) uses the canonical names.
**The Problem:** Misnamed lenses can confuse the informational mapping; low impact.
**Options:** (single viable) Use enum names (for example `api-surface`; keep `maintainability` implicit as a base lens) when the lines are next edited.
**Recommendation:** Align with the canonical vocabulary.

**User Decision:** Resolved — User accepted recommendation. Applied: all four phase headers now read `correctness, maintainability, standards` with a plain-language parenthetical.

---

### PF-007: Task 4.1.3 says to file `DEF-23`/`DEF-24`, which are already filed [🔵 OBSERVATION]

**Dimension:** 12 Consistency
**Location:** `99-execution-plan.md` task 4.1.3 (`:218`); `plans/00-roadmap.md` rows `DEF-23`/`DEF-24` (`:42-43`)
**Codebase Evidence:** Both defects are already in the roadmap's backlog, filed by the earlier preflight.
**The Problem:** The task reads as if the filings are pending; the executor should verify/keep them instead.
**Options:** (single viable) Reword to "keep the `DEF-23`/`DEF-24` filings current".
**Recommendation:** Reword during the Phase 4 documentation edit.

**User Decision:** Resolved — User accepted recommendation. Applied: task 4.1.3 now says "keep the `DEF-23`/`DEF-24` filings current".

---

## Iteration 2 — Fix Verification

| Check | Result |
|-------|--------|
| All 7 `User Decision` lines recorded; expanded modification set confirmed | ✅ |
| Task count 24 and contiguous numbering (6 + 6 + 7 + 5); progress header `0/24`; task 4.1.4 target `24/24` | ✅ |
| ST-22 scheduled once, in Phase 1 (07 Phase 1 table, 99 tasks 1.1.1/1.1.2/1.2.1/1.2.2, deliverables, Phase 1 impl-test row); absent from Phase 3 batches | ✅ |
| Phase 1 red split matches `07` and the probes: ST-1/ST-22/ST-25/ST-26 wrong `len`; ST-2–ST-5/ST-7/ST-8 C reject; ST-6/ST-9 guards | ✅ |
| Phase 3 no longer claims `len` work (title, objective, references, tasks, deliverables, impl tests) | ✅ |
| PF-002 mechanism consistent across `99` task 3.2.2 and `03-02` (helper contract, Proposed Changes, site table, error table) | ✅ |
| No residual "field-target assignment reaches the same store preparation" claim | ✅ |
| "add the matching spec cases" removed from 3.2.2 | ✅ |
| Deleter-count probe defined consistently in `99` (1.3.1, 4.1.1) and `07` (ST-25, impl-test description, E2E table) | ✅ |
| Test Categories row `ST-1 – ST-30`; all 30 ST cases present exactly once in `07` | ✅ |
| Lenses lines use the quality-profile vocabulary | ✅ |
| `DEF-23`/`DEF-24` filings reworded as kept current | ✅ |
| No new findings introduced by the edits | ✅ (bounded re-scan of every changed section) |

## Roadmap Sync

No lifecycle change: `DEF-15`, `DEF-19`, `DEF-21`, and `DEF-22` were already advanced to
**Plan Preflighted** (🔬) by the earlier set-wide pass, and this targeted re-audit does not
advance a stage. The plan remains ready for `exec-plan`.

---

## Out-of-Scope Context Notes (not findings)

These are pre-existing defects in the code, adjacent to the plan's surface but outside the
authorized requirements and the audit target. No finding is raised; a separate audit and
authorization would be needed before acting on them.

1. **Multi-extent index bounds guard uses the extent product.** `codegen/guards.rs:326-341`
   (`bounds_guard_condition`) checks `index < product(all extents)` for a fixed array, so
   `m[i]` on `i32[2][3]` allows indices 2–5 and reads out of bounds. The plan's `len` change does
   not touch this path; the plan's own tests do not exercise it (element-loop bounds come from
   `len`, which the plan fixes).
2. **`slice_source` uses the extent product for a fixed array** (`codegen/calls.rs:53-69`), so
   slicing a multi-extent fixed array treats it as one flat array. No requirement or ST case in
   this plan covers `slice()` on a multi-extent array.

## Verification Summary

| Check | Result |
|-------|--------|
| Audit target hash recorded (pre-fix) | ✅ `63e741b2…` |
| Every task line appears once; phase table counts 6/6/7/5 = 24 | ✅ |
| Spec-first ordering per feature phase (spec → red → implement → green → impl tests → verify) | ✅ all phases have reachable gates (PF-001 resolved) |
| Every ST case allocated to a phase and file | ✅ (ST-1–30; ST-22 in Phase 1) |
| Task file targets exist / are the right owners | ✅ (see Codebase Context) |
| Prior `00-preflight-report.md` findings carried forward | ✅ 8 resolved there; PF-001/002/004 were new residuals, now resolved |
| Roadmap rows | ✅ DEF-15/19/21/22 at Plan Preflighted; DEF-23/24 filed |
