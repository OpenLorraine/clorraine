# Preflight Report: Non-Lexical Lifetimes (NLL v2)

> **Status**: ✅ PREFLIGHT PASSED — all 12 findings resolved
> **Iteration**: 2 (re-scan after fixes)
> **Previous Iteration**: 11 findings — 11 resolved
> **This Iteration**: 1 new finding (PF-012) — resolved by user choice (Option A)
> **Carried Forward**: none
> **Artifact**: Implementation plan at `plans/non-lexical-lifetimes/` (9 documents: register, index, requirements, current state, three design docs, testing strategy, execution plan)
> **Artifact Revision**: uncommitted; iteration 1 concatenated content sha256 `22b8ed917cdcc1ab`; iteration 2 `26ff3156350e9f03`
> **Codebase Grounded**: 14 source files and 8 spec/golden files examined; 60+ references verified
> **Last Updated**: 2026-09-27 08:58

> **SAME-SESSION REVIEW**: This plan was created in the current session. Same-agent bias
> risk is elevated; an independent challenger subagent reviewed the four critical/major
> findings and confirmed each. Consider a fresh-session audit or a human domain review
> before execution, because the artifact is a borrow-checker soundness change.

## Codebase Context Summary

**Tech Stack:** Rust workspace, std-only; no crates, no build scripts. C99 runtime shims.
**Architecture:** The ownership stage (`compiler/src/own.rs`, `FunctionChecker`) walks each
function body and lowers it to the typed CFG (`ir::Function`) through `Builder` while it
checks. Loans are a lexical set (`compiler/src/borrow.rs`), carried in `FlowState.loans`
(`compiler/src/own/flow.rs`), retired by scope depth, and unioned at joins.
**Key files examined:** `compiler/src/own.rs`, `compiler/src/borrow.rs`,
`compiler/src/own/{flow,statements,place,call_check,lower,expr,test_support}.rs`,
`compiler/src/ir.rs`, `compiler/src/own/tests.rs`, `compiler/tests/{stage_spec_test,compile_fail_spec_test,ownership_drop_spec_test}.rs`,
the six loan goldens under `spec/tests/compile-fail/`, `spec/16-borrow-rules.md`,
`spec/lineage.md`, `spec/decisions.md`, `spec/03-ownership.md`, `spec/14-diagnostics.md`.

**Key observations:**

- The plan's architecture claim holds: the CFG already exists at the end of a function check
  (`compiler/src/own.rs:157,261`), so a post-walk pass needs no second walk.
- `check_defer_exit` (`own.rs:359-377`) re-runs the statement checker with diagnostics on and a
  cloned-then-restored builder. `Builder` is `Clone` and `new_block` allocates positionally
  (`ir.rs:522,582`), so block ids allocated during the discarded check alias later real blocks.
- `check_assign` (`expr.rs:291-322`) calls `write_place` (line 313) before `bind_value` (line 321)
  for whole-binding writes, including view rebinds.
- `borrow.rs:239-270` are `Loans`-type unit tests; `borrow.rs:272-345` are end-to-end checker
  tests via `test_support` helpers.
- The only compile-fail golden that flips under the new model is
  `E10031_view_widening_mutation.clo`; the other five loan goldens and every Rust assertion
  checked keep a use after the conflict.

### Domain lens

The system is a compiler/type checker, so the **compiler and language** lens applies. The
execution plan names it (`99-execution-plan.md`, `Lenses: compiler and language`); the design
docs reason in borrow-checker terms throughout. No separate domain finding.

### Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|-----------------|
| 1 | Ambiguities | 2 (PF-006, PF-011) | 🟡 |
| 2 | Implicit Assumptions | 1 (PF-001) | 🔴 |
| 3 | Logical Contradictions | 0 | — |
| 4 | Completeness Gaps | 2 (PF-001, PF-003) | 🔴 |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 1 (PF-002) | 🟠 |
| 7 | Testability | 1 (PF-007) | 🟡 |
| 8 | Security Blind Spots | 0 (N/A — compiler-internal, AR-18) | — |
| 9 | Edge Cases | 1 (PF-001) | 🔴 |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 3 (PF-005, PF-008, PF-009) | 🟡 |
| 13 | Codebase Alignment | 3 (PF-003, PF-009, PF-011) | 🟠 |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 1 | 1 pending |
| MAJOR | 3 | 3 pending |
| MINOR | 6 | 6 pending |
| OBSERVATION | 1 | 1 pending |

