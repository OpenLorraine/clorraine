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
4. The sibling not-lend audit (`check_not_lend_requirements`) exempts nullable substitutions the
   same way the store exemption does: the concrete `borrow v: String?` spelling is legal today,
   and the generic `borrow v: T` at `T = String?` must stay legal too (PF-006). The audit's
   existing duplicable carve-out gains a `substituted.nullable` clause, and an ST pins the
   generic nullable acceptance.

**One report per assignment site (AR-13, PF-004).** A site whose right side is itself a lend-place
move already judged by `move_place` (concrete) or by the `RequiresNotLend` duty
(parameter-typed) must not produce a second `E10034`. The selected mechanism is a recording-time
skip: the target-side judgment is skipped when the right side's move sources are lend-place moves
already judged by the existing path — `BindingKind::Param` with mode `None`/`Const`/`Borrow`, or
parameter-typed sources owing a not-lend duty at the same span (mirroring `move_place`'s exact
reporting set; do not broaden to `OutSlot` sources). ST-3 asserts exactly one `E10034` for both
the concrete and the generic both-borrow spellings.

**DEF-82 (R4, PF-005).** When the target is a `BindingKind::OutSlot` root and the right side is
exactly the target place — same binding **and** equal projections — report `E10020` with the
existing message (`'{name}' was moved and cannot be used`), the diagnostic a local moved-value
use receives. This covers the whole-binding form (`slot = slot`) and the exact projected form
(`slot[0] = slot[0]`), which this batch's P3 makes expressible for `T[]?` (indexing a nullable
slice requires narrowing). Corrected boundary, replacing the earlier "projected self-assign is
unreachable" claim: concrete owning element moves through an out-slot root are stopped by the
`E10026` element-move limit, and non-nullable lend roots by the not-lend duty; the substituted
maybe-owning element under an out-slot root records no duty, so the exact self-assign rule above
covers the shape this batch unlocks. The runtime-indexed variant (`xs[i] = xs[i]`) remains
unguarded and is a filed residual. ST-7 carries the projected clause (tested at P3).

**DEF-83 (R5, PF-002).** In `own/call_check.rs`'s borrow-argument arm, skip the `W10001` warning
when the effective parameter is a nullable borrow — `mode == Borrow` and the *substituted*
parameter type nullable, where the substitution is explicit or inferred. Compute the call's
inferred bindings once (the same `infer_call_types` the lowering uses, passed down from the call
check) so an inferred `setIt(p, borrow r)` with `raw u8?` is exempt exactly like the explicit
`setIt<raw u8?>(...)` spelling; warn only when the substituted type is duplicable, not a view,
and not nullable. A non-nullable duplicable lend (`borrow i32`, explicit or inferred) still
warns.

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
- `ir.rs` re-export and `own.rs` plumbing — the new duty record joins the existing collection
  (checker fields, constructor, `check` return tuple, module-level collection loop; PF-013).
- `own/instance_check.rs` — `check_not_lend_requirements` gains the nullable-substitution
  exemption (PF-006).

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
| Exact projected out-slot self-assignment | `E10020`, same as the whole-binding form | PF-005 |
| Nullable borrow parameter | No `W10001` | AR-9 |
| Inferred nullable borrow argument | No `W10001` (the substituted type is consulted) | PF-002 |
| Nullable substitution in a generic lend/store | Accepted — both audits exempt it | PF-006 |
| Duplicable lend store | Unchanged (accepted; `W10001` at the call) | AR-2 |
| Unmarked/`const` parameter write | Unchanged (existing read-only rule) | AR-13 |

> **Traceability:** every rule above back-references the register entry that resolved it. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification tests ST-1 – ST-8 ([07-testing-strategy.md](07-testing-strategy.md)): rejection
  shapes, the duplicable and nullable acceptance shapes (concrete and generic), the one-report
  pins (ST-3 concrete and generic), the inferred warning exemption, and the projected self-assign
  clause of ST-7 (exercised at P3).
- Host-ASan probes for the accepted store shapes (the `T[]?` out-slot write and the duplicable
  lend) prove no leak or double free.
- Implementation tests pin the emitted shape for the accepted out-slot stores and the absence of
  a second diagnostic at a deduped site.
