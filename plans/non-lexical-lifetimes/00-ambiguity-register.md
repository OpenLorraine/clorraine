# Ambiguity Register: Non-Lexical Lifetimes (NLL v2)

> **Status**: ✅ GATE PASSED — all 26 items resolved
> **Last Updated**: 2026-09-27 10:08

## Scope

This plan implements roadmap PLAN-12: replace the version-1 conservative
lexical loan model with non-lexical lifetimes (RFC 2094), so that a loan is
live from its creation to its **last use on any path** rather than to the end
of the scope that owns the derived view. It changes liveness only: the loan
set, its granularity (whole-owner loans, O8), the diagnostics, and every other
ownership rule stay as they are. It excludes per-field loan precision (O8),
aggregate view returns (F2), `Box<T>` (X1), nested partial moves (O4),
Polonius (v3), and any new syntax or diagnostic code.

The frozen specification already stages this upgrade: `spec/16-borrow-rules.md`
section 8 names v2 as "Non-lexical lifetimes: last-use regions and full NLL
(RFC 2094)", and `spec/lineage.md` section 6 pins the target model. The v1
chapters describe the lexical approximation, so this plan amends them to
describe the model v2 implements. One frozen golden flips
(`E10031_view_widening_mutation.clo`: the view's last use is before the
mutation); it is rewritten to a shape that still rejects, and the newly
accepted form is covered by a positive specification test. No other golden
changes: every other rejection fixture keeps a use after its conflict.

