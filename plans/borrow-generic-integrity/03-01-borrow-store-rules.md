# Borrow-Store Rules: Borrow/Generic Integrity

> **Document**: 03-01-borrow-store-rules.md
> **Parent**: [Index](00-index.md)

## Overview

This component owns the P1 fixes for stores into lend parameters: DEF-78 (a store releases
caller storage), DEF-82 (an out-slot self-assignment empties the caller's slot), and DEF-83 (a
spurious warning on a nullable out-slot). All three live on the assignment/call-check paths of
`compiler/src/own/`.

## Architecture

### Current Architecture

- `own/expr.rs check_assign` handles every assignment; the whole-binding branch evaluates the
  right side, writes the place, emits the store, and applies the binding effect.
- `own/statements.rs emit_store`/`prepare_store` emits `Statement::Move` or `Statement::Assign`;
  the emitter releases the target's old value before the store.
- `own/place.rs move_place` rejects *moves out of* a lend parameter (`E10034`, message
  `cannot release or store borrowed parameter '{name}'`) but no check covers a store *into* one.
- `own.rs declare_param` classifies a nullable borrow parameter as `BindingKind::OutSlot`; only
  that kind makes out-parameter writes legal.
- `own/call_check.rs` warns `W10001` for a writable lend of a duplicable value that is not a
  view; it does not consult the parameter's out-slot status.
- Generic bodies defer substitution-dependent judgments through duty records
  (`ir::RequiresNotLend`, `ir::RequiresDuplicable`, `ir::RequiresOutSlotPlace`) audited in
  `own/instance_check.rs`.

### Proposed Changes

**DEF-78 (R1, R2, R3).** In the whole-binding branch of `check_assign`, after the target place is
resolved and before the store is emitted:

1. When the target binding is `BindingKind::Param` with `Mode::Borrow` and its resolved type owns
   storage (non-duplicable, not a view, not unknown) — report `E10034` with the existing message
   and help (`cannot release or store borrowed parameter '{name}'`).
2. When the target type is parameter-typed (mentions a type parameter, `maybe_owns`) — record
   `ir::RequiresStorableSlot { function, parameter, ty, span }`, deduped per
   `(ir_name, span.start, span.end)`, and add its audit to `own/instance_check.rs`: for each
   recorded instance, substitute the slot type and report the same `E10034` when the substituted
   type is non-nullable and owns storage; report the first offending instance once (the existing
   audit shape).
3. Every other target keeps today's behavior: `BindingKind::OutSlot` accepts the store (the
   nullable out-parameter is the caller's slot), duplicable and view types accept it, and
   unmarked/`const` parameters keep the existing read-only-write rule.

**One report per assignment site (AR-13).** A site whose right side is itself a lend-place move
already judged by `move_place` (concrete) or by the `RequiresNotLend` duty (parameter-typed) must
not produce a second `E10034`. State the invariant as an assertion in the spec tests; the
implementation may reach it by not recording/reporting the target-side judgment when the right
side's move effects already judge the same site.

**DEF-82 (R4).** In the same whole-binding branch, when the target binding is
`BindingKind::OutSlot` and the right side is exactly the target place (same binding, same
projections), report `E10020` with the existing message (`'{name}' was moved and cannot be
used`) — the same diagnostic a local self-assignment receives. The rule is whole-binding only;
projected self-assignment on an out-slot is unreachable because nullable-capable kinds (handles,
`raw`, `cstring`, views) expose no owned members.

**DEF-83 (R5).** In `own/call_check.rs`'s borrow-argument arm, skip the `W10001` warning when the
parameter is a nullable borrow — the same predicate `declare_param` uses (`mode == Borrow` and
the parameter type is nullable). A non-nullable duplicable lend (`borrow i32`) still warns.

## Implementation Details

### New Types/Interfaces

```rust
/// A store of a whole value into a lend parameter that is not yet known to be
/// an out-slot.
pub struct RequiresStorableSlot {
    pub function: String,
    pub parameter: String,
    pub ty: Ty,
    pub span: Span,
}
```

Collected on `ir::Monomorphization` through `require_storable_slot` and read by the new audit
next to `check_not_lend_requirements` and `check_out_slot_requirements` (AR-5).

### New Functions/Methods

- `own/expr.rs` — the whole-binding store judgment (helper used by `check_assign`): concrete
  rejection and duty recording per the rules above.
- `own/instance_check.rs` — `check_storable_slot_requirements`: substitute each instance's type
  arguments into the recorded slot type, report `E10034` for the first non-nullable owning
  substitution, skip nullable substitutions (out-slots) and duplicable substitutions.

### Integration Points

- The recording site shares the span with the store, so the audit's report points at the
  assignment statement.
- The `E10034` message and help text are reused verbatim; no diagnostics registry change.
- The whole-binding branch already distinguishes projections, so field and element stores are
  untouched by rules 1–3.

## Code Examples

```clorraine
// Rejected (R1): the callee releases storage the caller owns.
func setF(borrow slot: String): void { slot = stringOf("x"); }   // E10034

// Rejected for T = String (R2); accepted for T = i32 (W10001 only).
func fill<T>(take v: T, borrow slot: T): void { slot = v; }

// Accepted (R3): the nullable out-parameter is the caller's slot.
func fillString(take v: String, borrow slot: String?): void { slot = v; }

// Rejected (R4): the caller's slot would lose its value.
func self(borrow slot: String?): void { slot = slot; }           // E10020
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Owning store into a concrete lend parameter | `E10034`, existing message/help | AR-2, AR-13 |
| Owning store into a parameter-typed lend parameter | `RequiresStorableSlot` duty; `E10034` per offending instance | AR-2, AR-5 |
| Out-slot self-assignment | `E10020`, existing message | AR-8 |
| Nullable borrow parameter | No `W10001` | AR-9 |
| Duplicable lend store | Unchanged (accepted; `W10001` at the call) | AR-2 |
| Unmarked/`const` parameter write | Unchanged (existing read-only rule) | AR-13 |

> **Traceability:** every rule above back-references the register entry that resolved it. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification tests ST-1 – ST-8 ([07-testing-strategy.md](07-testing-strategy.md)): rejection
  shapes, the duplicable and nullable acceptance shapes, the one-report pins, and the warning
  exemption.
- Host-ASan probes for the accepted store shapes (the `T[]?` out-slot write and the duplicable
  lend) prove no leak or double free.
- Implementation tests pin the emitted shape for the accepted out-slot stores and the absence of
  a second diagnostic at a deduped site.