---

## Findings

### PF-001: Events recorded by the exit-edge defer check are not anchored to the defer edges 🔴 CRITICAL

**Dimension:** Implicit Assumptions / Completeness Gaps / Edge Cases
**Location:** `03-02-checker-integration.md` §Recording hooks (the `check_defer_exit` row) and §Post-pass; `03-01-liveness-model.md` §Deferred blocks
**Codebase Evidence:** `compiler/src/own.rs:359-377` (`check_defer_exit` saves a builder **clone**, runs `check_stmt` with `report_diagnostics` still true, then restores the builder); `compiler/src/own/statements.rs:476-509` (`check_defer` lowers the block once with `report_diagnostics = false`); `compiler/src/ir.rs:522,582` (`Builder: Clone`, positional `new_block`).

**The Problem:** The plan says a deferred-block check records a `DeferredUse` in place of a
`LoanUse`, and the post-pass re-anchors `DeferredUse` to `DeferEdge` events. But `check_defer_exit`
runs the entire statement checker with event recording enabled, so an owner access (`Access`), a
loan creation (`Created`/`Request`), or a `Release` (its own `pop_scope`) can also be recorded
there — anchored to positions and block ids of the discarded builder clone. Because block ids are
positional and the clone is restored, those ids are reused by real blocks allocated later. The
post-pass then computes reachability over the final graph using aliased blocks, which can hide a
real conflict. Concretely, a `defer { sink(s); }` inside a loop body that moves `s` on each
iteration, with `observe(v)` in the next iteration, is a true `E10022`; a mis-anchored move event
can drop it. This is a soundness-relevant gap in a borrow checker.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Re-anchor **every** event recorded while a defer check is active to that defer's `DeferEdge` positions (extend the `DeferredUse` rule to all kinds) | Complete; keeps the "defer runs at exit" semantics; no discarded positions survive | Slightly more post-pass logic; needs a per-kind rule for defer-recorded `Access`/`Created`/`Release` |
| B | Record only uses during a defer check; suppress all other event kinds there | Simple; no aliasing | Loses defer owner accesses entirely; can under-report E10022/E10031 for moves/writes in a defer (false negatives) |
| C | Run the defer check against the real deferred block in the final graph instead of a discarded clone | Positions are real | Restructures the existing defer lowering/check split; larger change |

**Recommendation:** Option A — it is the only complete fix that preserves the "defer effects run on the exit edge" semantics and cannot silently drop a conflict. (Option B is not complete; Option C is disproportionate.)

**Related:** AR-8 (defer uses), AR-9 (determinism).
**User Decision:** Resolved — user accepted the recommendation (Option A, "apply all fixes"), 2026-09-27; see Iteration 2 log.

---

### PF-002: `write_place` records a spurious use when a view variable is rebound 🟠 MAJOR

**Dimension:** Feasibility Concerns / Codebase Alignment
**Location:** `03-02-checker-integration.md` §Recording hooks, the `write_place` row
**Codebase Evidence:** `compiler/src/own/expr.rs:313` (`write_place(id, &projections, ...)` before `bind_value` at `:321` for whole-binding writes); `compiler/src/own/place.rs:93-161` (`write_place` runs for empty projections too); `compiler/src/own/statements.rs:240-273` (`bind_value` is the binding/rebind site).