## Register

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| AR-1 | Scope | What the plan covers | NLL liveness only, keeping O8/F2/X1/O4/Polonius out; pull a deferred item forward | NLL liveness only; exclusions confirmed 2026-09-27 | ✅ Resolved |
| AR-2 | Scope | Authority to amend the frozen specification | amend spec + rewrite the flipping golden / amend spec + drop the golden / no spec edits (defer the plan) | Amend spec and rewrite the golden, with the wording shown before commit 2026-09-27 | ✅ Resolved |
| AR-3 | Behavioral | The v2 liveness model | reachability-based last-use regions / straight-line last use with a v1 fallback at joins | Reachability-based last-use regions (RFC 2094 shape; branches and loops analyzed) 2026-09-27 | ✅ Resolved |
| AR-4 | Technical | How the region analysis is implemented | post-walk event positions on the emitted CFG / two-pass re-walk / restructure the checker onto a separate IR pass | Post-walk pass over the emitted CFG: ordered events recorded during the existing walk, regions by reachability after it, conflicts and E10030 reported in a deterministic post-pass 2026-09-27 | ✅ Resolved |
| AR-5 | Behavioral | Loan identity and aliases | one loan id per view binding, shared by copies/derivations / one region per owner (union approximation) | One loan id per view value, shared by copies and derived views; a use through any alias extends the same region 2026-09-27 | ✅ Resolved |
| AR-6 | Behavioral | What counts as a use of a loan | any access through a view value, passing, returning, deriving, and defer uses / only direct reads | Grounded in AR-3: every access through a view value is a use, including writable-view writes, derivations, passing, returning, and `defer` uses; the narrower reading would end regions early and is not the confirmed model 2026-09-27 | ✅ Resolved |
| AR-7 | Behavioral | A use and a conflicting access in the same statement | event-order precision (evaluation order decides) / statement granularity (a same-statement use counts as live) | Evaluation-order precision: recorded events are ordered, so a read before a store in the same statement ends the loan before the store 2026-09-27 | ✅ Resolved |
| AR-8 | Behavioral | How E10030 is decided under liveness | reject iff a use is reachable from an owner-release point; report at the loan creation span with the existing message | Reject iff a use is reachable from an owner-release point (scope exit, return cleanup, `break`/`continue` cleanup); report at the loan's creation span with the current message and help 2026-09-27 | ✅ Resolved |
| AR-9 | UX | Where and in what order conflict diagnostics are reported once checks move to a post-pass | deterministic post-pass in event order, same spans and messages / keep inline reporting with a speculative pre-pass | Deterministic post-pass in recorded event order; the same spans, messages, and helps 2026-09-27 | ✅ Resolved |
| AR-10 | Technical | The now-unused lexical `Loans` state and its tests | remove it and migrate the tests / keep it | Remove it and migrate the tests to the liveness module 2026-09-27 | ✅ Resolved |
| AR-11 | Testing | Existing tests whose expectations flip | rewrite each to still reject and add positive NLL tests / reclassify as acceptance with a per-case ruling | Rewrite to keep testing the rule; every change listed 2026-09-27 | ✅ Resolved |
| AR-12 | Testing | Test placement | new dedicated Rust spec + impl files / extend existing suites / add positive goldens under `spec/tests` | New `compiler/tests/non_lexical_lifetimes_spec_test.rs` and `non_lexical_lifetimes_impl_test.rs` plus unit tests; no new goldens 2026-09-27 | ✅ Resolved |
| AR-13 | Technical | Verification commands | `make check` per task, `make verify` before completion / other | `make check` per task and `make verify` before completion 2026-09-27 | ✅ Resolved |
| AR-14 | Naming | Plan folder and roadmap link | `plans/non-lexical-lifetimes/`, `Implements: PLAN-12` / other | `plans/non-lexical-lifetimes/`, `Implements: PLAN-12` 2026-09-27 | ✅ Resolved |
| AR-15 | Edge cases | Loops (use in a later iteration), branch-only uses, early returns, `defer` uses, element-loop borrows, stdlib views, `String` widening, generic instances | behavior follows from the confirmed reachability model; each gets an ST case | Grounded in AR-3; no separate decision | ✅ Resolved |
| AR-16 | Behavioral | The monotonicity obligation | every program accepted under v1 stays accepted; no corpus regression | Grounded in `spec/16-borrow-rules.md:237-241`; a requirement, not a choice | ✅ Resolved |
| AR-17 | Integration points | Interfaces the change affects | internal only: `borrow`, `own`, the ownership tests, and the spec chapters; no CLI, ABI, profile, or emitted-C change | Grounded in the change surface; no decision | ✅ Resolved |
| AR-18 | Security & compliance | External input, injection, auth, data protection | N/A — compiler-internal; the change removes a class of false rejections, and the runtime safety rules are unchanged | N/A | ✅ Resolved |
| AR-19 | Non-functional | Performance, scalability, availability | no target; the analysis is bounded by the function's CFG and event count | N/A | ✅ Resolved |
| AR-20 | Data & state | Data model or persisted format | N/A — no persisted format; the event log is per-function, in memory, and deterministic | N/A | ✅ Resolved |
| AR-21 | Stakeholder conflicts | Competing priorities or permission boundaries | N/A — single maintainer | N/A | ✅ Resolved |
| AR-22 | UX & presentation | Diagnostic wording and locations | unchanged: the same codes, messages, helps, and spans; E10032/E10022/E10031 only fire on fewer programs | Grounded in AR-3; no decision | ✅ Resolved |
| AR-23 | Technical (complexity escalation) | Is the liveness analysis a material support surface? | (a) ordinary feature code reusing the existing walk and IR; (b) full escalation packet | (a) — the analysis is the requested feature itself, not general infrastructure: it adds one focused module over the CFG the checker already builds, no layer, dependency, harness, or cross-cutting refactor | ✅ Resolved |
| AR-24 | Technical (runtime) | Where the loans a view binding carries live (found by the Phase 2 review, RV-001/SA-1: a loan created on one branch was lost at the join, accepting a use-after-free) | carry the loan set in `FlowState` and union it at joins and loop entries / snapshot per-branch binding loans | Carry the loans in `FlowState` as a per-binding set, unioned by the merge; a rebind to a loanless value clears the set. Reuses the v1 flow-state mechanism; no new machinery | ✅ Resolved 2026-09-27 |
| AR-25 | Behavioral (runtime) | The exact `E10030` condition (found by the Phase 2 review, SA-2: the reachable-from-a-release rule rejected a loop-local owner v1 accepted) | reject when a use has no path from its creation that avoids a release / reject when any release reaches a use | Reject when a use has no release-free path from the loan's creation; a straight-line use before the release stays accepted, and a loop-local owner's next-iteration use is reached from a fresh creation, so the R6 monotonicity obligation holds. Amends AR-8; `spec/16-borrow-rules.md` step 7 updated | ✅ Resolved 2026-09-27 |
| AR-26 | Technical (runtime) | How deferred-block events keep their order (found by the Phase 2 review, SA-3: collapsing a defer's events onto one edge position rejected a straight-line defer v1 accepted and lost the owner name) | place each deferred event on the edge's real block in recorded order / tag and re-anchor deferred events to the edge | Place every event a deferred block records on the real block the edge leaves, in recorded order; the block's read/release order is preserved and the release on the edge follows the block's events. Amends the AR-4/AR-6 defer mechanism; removes the defer tag and re-anchoring step | ✅ Resolved 2026-09-27 |

### Resolution Notes

**AR-1:** Roadmap PLAN-12; the deferrals were re-confirmed against
`plans/ownership-drop-completion/01-requirements.md:62-67` and its AR-1.

**AR-2:** The amendment set is `spec/16-borrow-rules.md` sections 1, 6, 7, 8,
and 10; `spec/lineage.md` sections 6 and 10; `spec/decisions.md` row O11;
one wording fix at `spec/03-ownership.md:229`; and the rewrite of
`spec/tests/compile-fail/E10031_view_widening_mutation.clo`. Every other
compile-fail golden keeps its view used after the conflict, so all but this one
stay valid. The exact wording lives in `03-03-spec-amendment.md` and is reviewed
before the commit that lands it.

**AR-3:** The user chose the full model, including the acceptance of a view
whose last use precedes its owner's scope end (the outer-binding case that v1
rejects as E10030).

