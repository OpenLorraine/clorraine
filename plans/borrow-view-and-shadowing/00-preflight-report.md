# Preflight Report: Borrow-View Moves and Shadowed Narrowing

> **Status**: ✅ PASSED — all 4 findings resolved (0 critical, 0 major, 4 minor, 0 observation)
> **Iteration**: 1 (first scan)
> **Artifact**: Implementation plan at `plans/borrow-view-and-shadowing/` (8 documents)
> **Audit target hashes** (content at HEAD `b32b69a`):
> `00-index.md` c11b028 · `00-ambiguity-register.md` 7acbf77 · `01-requirements.md` f3f5512 ·
> `02-current-state.md` 690e3f2 · `03-01-borrow-view-moves.md` 125f560 ·
> `03-02-shadowed-narrowing.md` afa862c · `07-testing-strategy.md` a654b95 ·
> `99-execution-plan.md` 6a88048
> **Codebase Grounded**: 24 source/config files examined, 23 references verified,
> 3 stale/misattributed references found
> **Last Updated**: 2026-09-29
> **Same-session bias**: not applicable — the artifact was created in a prior session
> (commit `b32b69a`); this scan is the first review pass over it.

## Codebase Context Summary

**Tech stack:** Rust workspace (std-only), `clc` compiler + `clorraine` driver; C99
emission; Clorraine corpus and embedded stdlib; Docker SDK for the pinned toolchain.

**Architecture:** staged front end (`lex` → `parse` → `load` → `resolve` → `sema`) feeding
the ownership stage (`compiler/src/own/`) over a typed control-flow IR; the ownership
checker reasons over `Binding` records (`kind`, `mode`, `base`) and a flow state of
`PlaceState` (moved/partial/initialized/narrowed).

**Key files examined**

| File | Relevance |
|------|-----------|
| `compiler/src/own/place.rs` | `move_place` (182-240), `plain_binding` (601), `narrowing` (628) |
| `compiler/src/own/loops.rs` | `bind_for_in_element` (302-367); borrow element is `Local`+`Borrow`+`DerefTemp` (311-338) |
| `compiler/src/own/statements.rs` | `check_let` (178-215), `check_const` (262-287); declaration order |
| `compiler/src/own/expr.rs` | `check_nonnull_expr` (291), `check_value_into` (306), `consume_owned_place` (48), `report_element_move_limit` (89) |
| `compiler/src/own/call_check.rs` | `check_argument_place` take mode (260-282) |
| `compiler/src/own.rs`, `lower.rs`, `program.rs` | binding kinds/modes, captures, callee resolution |
| `compiler/src/own/test_support.rs`, `own/tests.rs` | unit-test harness (`compile`, `reports`, `with_view_helper`) |
| `compiler/tests/array_value_integrity_spec_test.rs` etc. | helper pattern (`ScratchDir`, `emit_host`, `build_host_c`, `run_host`, `assert_rejected_with`, 30 s `RUN_TIMEOUT`) |
| `spec/02`, `spec/03`, `spec/08`, `spec/14`, `spec/16` | the frozen oracle |
| `spec/check.sh`, root `Makefile`, `docker/Makefile`, `AGENTS.md` | verification commands and registry coverage rules |

**Reference verification:** every planned fix site and every claimed move-form funnel
exists. Verified: `move_place` receives `check_argument_place` (take),
`apply_binding_effect` moves, `check_return`, and `consume_owned_place`; `check_value_into`
runs after `declare_local` in both `check_let` and `check_const`; `Local`+`Borrow` is unique
to the loop element view (params are `Param`/`OutSlot`, captures and lambda params are
`Param`); `report_element_move_limit` already emits the exact E10026 strings.

**Live behavior probe (grounding the red phase):** the existing `target/debug/clc` binary
reproduced every recorded prediction — ST-1 – ST-5 accepted (element moved, e.g.
`(*__cl_tmp1) = NULL;` after the take call in the emitted C), ST-6 – ST-8 accepted, ST-9 and
ST-10 rejected with `E10010` on the shadowing initializer, ST-11 `E10010`, ST-12 accepted.
Extra probes: a duplicable element take (`i32`) stays accepted; nested field
`p.inner.name` already reports `E10026`; shadowing a narrowed *parameter* and the
`== null` **else** branch both report `E10010` today.

**Domain lens:** compiler-and-language (grammar/IR/type-checker evidence). Checked semantic
closure for the two rules (diagnostics location, compile-time-only behavior, ownership
semantics, conformance-test ordering); `data-and-migration` does not apply (no persisted
format, no protocol/ABI change); security lens reduces to the creation-time AR-11 decision
(compiler-internal acceptance only).

## Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|------------------|
| 1 | Ambiguities | 0 | — |
| 2 | Implicit Assumptions | 0 | — |
| 3 | Logical Contradictions | 0 | — |
| 4 | Completeness Gaps | 0 | — |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 0 | — |
| 7 | Testability | 1 | 🟡 |
| 8 | Security Blind Spots | 0 | — |
| 9 | Edge Cases | 1 | 🟡 |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 0 | — |
| 13 | Codebase Alignment | 2 | 🟡 |

## Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | — |
| MAJOR | 0 | — |
| MINOR | 4 | 4 resolved |
| OBSERVATION | 0 | — |

---

### PF-001: Guard snippet would compile with a warning and duplicates the E10026 strings 🟡 MINOR

**Dimension:** 13 — Codebase Alignment (Convention Violations)
**Location:** `03-01-borrow-view-moves.md`, section "Proposed Changes" (the guard code
block); also "Testing Requirements" and `99-execution-plan.md` task 1.2.1.
**Codebase Evidence:** the existing borrowed-parameter branch at `compiler/src/own/place.rs:185-196`
clones `name` only because its E10034 message interpolates it; the proposed guard's message
is the registry constant, so the cloned `name` is never read (unused-variable warning). The
identical message/help already lives in `FunctionChecker::report_element_move_limit`
(`compiler/src/own/expr.rs:89-96`), which the aggregate and ownerless paths call.
**The Problem:** executing the snippet literally leaves a dead `let name = binding.name.clone();`.
It produces a Rust warning — the plan's own acceptance criteria require no warnings — and it
becomes a third copy of the E10026 strings inside the ownership stage (DRY violation), while
a single-source helper already exists and is the established pattern for this diagnostic.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Replace the guard body with `self.report_element_move_limit(span); return;` | No unused binding; one source of the message/help; matches the DEF-21 pattern | None material |
| B | Keep the inline `self.error(...)` but drop the `let name` clone | Local to `place.rs` | Third copy of the same strings; helper left unused here |

**Recommendation:** Option A — the helper exists precisely for this diagnostic
(`report_element_move_limit`), so the guard becomes two lines and cannot drift from the
registry text.

**User Decision:** Resolved — User accepted recommendation: Option A.

---

### PF-002: Acceptance criterion 1 is unsatisfiable for R4's borrow-parameter clause 🟡 MINOR

**Dimension:** 7 — Testability
**Location:** `01-requirements.md` R4 and "Acceptance Criteria" item 1;
`07-testing-strategy.md` ST tables; `03-01-borrow-view-moves.md` "Testing Requirements".
**Codebase Evidence:** `spec/tests/compile-fail/E10034_borrow_released.clo` pins the frozen
E10034 behavior, but the plan's acceptance criterion names only
`borrow_view_and_shadowing_spec_test.rs`. The ST list has no E10034 case: ST-6 – ST-8 cover
view reads/lends/writes and the copy-form binding, ST-11/ST-12 cover the null rules. Only
`99-execution-plan.md` task 1.3.1 (implementation tests) claims the E10034 guard.
**The Problem:** R4 bundles four guard behaviors and requires "every requirement R1–R4 traces
to at least one passing specification test case in `borrow_view_and_shadowing_spec_test.rs`".
Three of the four trace; "a move out of a `borrow` parameter keeps reporting `E10034`" does
not. At close-out the executor must either weaken the criterion or add the missing case —
unplanned rework discovered at the acceptance gate.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Add ST-13 (move out of a `borrow` parameter ⇒ `E10034`) to the spec suite | Criterion satisfied as written; locks the regression guard next to the new guard | One more spec case |
| B | Amend acceptance criterion 1 to allow coverage by the existing E10034 golden/impl tests | No new test | Weakens the criterion and the traceability claim for the guard |

**Recommendation:** Option A — the case is one compile-fail assertion with the existing
helpers, and R4's guard is exactly the kind of behavior the new suite should pin.

**User Decision:** Resolved — User accepted recommendation: Option A (add ST-13).

---

### PF-003: Stale and misattributed file/line references 🟡 MINOR

**Dimension:** 13 — Codebase Alignment (Stale Assumptions)
**Location:** `02-current-state.md` §Existing Implementation and §Code Analysis;
`03-02-shadowed-narrowing.md` §Current Architecture; `00-index.md` §Related Files.
**Codebase Evidence:** `check_const` starts at `compiler/src/own/statements.rs:262` and ends
at `:287` (cited as `:256-281` in `02` and `03-02`). `check_let` spans `:178-215`
(cited as `:178-207`; line 207 is `set_place`, the call the plan moves around). `plain_binding`
is defined at `compiler/src/own/place.rs:601` and only *used* in `own/expr.rs:292` (the index
attributes it to `expr.rs`).
**The Problem:** the executor following the line ranges lands 6-8 lines off in the two
functions the plan edits, and the `plain_binding` attribution sends the reader to the wrong
file for the null-narrowing lookup. The specific anchors that carry the reasoning
(`place.rs:182`, `loops.rs:311-338`, `statements.rs:178`) are correct.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Correct the three anchors (`:262-287`, `:178-215`, `place.rs:601`) | Executor reads the right code; dimension-13 hygiene | Doc-only edit |
| B | Leave the ranges (approximate by design) | No edit | Misleads on every re-read; the plan is the executor's only map |

