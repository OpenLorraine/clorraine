# Review Report: Non-Lexical Lifetimes (NLL v2)

> **Document**: 00-review-report.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-27 18:12

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
passed; every probe the two reviewers used reports its expected diagnostic or is
accepted: `holeA2e`/`holeCond`/`holeDoWhile`/`holeForC`/`continueCarry`/
`nestedCarry`/`holeForStep`/`rr001`/`e4`/`e5`/`e6`/`e7`/`defere10030` → the
expected `E10030` (`defere10030` now names `s`); `rv001` → `E10022`;
`sa2`/`sa3`/`overR1`/`overR2`/`overR3` → accepted (no over-rejection).

### Fresh review of the follow-up (b961900..2f0f128)

Two independent generic subagents reviewed the follow-up. Verdict: FR-002 is
fixed and no over-rejection was found (a full differential over every tracked
`.clo`, plus 16,000/12,000 differential fuzz programs), but **FR-001 was only
partly closed** and a separate pre-existing critical was found.

| # | Severity | Finding | Status |
|---|----------|---------|--------|
| FR-001b | 🔴 CRITICAL | Both reviewers independently found the same residual: a loop body terminated by `continue` is treated as leaving the loop (`loop_entry_state` collapsed a terminated body to `pre`; `check_continue` recorded no edge state), so a loan created on a continue path was missing from the head and the exit. ASan-confirmed use-after-free; accepted at the pre-fix commit too. | Fixed: `check_continue` records its edge state by target block and `loop_entry_state` joins every `continue` edge into the head; `edge_state` drops the scopes the edge leaves. ST-37 – ST-42 pin the four loop kinds, the conditional continue, and the exit use. |
| NEW-001 | 🔴 CRITICAL | `defer_before_local`: the checker runs **every** defer before recording the scope's releases, but the emitter interleaves drops and defers by reverse registration order. A defer registered before a local it observes is checked as if the local were alive, while the emitted C drops the local first. ASan-confirmed use-after-free; pre-existing (accepted at `b961900`, unchanged by the follow-up). | **Fixed** under the user's ruling: `run_scope_cleanup` sorts a scope's bindings and defers by registration newest first and records them in that order on `pop_scope`, `break`/`continue`, and `return`; `DeferEntry` carries the registration number. `defer_before_local` now reports `E10030`; ST-43 – ST-47 pin the order and the mirror accepted case. |
| FM-004 | 🟡 MINOR | ST-32 (`for` step) and ST-35 are already green at `b961900`, so they are not regression tests for the follow-up; the earlier report's claim that all eight new cases were red is inaccurate. | Corrected here; ST-32/ST-35 retained as coverage/monotonicity guards. |
| FM-005 | 🟡 MINOR | No case pinned the three-clause `for` **condition** back edge (now ST-40). | Added ST-40. |
| FM-006 | 🟡 MINOR | The head reorder raises the `E10020` count from 1 to 2 on a loop whose condition is short-circuited after a body move (still rejected either way). | Report-only. |

### NEW-001 follow-up (user ruling: fix now + review)

The checker now models the emitter's cleanup order: a scope's bindings and
defers are one registration sequence and `run_scope_cleanup` records them newest
first on every exit path. `make check` is green and
`non_lexical_lifetimes_spec_test` has 47 passed. ### Fresh review of the continue + NEW-001 fixes (2f0f128..6dfba87)

Two independent reviewers agreed that the NEW-001 cleanup order now matches the
emitter (checked code against `codegen/cleanup.rs` and probed nested scopes,
multiple defers, and every exit kind) and that the single-path `continue` cases
are closed; ST-37 – ST-42, ST-43, and ST-45 – ST-47 are genuine red regression
tests (ST-44 is the accepted mirror). They also found:

| # | Severity | Finding | Status |
|---|----------|---------|--------|
| F5 | 🟠 MAJOR (regression from `6dfba87`) | Control flow inside a `defer` (`return`/`break`/`continue`) runs the cleanup of the enclosing scopes during the deferred check; with the kept flow state, live bindings were wiped and later uses reported spurious `E10021` (probe `q03_defer_return`). | **Fixed**: `restore_enclosing_state` puts back a binding whose enclosing scope lost it to the deferred block's own control flow, while a binding the defer rewrote/moved/cleared keeps its new state; ST-48 – ST-50 pin the accepted shapes. |
| F1 | 🔴 CRITICAL | The three-clause `for` step is checked with the last body path, not the join of the continue edges; a loan created on a continue path is missing at a step use (ASan-confirmed UAF), and a move in the step never reaches the head. | **Open — ruling requested.** |
| F2 | 🔴 CRITICAL | The `do … while` condition is evaluated with the last body path, so a continue-path loan is missing at the condition use (ASan-confirmed UAF). | **Open — ruling requested.** |
| F3 | 🔴 CRITICAL | The `do … while` exit state uses the last body path; a loan created only on a continue path is lost at the code after the loop (ASan-confirmed UAF). | **Open — ruling requested.** |
| F4 | 🔴 CRITICAL | `loop_exit_state` returns the last body path when the body always terminates with a literal-`true` condition and a break was seen, discarding the joined break edges; an earlier break path's loan is lost (ASan-confirmed UAF). Pre-existing. | **Open — ruling requested.** |
| F6 | 🟠 MAJOR | A defer whose body owns a temporary (e.g. `println($"…")`) panics the compiler: the temp binding is moved to scope 0 and a later cleanup indexes a truncated binding table. Pre-existing. | **Open — ruling requested.** |
| F7 | 🟠 MAJOR | Owned locals declared inside a defer are accepted but emit undeclared C identifiers (the known defer-local codegen gap). Pre-existing. | Open (Phase 3 / separate). |

Gate status: FR-001/FR-001b/FR-002/FM-001/FM-002 are fixed and verified; NEW-001
and F5 are fixed; F1 – F4 and F6 are open pending the user's ruling.
