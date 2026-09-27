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

/// What makes a loan's identity stable across every pass: its owner,
/// permission, and the source range of the derivation that created it.
pub(super) struct LoanOrigin { owner: usize, kind: LoanKind, start: u32, end: u32 }

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
}

pub(super) enum AccessKind { Write, Move }

/// One recorded fact at one position.
pub(super) struct LoanEvent {
    /// Where the event runs on the control-flow graph.
    pub(super) position: Position,
    pub(super) span: Span,
    pub(super) kind: LoanEventKind,
}

/// Builds the position graph and reports E10022, E10031, E10032, and E10030.
pub(super) fn check(
    function: &ir::Function,
    events: &[LoanEvent],
    names: &BTreeMap<usize, String>,
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
* `FunctionChecker.next_ordinal: BTreeMap<BlockId, usize>` — the next event
  ordinal in each block.
* `FunctionChecker.loan_ids: BTreeMap<LoanOrigin, LoanId>` — interns a loan id
  from its origin, so every pass over a derivation agrees on the id.
* `FunctionChecker.binding_names: BTreeMap<usize, String>` — the written name of
  every binding ever declared; it outlives the truncation at a deferred block's
  exit, so an `E10030` about a defer-local owner still names it.
* `FunctionChecker.defer_anchor: Option<BlockId>` — set while a deferred block
  is checked, so its events are placed at the edge's real block.
* `FunctionChecker.continue_states: BTreeMap<BlockId, FlowState>` — the joined
  state of every `continue` edge keyed by its target block, so `loop_entry_state`
  joins a `continue` back edge into the head even when it terminates the body.
* `DeferEntry.registration: u32` — the registration number a deferred block
  shares with the scope's bindings, so `run_scope_cleanup` interleaves the
  scope's drops and defers in the same reverse order the emitter runs them.
* `FlowState.loans: BTreeMap<usize, BTreeSet<LoanId>>` (`compiler/src/own/flow.rs`)
  — the loans each view binding may carry at a point; merged by union at joins
  and loop entries. Because a loan's id is interned from its origin, the
  speculative loop fixpoint populates this map too, so a loan created in a loop
  body is carried at the head, in the condition, and after the loop, and a loan
  created in a deferred block is carried on the exit edge. `pop_scope` drops the
  loans of the bindings it closes.

Events are recorded only while `report_diagnostics` is true, which excludes the
speculative loop fixpoint (`loop_entry_state` sets it false for the whole pass,
`compiler/src/own.rs:389-439`). The loan id is still interned on that pass, so
the fixpoint's loan sets stay bounded (one id per derivation site) while the
head state is correct. `lift_lambda` (`compiler/src/own/lower.rs:311-397`)
swaps the log, the loan interner, and the name table around the lifted
function's check and restores them.

### Loan identity resolver

`view_loans(&self, expr: &Expr) -> BTreeSet<LoanId>` mirrors `view_provenance`
(`compiler/src/own/place.rs:249-276`) on the loan chain. It returns the set the
current flow state gives the value, so a binding that may alias more than one
owner after a join yields every loan:

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
| `bind_value` (`compiler/src/own/statements.rs:240-273`) | for a view binding whose `view_provenance` is `Local`/`Param`: `view_loans(value)` empty ⇒ allocate an id, record `Created { owner }`, and set the binding's loan set to it; non-empty ⇒ record `Used` for each source and set the binding's loan set to the sources. A whole-binding rebind updates the set the same way; a rebind to a value with no loan clears it; `E10041` is checked in `record_view_loan` |
| `record_view_loan` (`compiler/src/own/statements.rs:276-301`) | keep the inline `E10041` check; intern the loan id from its origin on every pass so the speculative fixpoint and the reporting pass agree; record `Request` and `Created` only on the reporting pass |
| `read_place` (`compiler/src/own/place.rs:18-68`) | for every loan the read binding carries, record `Used` |
| `write_place` (`compiler/src/own/place.rs:93-161`) | after the const/read-only checks: for a projected write through a view binding, record `Used` for every loan it carries; for a non-view binding, record `Access { owner: id, access: Write }`; a whole-binding write of a view binding is a rebind, so no use is recorded here and `bind_value` updates the set |
| `move_place` (`compiler/src/own/place.rs:170-229`) | after the `E10034` check: record `Access { owner: id, access: Move }`; the `blocks_move` check is removed |
| `check_argument_place` Mode::Borrow (`compiler/src/own/call_check.rs:278-343`) | keep `E10038`, `E10041`, `W10001`, and the duplicate-`lent` check; replace `loans.conflict` with `Request { owner, kind: Exclusive }` |
| any call argument | `eval` reads the argument, so `read_place` records `Used` for every loan it carries |
| `check_return` (`compiler/src/own/statements.rs:592-626`) | `eval` reads the returned value, so `read_place` records `Used` for every loan it carries |
| `run_scope_cleanup` (`compiler/src/own.rs`) | one scope's bindings and defers, sorted by registration newest first; a binding records `Release` and emits its drop, a defer is checked at the edge anchor. This is the single order the checker records cleanup in, matching `emit_actions` |
| `pop_scope` (`compiler/src/own.rs`) | runs `run_scope_cleanup` for the innermost scope, then pops it |
| `emit_exit_cleanup` (`compiler/src/own/statements.rs`) | per scope deeper than `floor`, innermost first: a `ScopeExit` marker then `run_scope_cleanup` |
| `emit_return_cleanup` (`compiler/src/own.rs`) | per open scope, innermost first: a `ScopeExit` marker then `run_scope_cleanup` |
| `check_defer_exit` (`compiler/src/own.rs`) | set `defer_anchor` to the edge's block for the whole block check, so every recorded event lands on the real edge in recorded order; a nested defer keeps the outer anchor; roll back the graph and the binding/scope stacks but **keep the flow state**, so a rebind the defer makes to an outer binding is visible after the scope |
| loop head and condition (`compiler/src/own/loops.rs`) | `while` and the three-clause `for` are lowered so the head block is checked with the loop-entry state; the condition's view uses are recorded against the loans the back edge carries |
| `loop_entry_state` (`compiler/src/own.rs`) | the head joins the pre-state, the fall-through state, and every `continue` edge for this loop; a `break`/`return` edge reaches only the exit. The speculative pass clones and restores the whole scope stack, so a compiler-owned temporary a body or step materializes cannot leave a stale binding index in a scope |
| `check_for_c` / `check_do_while` (`compiler/src/own/loops.rs`) | the `for` step and the `do`/`while` condition and exit join the continue edges before they are checked; `loop_exit_state` joins every `break` edge when a `while (true)` body always terminates |
| `check_break` (`compiler/src/own/statements.rs`) | run the edge cleanup (interleaved drops and defers), then record the break-edge state, so a loan a defer creates joins the state after the loop |
| `check_continue` (`compiler/src/own/statements.rs`) | run the edge cleanup, then record the edge state in `continue_states` keyed by the target block; `edge_state` drops the scopes the edge leaves from the recorded state |
| `DeferEntry` (`compiler/src/own/flow.rs:202-207`) | unchanged shape: the registered depth and block |

`Release` events are recorded for every binding in a leaving scope. Loans only
ever name real storage owners (owners reached through the provenance chain),
so releases of view bindings are inert.

### Post-pass

After `builder.finish()`:

1. Build the successor lists from each block's terminator.
2. Group events by block in recorded order and build the position graph
   (`03-01`); a deferred block's events are already placed on the edge's real
   block, so no deferred position needs resolving.
3. For each loan, mark forward-reachable positions from `Created` and
   backward-reachable positions from every use; the region is the
   intersection.
4. Walk the events in order:
   * `Access::Move` with a live loan of the owner ⇒ `E10022`;
   * `Access::Write` with a live **shared** loan ⇒ `E10031`;
   * `Request` conflicting with a live loan (`loan.kind == Exclusive ||
     request.kind == Exclusive`) ⇒ `E10032`.
5. For each loan, compute the positions reachable from its creation without
   leaving a release of its owner; a use outside that set has no release-free
   path and marks the loan for `E10030`. Report `E10030` once per marked loan
   at its creation span, in loan-id order.
6. Conflicts report at the event span with the v1 messages and helps
   (`compiler/src/own/place.rs:118-123`, `compiler/src/own/statements.rs:289-293`,
   `compiler/src/own/call_check.rs:325-329`).

### Wiring

* `FunctionChecker::check()` (`compiler/src/own.rs:246-273`): after
  `builder.finish()` and the locals/temps are set, call
  `liveness::check(&function, &self.events, &self.binding_names, self.diagnostics)`.
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
| An event recorded in a discarded speculative graph | prevented: recording is gated on `report_diagnostics`, the log is swapped around lambda lifting, and every event recorded inside a defer is anchored to the edge's real block in recorded order | AR-4, AR-26 |

## Testing Requirements

* Unit tests drive `regions` and the rule decisions on hand-built event logs
  and small `ir::Function` graphs (branch, loop, empty block, no-use loan).
* The end-to-end behavior is owned by `07-testing-strategy.md`; the
  implementation tests build and run the accepted programs.
