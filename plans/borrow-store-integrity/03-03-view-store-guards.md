# View-Store Guards: Borrow-Store Integrity

> **Document**: 03-03-view-store-guards.md
> **Parent**: [Index](00-index.md)

## Overview

This component owns the two guards a store target is missing relative to the read path:
DEF-105 (an element store through a possibly-null view must require the same non-null narrowing a
read requires) and DEF-106 (a constant-index element store into a dynamically-bounded view must
emit the `E11211` bounds trap). Both changes sit on the assignment path: the checker
(`own/expr.rs`) adds the null requirement, and the lowering/emission pair adds the bounds check
before the store.

## Architecture

### Current Architecture

- **Reads** evaluate `ExprKind::Index` by checking the base's nullability first
  (`own/expr.rs:281-289`, gated on `unsafe_depth`), and the emitter's guard pass
  (`codegen/guards.rs`) emits `if ({condition}) cl_trap("E11211", "index out of range");` for
  every `ValueKind::Index` whose base is a fixed-growable array, slice, or string view
  (`bounds_guard_condition`; in-range fixed-array constants are skipped as already validated).
- **Writes** never evaluate the target as a value. `check_assign`/`check_update` run
  `check_place_raw` over the target's projections (the raw gate) and then emit the store; the
  constant-index element path renders the place directly (`((*xs)).ptr[0] = v;`), and only the
  run-time-index path routes through `Statement::StorePointer`, whose `AddressOf(Index)` operand
  earns the guard.
- The emitter's `Place` carries only the final type, so an emission-side bounds guard cannot
  recompute the per-level base types for DEF-106; the checker, which types every AST node, can.

### Proposed Changes

**DEF-105 — a store target's element base obeys the read's null requirement.**

A new checker walk `FunctionChecker::check_store_index_nonnull(&mut self, target: &Expr, span: Span)`
runs for every write target (assignment, compound assignment, and update). It walks the target
expression exactly as the read path walks a value:

- `ExprKind::Index { base, .. }`: let `base_ty = self.ty(base)`; when `base_ty` is not `raw` and
  `base_ty.nullable` and `self.unsafe_depth == 0`, report through `check_nonnull_expr(base, span)`
  — the existing `E10010` ("value '{name}' may be null here") with the narrowing-aware state, so a
  narrowed base stays accepted; then recurse into `base`.
- `ExprKind::Member { base, .. }`: recurse into `base` (a nullable struct base is not expressible
  — `E10011` — and a view field is not expressible — `E10205`; the recursion keeps the walk
  faithful for any future nullable-capable field).
- `ExprKind::Unsafe(inner)`: recurse (the `unsafe_depth` gate already disables the check inside an
  `unsafe` block, matching the read path).
- Anything else: stop.

**DEF-106 — a constant-index element store carries the read's bounds trap.**

A new IR statement carries the guard-only values of a store target:

```rust
/// Runtime guards for a store target that has no value form.
///
/// A store target never evaluates as a value, so its element bounds checks
/// cannot ride on the stored value. The checker lowers the target's element
/// selection into index values here; the emitter runs the ordinary guard
/// pass over them and evaluates nothing.
Guards {
    /// One value per outermost element selection; each carries the index
    /// chain so the guard pass visits every level in evaluation order.
    checks: Vec<Value>,
},
```

- The checker builds each check from the target's AST where every node's type is known:
  `Value::new(element_ty, ValueKind::Index { base: lower_value(base_expr), index: lower_value(index_expr) })`
  for the outermost `Index` node of an element selection. One check per outermost selection; the
  guard pass recurses into the index chain and emits each level's trap in order.
- The checker emits `Statement::Guards { checks }` immediately before the store statement in the
  constant-index element path (the run-time-index path is unchanged — its `StorePointer` pointer
  already carries the guard).
- The emitter arm is `for check in checks { self.emit_guards(check); }`; fixed-array bases reduce
  to `bounds_guard_condition == None` and emit nothing, so their emission stays byte-identical.
- The read-analysis pass for local liveness (`codegen/function.rs`, the statement match near
  `note_value`) gains an arm that notes the check values' places as reads, so the guard counts as
  a use of its base.

## Implementation Details

### New Functions/Methods

- `check_store_index_nonnull` — the DEF-105 walk (`own/expr.rs`), called once near the top of
  `check_assign` and `check_update`.
- `guard_checks_for_store_target` — the DEF-106 lowering helper (`own/expr.rs`) building the
  `Guards` checks from the target's element selection.
- `Statement::Guards` — the new IR variant (`ir.rs`) with its emitter arm
  (`codegen/function.rs`) and read-analysis arm.

### Integration Points

- `check_nonnull_expr` and the narrowing state are reused unchanged (DEF-105).
- `bounds_guard_condition`, `emit_bounds_guard`, and `emit_guards` are reused unchanged
  (DEF-106).
- The `StorePointer` path keeps its existing guard behavior; fixed-array and non-element stores
  keep their existing emission.

## Code Examples

### Example 1: DEF-105 (fixed)

```clorraine
func f(borrow xs: u8[]?): i32 {
    xs[0] = 1;                 // rejected E10010: 'xs' may be null here
    if (xs != null) {
        xs[0] = 1;             // accepted; the narrowing survives the write
    }
    return 0;
}
```

### Example 2: DEF-106 (fixed)

```clorraine
func f(borrow xs: i32[]?): i32 {
    if (xs != null) {
        xs[0] = 1;             // traps E11211 at run time when the view is empty
    }
    return 0;
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Element store through an unnarrowed possibly-null view | `E10010` at the store, mirroring the read path | AR-5 |
| Element store through a narrowed or non-nullable view | Accepted; emission unchanged | AR-5 |
| Constant-index store out of range on a dynamic bound | `E11211` trap statement before the store | AR-6 |
| Constant-index store on a fixed-array base | Semantic-layer check; no guard line (byte-identical) | AR-6 |
| Runtime-index store | Unchanged; `StorePointer` already guards | AR-6 |

> **Traceability:** every error-handling strategy and design choice back-references the register
> entry that resolved it.

## Testing Requirements

- Specification cases ST-13 – ST-21 ([07-testing-strategy.md](07-testing-strategy.md)), including
  the trap runs, the narrowed acceptance, and the ASan probes.
- Implementation pins: the trap line spelled before the store, the fixed-array emission unchanged,
  the `Guards` statement shape, and byte-identical re-emission.
