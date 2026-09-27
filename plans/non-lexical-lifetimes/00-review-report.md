# Review Report: Non-Lexical Lifetimes (NLL v2)

> **Document**: 00-review-report.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-27 11:45

## Phase 1: Specification tests and frozen-spec coherence

- **Phase baseline tree**: `93a099a8c079a4b42bfbbd716707f5e63b1dab85`
- **Scope mode**: strict
- **Diff**: `compiler/tests/non_lexical_lifetimes_spec_test.rs` (new),
  `spec/16-borrow-rules.md`, `spec/lineage.md`, `spec/decisions.md`,
  `spec/03-ownership.md`, `spec/tests/compile-fail/E10031_view_widening_mutation.clo`,
  `plans/non-lexical-lifetimes/99-execution-plan.md`, `plans/00-roadmap.md`
- **Reviewers**: correctness reviewer (independent generic subagent; the specialist
  agent is manual-only, so a generic subagent received the complete dispatch packet)
- **Lenses**: correctness (base); maintainability; standards; domain lens: compiler and language
- **Verdict**: ✅ pass — no critical or major findings

The reviewer reproduced the red phase independently
(`9 passed / 7 failed`; the failing set is ST-1, ST-2, ST-3, ST-8, ST-10, ST-12,
ST-14, exactly as recorded), re-ran `./spec/check.sh` (OK) and
`cargo test --test compile_fail_spec_test` (3 passed, including the rewritten
golden), and confirmed the phase diff touches no compiler source. It traced all
sixteen expectations to the amended chapters, verified the nine Ch 16 edits,
both lineage edits, ledger row O11, and the Ch 03 wording fix against
`03-03-spec-amendment.md`, and confirmed the golden keeps a rejected shape.

The intentionally red acceptance cases are not a defect: they pin the v2
behavior the implementation must satisfy in Phase 2.

| # | Severity | Location | Finding | Ruling |
|---|----------|----------|---------|--------|
| RV-1 | 🟡 MINOR | `compiler/tests/non_lexical_lifetimes_spec_test.rs` module doc | Refers in the present tense to an implementation-test file that lands in Phase 3 | Report-only |
| RV-2 | 🟡 MINOR | test doc comments (branch/scope/evaluation-order cases) | Cite `spec/16` §7 where §1 and `spec/04-expressions.md` are the precise sources | Report-only |
| RV-3 | 🟡 MINOR | `assert_reports_counted` | Pins the expected line count and exit 1, not the absence of extra diagnostics (same as the existing harness) | Report-only |
| RV-4 | 🟡 MINOR | `plans/00-roadmap.md:31` | Changed outside the recorded expected-modification set; standard lifecycle bookkeeping | Report-only |

No re-review was required because no critical or major finding was raised.

## Phase 2: Liveness implementation

- **Phase baseline tree**: `77f59e1f71bbc3aa66bbf84e793e628b1dd172f5`
- **Scope mode**: strict
- **Diff**: `compiler/src/own/liveness.rs` (new), `compiler/src/own.rs`,
  `compiler/src/own/{flow,place,statements,call_check,lower}.rs`,
  `compiler/src/borrow.rs`; fix follow-up adds `compiler/tests/non_lexical_lifetimes_spec_test.rs`,
  `spec/16-borrow-rules.md`, and the plan documents
- **Reviewers**: correctness reviewer and a soundness/semantics auditor
  (independent generic subagents; the specialist agents are manual-only)
- **Lenses**: correctness, maintainability, standards; domain: compiler and language
- **Verdict**: 🔴 CRITICAL and 🟠 MAJOR findings raised; fixed and re-verified after the user's ruling

Both reviewers independently found the same soundness regression and two
monotonicity regressions, and reproduced them against the built `clc` (the
critical one with AddressSanitizer):

| # | Severity | Finding | Ruling |
|---|----------|---------|--------|
| RV-001 / SA-1 | 🔴 CRITICAL | The carried loan sat on the binding and was not merged at joins, so a branch-local rebind erased the other path's loan and a use-after-free was accepted | Fix: carry the loan set in `FlowState` and union at joins (AR-24) |
| SA-2 | 🟠 MAJOR | `E10030` reported a loop-local owner whose per-iteration use follows the previous iteration's release; v1 accepted it | Fix: reject only when a use has no release-free path from its creation (AR-25) |
| SA-3 | 🟠 MAJOR | Deferring every deferred event to one edge position lost the block's order, so a straight-line defer's own read/release rejected and named the owner `'?'`; v1 accepted it | Fix: place deferred events on the edge's real block in recorded order (AR-26) |
| RV-002, RV-003, SA-4, SA-5 | 🟡 MINOR | Stale loan on a loanless rebind (fixed by AR-24); O(n²) ordinal scan and a stale `merge` doc (fixed); defer-local binding-index reuse in diagnostics (report-only) | Report-only / fixed where noted |

Fix verification: `make check` green; `non_lexical_lifetimes_spec_test`
21 passed (the five new regression cases, ST-17 – ST-21, went red first); the
critical probe now reports `E10022`, and the loop-local and defer-local probes
are accepted. A single re-review of the fix diff was run (see below).

### Phase 2 fix re-review

Verdict: the three named findings are fixed, but the re-review found one
residual 🔴 CRITICAL:

| # | Severity | Finding | Ruling |
|---|----------|---------|--------|
| RR-001 | 🔴 CRITICAL | A loan created in a loop body was absent from the post-loop flow state, because the loop exit state came from the speculative fixpoint snapshots (which record no loans); a view of a loop-local owner used after the loop was accepted (ASan-confirmed use-after-free), and a move/write of an outer owner borrowed only in the loop was missed | Fixed under AR-24: each loop unions the real body's loans into its exit state, and `pop_scope` drops a closed scope's loans; ST-22 – ST-27 pin the four loop kinds and the move/write variants |

The re-review re-confirmed the join fix, the release-free `E10030`, the
deferred anchoring, the loop fixpoint termination, and the removal of the defer
tag. Per the quality loop, a fix rejected by the single re-review is reported
rather than re-reviewed again (no third pass). The follow-up fix is verified by
`make check` (green) and `non_lexical_lifetimes_spec_test` (27 passed); the new
cases were validated against the pre-fix behavior reproduced by the re-review
with `clc` and AddressSanitizer.

### Phase 2 final review (out-of-cycle, user-requested)

- **Diff**: `77f59e1f..b961900` (the whole Phase 2 change, including the RR-001 fix)
- **Reviewers**: two independent generic subagents — one soundness (false
  accepts), one conformance/monotonicity (false rejects and test quality)
- **Verdict**: 🔴 two CRITICAL soundness holes remain; the RR-001 fix is a
  partial fix of a larger class. `carry_loans` introduced no over-rejection
  (confirmed by a 20,000-program differential run and every tracked `.clo`).

| # | Severity | Finding | Status |
|---|----------|---------|--------|
| FR-001 | 🔴 CRITICAL | The loop **entry/back-edge** state is built speculatively, so it never carries loans created in the body. The one real body pass starts from that loan-free state, so a use at the top of the body (or in the condition) records no use and no `E10030`. RR-001 fixed only the **exit**; this is the same mechanism on the entry edge. ASan-confirmed use-after-free; `clc` accepts. | Open |
| FR-002 | 🔴 CRITICAL | Flow effects inside a `defer` are discarded (`check_defer_exit` restores `saved_state`), so a view widened inside the defer stays loan-free; a later use of the escaped view records no use and no `E10030`. ASan-confirmed use-after-free; v1 rejected it. | Open |
| FM-001 | 🟡 MINOR | `E10030` prints `'?'` for an owner declared inside a `defer` (defer-local binding-index reuse; SA-5, still open) | Open |
| FM-002 | 🟡 MINOR | `assert_reports_counted` does not assert the absence of extra diagnostics (same as the existing harness and RV-3) | Open |
| FM-003 | 🟡 MINOR | The spec-test module doc names a Phase 3 implementation-test file that does not yet exist; `liveness.rs` has no unit tests; accepted programs are never built/run, which is why FR-001/FR-002 escaped `make check` | Open (Phase 3) |
| FI-001 | ℹ️ INFO | ST-17 – ST-27 pass at the pre-change tree too (they guard the intermediate v2 trees, not the baseline) | Accepted |
| FI-002 | ℹ️ INFO | A `let` inside a `defer` emits an undeclared C local (pre-existing, unchanged by this diff) | Accepted |

Both criticals were reproduced at HEAD by this review: `clc` accepts each probe
(exit 0), and the emitted C run under AddressSanitizer reports
`heap-use-after-free`. Neither case is in ST-1 – ST-27, so `make check` stays
green. The RR-001 fix is therefore correct but incomplete, and the quality gate
remains open until FR-001 and FR-002 are fixed and re-reviewed.

### Phase 2 final-review follow-up (user ruling: fix criticals and minors)

The two criticals and the two open minors were fixed after the user's ruling:

| # | Fix |
|---|-----|
| FR-001 | A loan's id is interned from its origin (owner, kind, derivation source range), so the speculative loop fixpoint and the reporting pass agree on it; the fixpoint now populates the flow-state loans, so a loan created in a loop body is carried at the head, in the condition, and after the loop. The three-clause `for` and `while` check their condition with the loop-entry state, and the exit-only `carry_loans` union was removed. |
| FR-002 | `check_defer_exit` keeps the flow state (only the graph, binding table, and scope stack roll back), so a defer's rebind of an outer view persists after the scope; a `break`/`continue` runs its defers before the edge state is recorded. |
| FM-001 | A persistent `binding_names` table outlives a deferred block's truncation, so `E10030` names a defer-local owner. |
| FM-002 | `assert_reports_counted` rejects any diagnostic not listed in the expectation and pins the total error count. |

Verification (all green): `make check`; `non_lexical_lifetimes_spec_test` 36
passed (the eight new cases ST-28 – ST-36 were red against the reviewer's probes
at HEAD, and the shapes were ASan-confirmed use-after-frees there); every probe
the two reviewers used now reports its expected diagnostic or is accepted:
`holeA2e`/`holeCond`/`holeDoWhile`/`holeForC`/`continueCarry`/`nestedCarry`/
`holeForStep`/`rr001`/`e4`/`e5`/`e6`/`e7`/`defere10030` → the expected `E10030`
(`defere10030` now names `s`); `rv001` → `E10022`; `sa2`/`sa3`/`overR1`/`overR2`/
`overR3` → accepted (no over-rejection). Because the code changed after this
review, a fresh independent review of the follow-up diff is required before the
gate closes; it is dispatched next.