**The Problem:** The hook says "if the binding carries a loan, record `Used` (a write through a
writable view)". `write_place` is also called for a whole-binding rebind of a view variable
(`v = other`, projections empty), which is not an access through the old loan. Recording a `Used`
there extends the old loan's region beyond its real last use. Example: `let v = bytes(s); let w =
bytes(s); observe(v); sink(s); v = w;` — the rebind wrongly revives `v`'s loan past `sink(s)` and
can produce a spurious `E10022`, which contradicts the plan's claim that v2 accepts exactly the
NLL-safe programs. It is over-rejection, not unsoundness.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Record a view `Used` only for a write **through** the loan (projected write into its storage); let `bind_value` update the binding's loan id on a whole-binding write | Exact; matches the model; rebinds handled where provenance already is | Requires the hook to distinguish empty vs projected writes |
| B | Record `Used` for every write of a loan-carrying binding (as written) | No extra code | Over-rejects; contradicts R1/the exactness claim |
| C | Treat a rebind as ending the old loan and starting the new one with an explicit end event | Exact | Adds a new event kind and more machinery than needed |

**Recommendation:** Option A — the smallest change that matches the model; the rebind already flows through `bind_value`, so no new machinery.

**User Decision:** Resolved — user accepted the recommendation (Option A, "apply all fixes"), 2026-09-27; see Iteration 2 log.

---

### PF-003: The removal range deletes valid end-to-end checker tests 🟠 MAJOR

**Dimension:** Completeness Gaps / Codebase Alignment
**Location:** `03-02-checker-integration.md` §Removal (`compiler/src/borrow.rs:239-345`); `99-execution-plan.md` task 2.1.7; AR-10
**Codebase Evidence:** `compiler/src/borrow.rs:239-270` are the `Loans`-type unit tests; `:272-345` are source-compiling checker tests (`a_move_while_a_view_is_live_is_rejected`, `a_write_while_a_view_is_live_is_rejected`, `a_shared_reborrow_of_a_borrow_parameter_is_accepted`, `a_field_view_borrows_the_whole_struct_conservatively`, `two_shared_views_of_one_owner_are_accepted`) using `test_support`.

**The Problem:** The table labels `:239-345` as "`Loans` unit tests", and task 2.1.7 says to
remove the obsolete unit tests. Only `:239-270` test the removed `Loans` type; `:272-345` test the
checker rules and stay green under the new model. Following the plan literally deletes valid
coverage and contradicts AR-10 ("migrate the tests").

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Remove only `:239-270`; keep `:272-345` (they remain valid checker tests) | Preserves coverage with no work | None material |
| B | Remove all `:239-345` and re-create equivalent cases in the new test files | One home for the rules | Duplicates work and risks losing edge cases |
| C | Keep everything | No risk | Leaves the dead `Loans` type tests compiling against a removed type (won't compile) |

**Recommendation:** Option A — correct the range; the `Loans`-type tests must go with the type, the checker tests must stay. (C is infeasible: the `Loans` tests won't compile once the type is removed.)

**User Decision:** Resolved — user accepted the recommendation (Option A, "apply all fixes"), 2026-09-27; see Iteration 2 log.

---

### PF-004: The spec amendment overclaims exact RFC 2094 equivalence 🟠 MAJOR

**Dimension:** Ambiguities / Codebase Alignment
**Location:** `03-03-spec-amendment.md` §`spec/lineage.md` §6 replacement and §`spec/16-borrow-rules.md` §6 closing replacement
**Codebase Evidence:** `03-01-liveness-model.md:141-143` (whole-owner granularity, per-field precision excluded); `:100-103` (a defer with control flow is conservative); `00-ambiguity-register.md` AR-1.

**The Problem:** The proposed wording says the checker "accepts exactly the programs NLL (RFC
2094) accepts". The model deliberately keeps whole-owner loan granularity (field borrows are
conservative), treats a defer containing control flow conservatively, and excludes other NLL
features. The frozen oracle would then claim an exactness the implementation does not have. The
spec is the immutable oracle for future work; an overclaim is a durable accuracy defect.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Reword to "computes last-use regions at the language's whole-owner loan granularity; it accepts the programs the lexical checker rejected but no more precise than its loan granularity allows" | Accurate; keeps the useful claim | Slightly longer |
| B | Keep "exactly the programs NLL accepts" | Short | False as written (field/defer divergences) |
| C | Drop the equivalence claim, describe the model only | Safe | Loses the reader's anchor to RFC 2094 |

**Recommendation:** Option A — state the model and its granularity without claiming exact RFC 2094 equivalence.

**User Decision:** Resolved — user accepted the recommendation (Option A, "apply all fixes"), 2026-09-27; see Iteration 2 log.

---

### PF-005: Testing strategy numbering drift (ST-16 vs ST-17) 🟡 MINOR

**Dimension:** Consistency
**Location:** `03-01-liveness-model.md:164` ("ST-1 – ST-17")
**Codebase Evidence:** `07-testing-strategy.md` defines exactly ST-1 – ST-16; `99-execution-plan.md` uses ST-1 – ST-16.
**The Problem:** The liveness-model doc cites a non-existent ST-17.
**Options:** (A) Correct the range to ST-1 – ST-16. (B) Add an ST-17.
**Recommendation:** Option A — no ST-17 exists and none is needed.
**User Decision:** Resolved — user accepted the recommendation (Option A, "apply all fixes"), 2026-09-27; see Iteration 2 log.

---

### PF-006: `Switch` successor set is ambiguous 🟡 MINOR

**Dimension:** Ambiguities
**Location:** `03-01-liveness-model.md` §The position graph ("every `Switch` target")
**Codebase Evidence:** `compiler/src/ir.rs:398` — `Switch { arms, default, join }`; only `arms` and `default` are real successors; `join` is the post-switch block reached by separate terminators.
**The Problem:** "Every `Switch` target" can be read as arms only or as arms+default+join.
Omitting `default` under-approximates reachability (a use in the default clause could be missed),
which is soundness-relevant; including `join` adds a non-edge.
**Options:** (A) State explicitly: successors are `Goto`, both `Branch` arms, every `Switch` arm and its `default` (never `join`). (B) Leave as is.
**Recommendation:** Option A — one line removes a soundness-relevant ambiguity.
**User Decision:** Resolved — user accepted the recommendation (Option A, "apply all fixes"), 2026-09-27; see Iteration 2 log.

---

### PF-007: The rewritten golden is not verified in Phase 1 🟡 MINOR

**Dimension:** Testability
**Location:** `99-execution-plan.md` task 1.2.3 (`__Verify__: ./spec/check.sh`)
**Codebase Evidence:** `./spec/check.sh` validates chapter shape, links, and code registration; it does not run the compile-fail goldens. The goldens run via `compiler/tests/compile_fail_spec_test.rs` (`cargo test`), which is red through Phase 1 because the new spec tests are red.
**The Problem:** After rewriting `E10031_view_widening_mutation.clo`, the plan's verify command does
not exercise it through `clc`, so "the rewritten golden still reports E10031" is unverified until
Phase 2.
**Options:** (A) Run the single golden through the built `clc` (or `cargo test --test compile_fail_spec_test`) as the task's verification. (B) Rely on Phase 2's `make check`.
**Recommendation:** Option A — verify the oracle edit immediately, independent of the red spec tests.
**User Decision:** Resolved — user accepted the recommendation (Option A, "apply all fixes"), 2026-09-27; see Iteration 2 log.

---

### PF-008: Test-harness citation points at an internal-only helper 🟡 MINOR

**Dimension:** Consistency / Codebase Alignment
**Location:** `07-testing-strategy.md` §Specification Tests ("like `compiler/src/own/test_support.rs` and `compiler/tests/ownership_drop_spec_test.rs`")
**Codebase Evidence:** `compiler/src/own/test_support.rs:17` is `pub(crate)` (unit-test only, `#[cfg(test)]` mod at `own.rs:57`); `ownership_drop_spec_test.rs` drives the built `clc` via `CARGO_BIN_EXE_clc`.
**The Problem:** The new integration test file cannot use `test_support`; citing it can mislead the
spec-author into an uncompilable helper reference.
**Options:** (A) Cite only the integration harness (`ownership_drop_spec_test.rs`, `stage_spec_test.rs`) for the new file, and reserve `test_support` for the in-module unit tests. (B) Leave as is.
**Recommendation:** Option A.
**User Decision:** Resolved — user accepted the recommendation (Option A, "apply all fixes"), 2026-09-27; see Iteration 2 log.

