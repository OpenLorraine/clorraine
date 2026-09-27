# Checker Integration: Non-Lexical Lifetimes

> **Document**: 03-02-checker-integration.md
> **Parent**: [Index](00-index.md)

## Overview

This document owns how the compiler implements the model of `03-01`: the new
module, the event log and its recording hooks, the post-pass, the removal of
the lexical state, and the wiring into the existing stage. The checker
already lowers every body into the typed CFG while it checks
(`compiler/src/own.rs:157`), so no second walk and no new stage are needed
(AR-4).

## Architecture

### New module

`compiler/src/own/liveness.rs` (new, `mod liveness;` in `compiler/src/own.rs`):

```rust
/// Identity of one loan inside one function.
pub(super) struct LoanId(usize);

/// Identity of one deferred block inside one function.
pub(super) struct DeferId(usize);

/// A point in the finished function's event sequence.
pub(super) struct Position {
    pub(super) block: ir::BlockId,
    pub(super) index: usize,
}

/// What one event records.
pub(super) enum LoanEventKind {
    Created { loan: LoanId, owner: usize, kind: LoanKind },
    Used { loan: LoanId },
    Access { owner: usize, access: AccessKind },
    Request { owner: usize, kind: LoanKind },
    Release { owner: usize },
    DeferEdge { defer: DeferId },
}

pub(super) enum AccessKind { Write, Move }

/// One recorded fact at one position.
pub(super) struct LoanEvent {
    /// Where the event was recorded; ignored when `defer` is set.
    pub(super) position: Position,
    /// The deferred block the event belongs to, or `None` for ordinary flow.
    pub(super) defer: Option<DeferId>,
    pub(super) span: Span,
    pub(super) kind: LoanEventKind,
}

/// Builds the position graph and reports E10022, E10031, E10032, and E10030.
pub(super) fn check(
    function: &ir::Function,
    events: &[LoanEvent],
    names: &[String],
    diagnostics: &mut Diagnostics,
);
```

The pure analysis is factored so unit tests can drive it without the walk:
`successors(function)`, `regions(events, &successors)` (forward reachability
from each creation ∩ backward reachability from each use), and
`report(...)`.

### Checker state

* `FunctionChecker.events: Vec<LoanEvent>` — the log of the current function
  or lifted lambda.
* `FunctionChecker.next_loan: usize`, `next_defer: usize` — id counters.
* `FunctionChecker.current_defer: Option<DeferId>` — set while a deferred block
  is checked.
* `Binding.loan: Option<LoanId>` (`compiler/src/own/flow.rs:30`) — the loan a
  view binding carries, or `None`.

Events are recorded only while `report_diagnostics` is true, which already
excludes the speculative loop fixpoint (`loop_entry_state` sets it false for
the whole pass, `compiler/src/own.rs:389-439`). `lift_lambda`
(`compiler/src/own/lower.rs:311-397`) takes the log, checks the lifted
function with it, and restores the saved log.

### Loan identity resolver

`view_loan(&self, expr: &Expr) -> Option<LoanId>` mirrors `view_provenance`
(`compiler/src/own/place.rs:249-276`) on the loan chain:

| Expression | Result |
| ---------- | ------ |
| `Name` of a view binding | that binding's `loan` |
| `Qualified` whose head is a view binding | that head's `loan` |
| `Member` / `Index` | the base's loan |
| `Unsafe` / `RawCast` / `Cast` | the inner value's loan |
| a view-returning call | the loan of the view value at the elision-candidate argument |
| an owner, a literal, or anything else | `None` |

## Implementation Details

### Recording hooks

| Site | Event recorded |
| ---- | -------------- |
| `bind_value` (`compiler/src/own/statements.rs:240-273`) | for a view binding whose `view_provenance` is `Local`/`Param`: `view_loan(value)` is `None` ⇒ allocate an id, record `Created { owner }`, and store the id on the new binding; `Some(source)` ⇒ record `Used { source }` and store the source id. A whole-binding rebind updates the loan the same way; `E10041` is checked in `record_view_loan` |
| `record_view_loan` (`compiler/src/own/statements.rs:276-297`) | keep the inline `E10041` check; record `Request`; the inline `E10032` check and `Loans::add` are removed |
| `read_place` (`compiler/src/own/place.rs:18-68`) | if the read binding carries a loan, record `Used`, tagged with the active defer id while `current_defer` is set |
| `write_place` (`compiler/src/own/place.rs:93-161`) | after the const/read-only checks: for a projected write through a view binding, record `Used` of its loan; for a non-view binding, record `Access { owner: id, access: Write }`; a whole-binding write of a view binding is a rebind, so no use is recorded here and `bind_value` updates its loan |
| `move_place` (`compiler/src/own/place.rs:170-229`) | after the `E10034` check: record `Access { owner: id, access: Move }`; the `blocks_move` check is removed |
| `check_argument_place` Mode::Borrow (`compiler/src/own/call_check.rs:278-343`) | keep `E10038`, `E10041`, `W10001`, and the duplicate-`lent` check; replace `loans.conflict` with `Request { owner, kind: Exclusive }` |
| any call argument | if `view_loan(expr)` is `Some`, record `Used` |
| `check_return` (`compiler/src/own/statements.rs:592-626`) | if the returned view value carries a loan, record `Used` |
| `pop_scope` (`compiler/src/own.rs:314-339`) | `Release` for every binding of the scope being left; `end_scope` is removed |
| `emit_exit_cleanup` (`compiler/src/own/statements.rs:561-585`) | `Release` for every binding of each scope deeper than `floor` |
| `emit_return_cleanup` (`compiler/src/own.rs:706-728`) | `Release` for every binding of every open scope |
| `run_defers_from` (`compiler/src/own.rs:347-356`) | `DeferEdge { defer }` for the entry about to run |
| `check_defer_exit` (`compiler/src/own.rs:359-377`) | set `current_defer` for the whole block check, so every recorded event carries the defer id (not only uses); restore after |
| `DeferEntry` (`compiler/src/own/flow.rs:202-207`) | gains `id: DeferId`, assigned at registration |

