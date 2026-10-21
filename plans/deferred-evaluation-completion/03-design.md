# Design — Deferred Evaluation Completion (Batch V)

## `DEF-534` — `DiscardThen`

- Add `ValueKind::DiscardThen { effects, value }`. The yielded value is read by
  the enclosing expression; the effects are emitted where that value is
  evaluated.
- Lower a deferred fixed-array `len` operand's effects to a value:
  - a conditional becomes a conditional of effect values;
  - an array/struct/template recurses into its parts;
  - a member/index binds a fresh root to a temporary so its owned fields
    release with the statement;
  - a call or other fresh value binds through the guarded materialization.
- Codegen has two forms:
  - `emit_guards` / `emit_statement_discard_fill` use statements for a loop
    test, a conditional arm, or a for step, including the statement-form drop
    of a growable container;
  - `guard_expressions` uses an inline comma expression for a short-circuit
    right side or a guarded operand.
- A `DiscardThen` is impure, so an index/divisor/slice-count use materializes it
  once. A conditional carrying one is materialized once when it sits inside an
  arithmetic step or another repeated expression.
- A loop condition containing a `DiscardThen` is materialized per test; a
  conditional test fills the temporary through per-arm statements.

## `DEF-535` — conditional opaque store

- `record_opaque_store` returns the instance-decided flag for any owned source,
  not only a simple place, so the declaration does not also run the arm moves.
- Record every conditional arm (`OpaqueSource`) with its name, path, parameter
  flag, and later uses. A caller-backed lend arm contributes no dying frame; a
  local or by-value arm does.
- A fresh arm always contributes a statement-temporary frame. At a view
  instance it reports `E10030` at the store when no later read already does.
- The instance audit:
  - a view destination widens every arm;
  - an owning destination reports `E10034` for each non-take lend arm and
    `E10026` for a deeper projected arm;
  - a later use reports `E10020` for the moved field and `E10025` for a
    whole-value read after a top-level field move.
- Definite re-initializations live in `FlowState::reinitialized_sources` and
  join by intersection, so a branch write keeps the moved state while an
  unconditional or all-path write clears it.

## Residual

The caller-side rule that a stored view keeps its backing borrowed for the
container's lifetime is missing (`DEF-540`). The concrete twin has the same gap;
this batch's generic conditional acceptance reaches it too. The batch records
the row; it does not claim the accepted view store is memory-safe until the rule
lands.