---

### PF-009: Two wrong file:line references 🟡 MINOR

**Dimension:** Consistency / Codebase Alignment
**Location:** `03-01-liveness-model.md:97-98` cites `compiler/src/own/statements.rs:359-377` for the exit-edge check; `:145` cites `compiler/src/own/place.rs:544-574, statements.rs:602-617` for the escape rules
**Codebase Evidence:** The exit-edge check is `compiler/src/own.rs:359-377` (the same doc's sibling `03-02` cites it correctly). `place.rs:544-574` is `elision_prelude`/`elision_postlude`; escape checks are `place.rs:232-244` and `expr.rs:304-312`.
**The Problem:** Mis-targeted references send an executor to unrelated code.
**Options:** (A) Correct both references. (B) Drop the parenthetical line references.
**Recommendation:** Option A.
**User Decision:** Resolved — user accepted the recommendation (Option A, "apply all fixes"), 2026-09-27; see Iteration 2 log.

---

### PF-010: The loop-log truncation instruction is redundant 🔵 OBSERVATION

**Dimension:** Consistency
**Location:** `03-02-checker-integration.md` §Checker state
**Codebase Evidence:** `own.rs:400-401,437` — `loop_entry_state` disables `report_diagnostics` for the whole speculative fixpoint, and the plan gates recording on `report_diagnostics`, so no events are recorded during the speculative pass.
**The Problem:** The plan tells the executor to save/truncate the event log in `loop_entry_state`,
but nothing is recorded there to truncate. Harmless but misleading.
**Options:** (A) Drop the truncation instruction (keep the gate). (B) Keep it as defensive.
**Recommendation:** Option A — the `report_diagnostics` gate already prevents speculative events.
**User Decision:** Resolved — user accepted the recommendation (Option A, "apply all fixes"), 2026-09-27; see Iteration 2 log.

---

### PF-011: The `bind_value` hook omits the loan-creation condition 🟡 MINOR

**Dimension:** Ambiguities / Codebase Alignment
**Location:** `03-02-checker-integration.md` §Recording hooks, the `bind_value` row ("after the `E10041` check: `view_loan(value)` is `None` ⇒ allocate a id and record `Created`")
**Codebase Evidence:** `compiler/src/own/statements.rs:240-263` — `bind_value` records a loan only when `view_provenance` is `Local`/`Param`; `E10041` is checked inside `record_view_loan` (`:276-297`), not in `bind_value`.
**The Problem:** As written, the hook could create a loan for any view with no owner (a static
literal or a global), but `Created { owner: usize }` needs a real owner binding. The correct
condition (provenance `Local`/`Param`) lives in `03-01` but not in the hook table, and the `E10041`
placement is mislabeled.
**Options:** (A) Restate the hook as: "when the value is a view whose provenance is `Local`/`Param`,
create a loan with that owner; when `view_loan(value)` is `Some`, share it" and move the `E10041`
mention to `record_view_loan`. (B) Leave as is.
**Recommendation:** Option A — removes ambiguity that could yield a phantom owner.
**User Decision:** Resolved — user accepted the recommendation (Option A, "apply all fixes"), 2026-09-27; see Iteration 2 log.

---

## Convergence

The iteration-1 severity tables above are the historical first-scan view. After the
user's "apply all fixes" instruction, PF-001 – PF-011 are resolved and verified; PF-012
was resolved by the user's Option A choice and applied. No unresolved 🔴/🟠 remains and no
finding is pending, so the artifact **passes preflight with all 12 findings resolved**.
The current artifact revision is `26ff3156350e9f03`; a later edit to the plan must be
compared against it before execution consumes the pass.

---

## Iteration 2 — Fix Verification and Bounded Re-scan

> **Previous iteration**: 11 findings — 11 resolved by user decision ("apply all fixes").
> **This iteration**: 1 new minor (PF-012).
> **Fixes applied by**: the reviewing agent, on the user's explicit instruction. No
> behavior-bearing code was touched; only the plan documents.

### Decision and fix log

| Finding | Decision | Fix applied | Verified |
|---------|----------|-------------|----------|
| PF-001 | Resolved — user accepted Option A | `03-01` §Ordered events + §Deferred blocks: every event recorded while a defer is checked carries the defer id and is evaluated at the defer's `DeferEdge`s; `03-02` gains `LoanEvent.defer: Option<DeferId>`, drops `DeferredUse`, adds the post-pass resolution step, updates the error-handling row; `02` risk row; `99` tasks 2.1.5/2.1.6; `07` unit-test list | Leftover grep for `DeferredUse`/`DeferUse`/discarded-position wording: clean |
| PF-002 | Resolved — user accepted Option A | `03-02` `write_place` row: `Used` only for a projected write through a view; whole-binding view writes are rebinds handled by `bind_value` | Row present; `bind_value` row states it updates the loan on rebind |
| PF-003 | Resolved — user accepted Option A | `03-02` removal table: `borrow.rs:239-270` only, `:272-345` kept; `02` file row updated | Range is now `:239-270`; no `:239-345` anywhere |
| PF-004 | Resolved — user accepted Option A | `03-03`: both replacement texts reworded to last-use regions at whole-owner granularity, no exact-NLL claim | Grep for "accepts exactly" clean |
| PF-005 | Resolved — user accepted Option A | `03-01` testing line now ST-1 – ST-16 | Grep for `ST-17` clean |
| PF-006 | Resolved — user accepted Option A | `03-01` successor sentence: every `Switch` arm and its `default`, never the `join` block | Sentence present |
| PF-007 | Resolved — user accepted Option A | `99` task 1.2.3 and its Verify line add `cargo test --test compile_fail_spec_test` for the rewritten golden | Task text present |
| PF-008 | Resolved — user accepted Option A | `07` harness paragraph cites the integration harnesses and reserves `test_support` for unit tests | Paragraph present |
| PF-009 | Resolved — user accepted Option A | `03-01`: exit-edge reference is `own.rs:359-377`; escape-rule references corrected | Wrong refs grep clean |
| PF-010 | Resolved — user accepted Option A | `03-02` checker-state paragraph and `99` task 2.1.5 explain the `report_diagnostics` gate instead of truncation; `02` risk row updated | Grep for "save/truncate" clean |
| PF-011 | Resolved — user accepted Option A | `03-02` `bind_value` row states the `Local`/`Param` provenance condition and moves the `E10041` mention to `record_view_loan` | Row present |
| PF-012 | Resolved — user chose Option A | `03-01` event table and rule table now use the `03-02` variant names (`Created`, `Used`, `Access { access: Write/Move }`, `Request`, `Release`) | Old names grep clean |

### PF-012: Event names drift between the model table and the enum variants 🟡 MINOR

**Dimension:** Consistency
**Location:** `03-01-liveness-model.md` §Uses and accesses (event names `LoanCreated`, `LoanUse`, `OwnerAccess::Write`, `OwnerAccess::Move`, `LoanRequest`, `OwnerRelease`); `03-02-checker-integration.md` §New module (`LoanEventKind::{Created, Used, Access{access}, Request, Release, DeferEdge}`)
**The Problem:** The model document and the integration document name the same events
differently, which can confuse the implementer mapping model to code.
**Options:** (A) Align `03-01`'s event names to the `03-02` variants (`Created`, `Used`,
`Access … Write/Move`, `Request`, `Release`, `DeferEdge`). (B) Align the enum to the
model names. (C) Leave as is.
**Recommendation:** Option A — `03-02` owns the code identifiers; the model table should
use them.
**User Decision:** Resolved — user chose Option A, 2026-09-27; fix applied and verified (old names grep clean).

### Regression check

The fixes are documentation-only, so no code compiles differently. Direct consequences
checked: the deferred-resolution rule is now stated consistently in `03-01` (ordered
events, deferred blocks, event table), `03-02` (event struct, hooks, post-pass,
error handling), `02` (risk), `07` (unit tests), and `99` (tasks); the removal range is
consistent across `03-02` and `02`; the spec wording is consistent with AR-1/AR-23 and
`03-01`'s "What does not change". No new contradiction was found in the bounded re-scan
of dimensions 1 – 13 on the changed sections beyond PF-012.
