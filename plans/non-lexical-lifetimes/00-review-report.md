# Review Report: Non-Lexical Lifetimes (NLL v2)

> **Document**: 00-review-report.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-27 10:08

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

Pending.