**AR-11:** The known flip set is exactly one frozen golden; a full-suite sweep
during execution confirms the count. A flipped Rust assertion is rewritten so
it still pins its rule (a use after the conflict keeps it rejected), and the
newly accepted form becomes a positive ST case.

### Resolution Notes (AR-4 – AR-10)

**AR-4 (architecture):** The checker already lowers each body into the typed
CFG (`compiler/src/own.rs:157`, `Builder`) while it checks, so the chosen design
records ordered events during that walk and runs a post-walk reachability pass
on the real graph: no second walk, no new stage, and branches and loops are
exact. The rejected alternative kept conflict reporting inline via a silent
pre-walk; it duplicates the walk and needs an event-to-position mapping for no
observable gain.

**AR-5 (loan identity):** Each view value carries a loan id, shared by copies
and derived views; a use through any alias extends the same region — the
RFC 2094 behavior for a copied reference. The rejected union-per-owner option
is sound but over-rejects a bound-and-unused writable view when a read view is
later used.

**AR-6 (use definition):** A use is any access through a view value: a read
(including `len(v)`, an argument, a comparison), a write through a writable
view, a view derivation (`slice`, `bytes`, member/index of a view), returning a
view, and a use inside a `defer` block (which runs on the scope's exit edges).
Lending an owner to a call is a new call-scoped loan, not a use of a persistent
loan. This follows from the confirmed model; no separate user decision.

**AR-7 (same-statement ordering):** Events are recorded in evaluation order, so
`s = stringOf(v)` (the view is read before the store) is accepted, matching
NLL. Statement granularity would reject it and is not the confirmed model.

**AR-8 (E10030):** Owner-release points are the scope exit, the return cleanup,
and the `break`/`continue` cleanup, where the owner's drop runs. A use
reachable from any release point means the view accesses dead storage; the
diagnostic stays at the loan's creation span with the current message and help
(`spec/14-diagnostics.md:58`) so the frozen golden keeps passing.

**AR-9 (diagnostics):** `E10022`, `E10031`, `E10032`, and the liveness-based
`E10030` are reported in a deterministic post-pass, after the walk's other
diagnostics, in recorded event order; spans, messages, and helps are unchanged.
The fixed-point walk plus a deterministic event order keeps
`determinism_spec_test` green.

**AR-10 (dead code):** Once conflicts and E10030 are decided by regions, the
lexical `Loans` set (`compiler/src/borrow.rs:43-125`), `FlowState.loans`
(`compiler/src/own/flow.rs:113`), the `merge` loan union, `end_scope`, and the
lexical E10030 check (`compiler/src/own/statements.rs:247-260`) become unused
and are removed under the no-dead-code rule; their unit tests are migrated to
the liveness module.

### Resolution Notes (AR-24 – AR-26, Phase 2 review)

**AR-24 (flow-sensitive loans):** The loan must live in the flow state, not on
the binding, because a view assigned one owner on one path and another owner on
another carries both loans after the join. The merge unions the per-binding
loan sets, exactly as the v1 lexical set was unioned; no new layer or
dependency is added. The Phase 1 spec suite covers only loans created before a
branch, so ST-17 – ST-19 were added to pin the join behavior. The single
re-review then found (RR-001) that a loan created in a loop body was dropped at
the loop exit, because the exit state came from the speculative snapshots
(which record no loans); each loop now unions the real body's loans into its
exit state, and `pop_scope` drops a closed scope's loans. ST-22 – ST-27 pin the
four loop kinds and the move/write variants.

**AR-25 (E10030 release-free reachability):** A use is rejected only when every
path from the loan's creation to it crosses a release of the owner. A use before
the release has a release-free path and is accepted, and a loop-local owner's
next-iteration use is reached from a fresh creation, so the outer-scope case
(ST-11) still rejects while the loop-local case (ST-20) accepts. This keeps the
monotonicity obligation of AR-16: every program the lexical checker accepted
stays accepted. `spec/16-borrow-rules.md` step 7 was updated to state the
release-free condition.

**AR-26 (deferred-block anchoring):** The deferred block is checked against a
discarded clone of the graph, so its events are placed at the real block the
edge leaves instead of at a clone position a later real block could reuse. The
events keep the order the block records them. The defer id and the re-anchoring
step are removed from the module.

A later review found that the *relative order of a defer and the scope's
releases* was wrong: the checker ran every defer before every release, while the
emitter interleaves drops and defers by one reverse registration sequence, so a
defer registered before a local was checked as if the local were alive although
the emitted C frees it first (NEW-001, ASan-confirmed). `run_scope_cleanup` now
sorts a scope's bindings and defers by registration newest first and records
them in that order on every exit path (`pop_scope`, `break`/`continue`, and
`return`), matching `emit_actions`; `DeferEntry` carries the registration
number. ST-43 – ST-47 pin the order and the mirror (defer-after-local) accepted
case.

### Final-review follow-up (FR-001, FR-002, FM-001)

The out-of-cycle final review of `77f59e1f..b961900` found the RR-001 fix
incomplete: a loan's id was allocation-ordered, so a speculative pass could not
carry a body-created loan and the loop **entry** (body top, condition, `for`
step) recorded no use (FR-001); and a deferred block's flow effects were
discarded, so a view it rebound to a scope-local owner kept no loan after the
scope (FR-002). Both were ASan-confirmed use-after-frees the pre-change compiler
rejected.

The fix, under AR-24 and AR-26:

* A loan's id is **interned from its origin** (owner, kind, and the derivation's
  source range), so every pass over the derivation agrees on it. The speculative
  loop fixpoint now populates the flow-state loans — one id per derivation site,
  so it stays finite — and a loan created in the loop body is carried at the
  head, in the condition, and after the loop. The exit-only `carry_loans` union
  is removed.