**Recommendation:** Option A.

**User Decision:** Resolved — User accepted recommendation: Option A.

---

### PF-004: Boundary coverage gaps for the authorized behavior 🟡 MINOR

**Dimension:** 9 — Edge Cases (also 7)
**Location:** `07-testing-strategy.md` Phase 1 guards (ST-6 – ST-8) and Phase 2
(ST-9 – ST-12); `01-requirements.md` R2/R4.
**Codebase Evidence:** probes on `target/debug/clc` (2026-09-29):
(1) `if (b == null) { return 1; } else { let b: Buffer = b; ... }` reports `E10010` today —
the mirror branch of the narrowing (`check_if` applies narrowing on both branches,
`own/statements.rs:428-442`) and the same fix must accept it;
(2) `func f(b: Buffer?) { if (b != null) { let b: Buffer = b; ... } }` reports `E10010` today —
the same declaration-order root cause on a parameter owner, not covered by ST-9 (a local);
(3) `for (borrow row in m) { let copy: i32[3] = row; }` is accepted today and must stay
accepted — the guard's boundary on a duplicable copy through a *borrow-form* view, which
ST-8 does not exercise (ST-8 uses the copy-form loop).
**The Problem:** the fix silently generalizes to these boundaries; nothing in the suite pins
them, so a future change to `check_if` narrowing, to the parameter path, or to the
`Local`+`Borrow` guard's scope can regress without a red test. The un-narrowed/`E10012`
boundaries are covered (ST-11, the golden suite).

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Add the `== null` else-branch shadow case as an ST; keep the parameter and borrow-form-copy checks as implementation/unit guards | Pins the true semantic boundary of R2 at spec level; keeps the suite lean | One more ST case |
| B | Add all three as ST cases | Full boundary pinning at spec level | Three cases for one minor rule; some overlap with existing suites |
| C | Accept the gap as recorded risk | No work | Unpinned generalization; invisible regressions |

**Recommendation:** Option A — the `== null` branch is the one distinct semantic branch of
the narrowing rule R2 authorizes; the other two are guard-boundary checks that the
implementation suite can carry.

**User Decision:** Resolved — User accepted recommendation: Option A.

---

## Verdict

**✅ PASSED — all 4 findings resolved (Option A each); 0 critical / 0 major.**

The design is small, correctly grounded in the frozen specification, and every claimed code
anchor, funnel, and red-phase prediction was verified against the live tree and the built
compiler. The four minor findings were documentation, traceability, and coverage-precision
issues.

**Remediation status:** the accepted fixes were **applied on 2026-09-29 12:40** on explicit
user instruction; the scan-time hashes above are superseded by the applied revision below.

**Roadmap:** advanced to `Plan Preflighted` (2026-09-29 12:25).

## Fix Application (post-decision, 2026-09-29 12:40)

| Finding | Fix applied | Files | Applied revision hash |
|---------|-------------|-------|-----------------------|
| PF-001 | Guard snippet now reports through `report_element_move_limit(span); return;` | `03-01` | 7de4074 |
| PF-002 | ST-13 (borrow-parameter `E10034` guard) added to Phase 1 | `07`, `99`, `03-01` | db1adf2 · c68a1eb · 7de4074 |
| PF-003 | `check_let` → `:178-215`, `check_const` → `:262-287`, declaration snippet → `:201-208`; `plain_binding` moved to the `place.rs` line in the index | `02`, `03-02`, `00-index` | e5d920b · 8615fe0 · 1320721 |
| PF-004 | ST-14 (`== null` else-branch shadow) added to Phase 2 | `07`, `99`, `03-02` | db1adf2 · c68a1eb · 8615fe0 |

**Targeted re-check (after edits):**

- All ST references re-grepped: only ST-1 – ST-14 exist; every group reference (phase tasks,
  red-phase table, coverage table) matches the tables; no stale `ST-9 – ST-12`-only range remains.
- The corrected snippet is valid against the tree: `report_element_move_limit` is `pub(super)`
  on `FunctionChecker` in `own/expr.rs` and callable from `own/place.rs`; no dead binding remains.
- The corrected anchors were verified against the tree: `statements.rs:178`, `:215`, `:262`,
  `:287`, `:201-208`; `place.rs:601`.
- Doc-only edits: no behavior, ownership, dependency-ordering, security, or compatibility
  contract changed; no semantic re-scan triggered.