`Release` events are recorded for every binding in a leaving scope. Loans only
ever name real storage owners (owners reached through the provenance chain),
so releases of view bindings are inert.

### Post-pass

After `builder.finish()`:

1. Build the successor lists from each block's terminator.
2. Group events by block in recorded order and build the position graph
   (`03-01`).
3. Resolve deferred events: an event with `defer: Some(id)` is evaluated at
   every `DeferEdge { defer: id }` position, in recorded order; its own
   position is ignored, so no event ever depends on a discarded block.
4. For each loan, mark forward-reachable positions from `Created` and
   backward-reachable positions from every use; the region is the
   intersection.
5. Walk the events in order, at each resolved position:
   * `Access::Move` with a live loan of the owner ⇒ `E10022`;
   * `Access::Write` with a live **shared** loan ⇒ `E10031`;
   * `Request` conflicting with a live loan (`loan.kind == Exclusive ||
     request.kind == Exclusive`) ⇒ `E10032`;
   * for each `Used`, if an `OwnerRelease` of the loan's owner reaches the use
     position ⇒ mark the loan for `E10030`.
6. Report `E10030` once per marked loan at its creation span, in loan-id order.
7. Conflicts report at the event span with the v1 messages and helps
   (`compiler/src/own/place.rs:118-123`, `compiler/src/own/statements.rs:289-293`,
   `compiler/src/own/call_check.rs:325-329`).

### Wiring

* `FunctionChecker::check()` (`compiler/src/own.rs:246-273`): after
  `builder.finish()` and the locals/temps are set, call
  `liveness::check(&function, &self.events, &names, self.diagnostics)` with
  `names` derived from the current bindings.
* `lift_lambda` (`compiler/src/own/lower.rs:367-384`): after the lifted
  function is finished, run the same call while the lambda's bindings and
  events are still installed.

## Removal of the lexical state (AR-10)

| Removed | Location |
| ------- | -------- |
| `Loan`, `Loans` and their methods | `compiler/src/borrow.rs:27-125` |
| `Loans` unit tests | `compiler/src/borrow.rs:239-270` (the checker tests at `:272-345` and the view-predicate tests at `:347-365` stay) |
| `FlowState.loans` | `compiler/src/own/flow.rs:109-114` |
| Loan union in `merge` | `compiler/src/own.rs:638-640` |
| `end_scope` call | `compiler/src/own.rs:338` |
| Lexical `E10030` in `bind_value` | `compiler/src/own/statements.rs:247-260` |
| Inline `E10031` / `E10022` checks | `compiler/src/own/place.rs:116-125`, `:185-194` |
| Inline loan conflict at request sites | `compiler/src/own/statements.rs:287-295`, `compiler/src/own/call_check.rs:320-331` |

`LoanKind` (`compiler/src/borrow.rs:20-25`), `Provenance`, and the view
predicates stay in `borrow.rs`. The dormant `ir::LoanKind` and
`Statement::Borrow` scaffolding is untouched (AR-17).

## Integration Points

| Interface | Effect |
| --------- | ------ |
| `own::check` output (`OwnershipOutput`) | unchanged: the same `ir::Function` list and monomorphization inputs |
| `codegen` | unchanged — no emitted-C change |
| Driver pipeline (`compiler/src/driver.rs:300`) | unchanged call order |
| Diagnostics (`compiler/src/diag.rs`) | same codes, messages, helps; appended after the walk's diagnostics (AR-9) |

## Error Handling

| Error case | Handling | AR Ref |
| ---------- | -------- | ------ |
| A conflict found in the post-pass | report at the event span, v1 message and help | AR-9 |
| `E10030` found by reachability | report once at the loan's creation span | AR-8 |
| An invalid program now reports more than one conflict | accepted: conflicts no longer return early; the Phase 2 sweep reconciles any test that pins an exact code vector | AR-11 |
| An event recorded in a discarded speculative graph | prevented: recording is gated on `report_diagnostics`, the log is swapped around lambda lifting, and every event recorded inside a defer is evaluated at that defer's `DeferEdge` positions, never at its recorded one | AR-4 |

## Testing Requirements

* Unit tests drive `regions` and the rule decisions on hand-built event logs
  and small `ir::Function` graphs (branch, loop, empty block, no-use loan).
* The end-to-end behavior is owned by `07-testing-strategy.md`; the
  implementation tests build and run the accepted programs.