* A deferred block's effect on bindings that outlive its scope is kept; only the
  graph, the binding table, and the scope stack roll back. A `break` or
  `continue` runs the defers it triggers before its edge state is recorded.
* The name table is persistent across a deferred block's truncation, so an
  `E10030` about a defer-local owner names it instead of printing `?` (FM-001).

ST-28 – ST-36 pin the four loop kinds, the condition and step uses, the defer
rebind, the `break` defer, and the owner name. `carry_loans` was confirmed not to
over-reject (a 20,000-program differential and every tracked `.clo` file).

A fresh independent review of that fix then found the loop family still open on
the **`continue`** edge: a body terminated by `continue` was treated as leaving
the loop, so its state never reached the head (a residual FR-001, ASan-confirmed
at both the pre-fix and the follow-up commit). `check_continue` now records its
edge state keyed by the target block, and `loop_entry_state` joins every
`continue` edge into the head while keeping `break`/`return` out; `edge_state`
drops the scopes the edge leaves. ST-37 – ST-42 pin the four loop kinds, the
conditional continue, and the exit use. The same review found a separate,
pre-existing critical in the defer model (NEW-001: the checker ran all defers
before all releases while the emitter interleaves them by reverse registration
order), fixed under the user's ruling: `run_scope_cleanup` records each scope's
bindings and defers in reverse registration order on every exit path, and
ST-43 – ST-47 pin it.
