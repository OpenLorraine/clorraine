# Current State: Non-Lexical Lifetimes (NLL v2)

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The ownership stage runs after resolution and typing and before codegen
(`compiler/src/driver.rs:300`). It walks each function body with
`FunctionChecker` (`compiler/src/own.rs:128`) and, while it checks, lowers the
body into the typed control-flow IR through `Builder` (`compiler/src/own.rs:157`)
— so at the end of a function check the real CFG already exists.

Loans live in a lexical set. `Loan` carries the owner binding, the kind, the
scope depth it begins at, and a span (`compiler/src/borrow.rs:29-39`);
`Loans` is a canonical set with conflict predicates (`compiler/src/borrow.rs:43-125`).
The set travels in `FlowState.loans` (`compiler/src/own/flow.rs:113`) and is
retired by scope depth in `pop_scope` (`compiler/src/own.rs:338`), unioned at
joins in `merge` (`compiler/src/own.rs:638-639`), and iterated to a fixed point
for loops (`compiler/src/own.rs:389-439`).

A view binding records a loan of its owner (`compiler/src/own/statements.rs:261,
276-297`), and the lexical `E10030` check compares the view binding's scope
depth with the owner's at binding time (`compiler/src/own/statements.rs:247-260`).
Conflicts are decided inline: writes consult `blocks_write`
(`compiler/src/own/place.rs:116`), moves consult `blocks_move`
(`compiler/src/own/place.rs:185`), and loan requests consult `conflict`
(`compiler/src/own/statements.rs:287`, `compiler/src/own/call_check.rs:322`).

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/own/liveness.rs` | — | **New**: event types, region pass, post-pass checks |
| `compiler/src/borrow.rs` | Loan set, provenance, view predicates | Remove `Loan`/`Loans` and their unit tests (`:239-270`); keep the checker tests (`:272-345`), `LoanKind`, and the predicates |
| `compiler/src/own/flow.rs` | Walk state | Remove `FlowState.loans`; add `Binding.loan` |
| `compiler/src/own.rs` | The walk, scopes, merges, loop fixpoint, cleanup emission | Event log; remove loan merge/expiry; release events; wiring |
| `compiler/src/own/statements.rs` | Binding, assignment, exits, returns | Create/record loans; remove lexical `E10030` and inline `E10032` |
| `compiler/src/own/place.rs` | Place reads/writes/moves, provenance | Record uses and accesses; remove inline `E10022`/`E10031` |
| `compiler/src/own/call_check.rs` | Call arguments and lends | Record loan requests and uses; keep the duplicate-lend check |
| `compiler/src/own/lower.rs` | Value lowering, lambda lifting | Save/swap the event log for lifted lambdas |
| `compiler/src/own/expr.rs` | Expression checking | The read sites that become use hooks (`:87`, `:215`, `:302`, `:380`) |

### Code Analysis

The three pieces the design reuses already exist:

* **The CFG.** `ir::Function` holds blocks with terminators and ordered
  statements (`compiler/src/ir.rs:432-500`); reachability over it is what the
  liveness pass needs, and it is built by the same walk that will record events.
* **View provenance.** `Provenance` says where a view value's storage comes
  from (`compiler/src/borrow.rs:134-145`) and is resolved by
  `view_provenance` (`compiler/src/own/place.rs:249-276`). Loan identity
  attaches to the same chain (AR-5).
* **Loan kinds.** `view_loan_kind` already classifies a derivation as shared or
  exclusive from the callee's `StdlibEffect` (`compiler/src/own/place.rs:282-299`,
  `compiler/src/own/program.rs:49`).

The lexical model has two duplicate implementation points to remove — the
expiry/union handling in the walk and the inline conflict checks — plus the
binding-time `E10030` comparison (AR-10).

## Gaps Identified

### Gap 1: A loan expires at its view's scope end

**Current Behavior:** `Loan.origin` is a scope depth
(`compiler/src/borrow.rs:36`) and `Loans::end_scope` drops loans whose origin is
the closing scope (`compiler/src/borrow.rs:70-72`, called at
`compiler/src/own.rs:338`).
**Required Behavior:** a loan ends at its last use on any path (R1, AR-3).
**Fix Required:** regions computed over the emitted CFG in a post-walk pass
(AR-4).

### Gap 2: `E10030` is a lexical comparison at binding time

**Current Behavior:** `bind_value` rejects when the view's scope depth is
smaller than the owner's (`compiler/src/own/statements.rs:247-260`).
**Required Behavior:** reject only when a use is reachable from an
owner-release point (R4, AR-8).
**Fix Required:** the liveness pass checks uses against release events; the
lexical check is deleted.

### Gap 3: View values carry no loan identity

**Current Behavior:** `Provenance` names the owning binding only
(`compiler/src/borrow.rs:134-145`); a loan is a set entry with no identity.
**Required Behavior:** copies and derived views share one loan id, and uses
through any alias extend that loan's region (R2, AR-5).
**Fix Required:** a `LoanId` travels with view values (a `Binding.loan` field
plus a `view_loan(expr)` resolver mirroring `view_provenance`).

### Gap 4: Uses are never recorded

**Current Behavior:** conflicts read the current live set during the forward
walk (`compiler/src/own/place.rs:116,185`; `compiler/src/own/statements.rs:287`;
`compiler/src/own/call_check.rs:322`).
**Required Behavior:** uses are recorded and regions decide the conflicts
post-walk (R1, R3, AR-4, AR-6, AR-7).
**Fix Required:** event recording at the read/write/move/call/return/cleanup
sites plus the post-pass.

### Gap 5: Dormant loan scaffolding is unused

**Current Behavior:** `ir::LoanKind` (`compiler/src/ir.rs:60-67`) and
`Statement::Borrow` (`compiler/src/ir.rs:306-313`) are declared but never
constructed; codegen matches the statement as a no-op
(`compiler/src/codegen/function.rs:404`).
**Required Behavior:** unchanged — this plan does not adopt the dormant
scaffolding (AR-17).
**Fix Required:** none; recorded so an executor does not mistake it for the
liveness hook.

## Dependencies

### Internal Dependencies

- `compiler/src/ir.rs` — the CFG the pass runs on (`Function`, `Block`,
  `Terminator`).
- `compiler/src/types` — `Ty`, `TypeTable::duplicable`, view predicates.
- `compiler/src/sema` — resolved types the stage already consumes.
- `compiler/src/own/program.rs` — `Callee`, `StdlibEffect` for loan kinds.
- `compiler/src/diag.rs` — the diagnostic sink.

### External Dependencies

- None. The workspace is std-only; no crates, no build scripts, no harness.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| A missed use site weakens liveness and accepts an unsafe program | Medium | High | Enumerate every access hook in 03-02; the spec suite covers each class (07); the monotonicity run keeps v1 acceptances; runtime impl tests build and free |
| Event positions record during the speculative loop fixpoint or a discarded defer edge | Medium | High | Recording is gated off during the speculative loop fixpoint; the log is swapped around `lift_lambda`; every event recorded inside a defer is evaluated at the defer's recorded exit edges, never at its own position (03-02) |
| Diagnostic order changes break a test that pins ordering | Low | Medium | Same codes/messages/spans; deterministic event order (AR-9); the sweep in Phase 2 lists any test change |
| The flipping golden or another fixture is missed | Low | Medium | `make check` fails on any missed golden; the sweep task reconciles the full list (AR-11) |
| Spec amendment drift from the implementation | Low | High | `03-03` fixes the exact edits; the amended chapter is reviewed before its commit (AR-2) |
| Loop/lambda interaction reintroduces lexical expiry | Low | High | The lexical state is deleted, not bypassed (AR-10); no fallback path remains |
