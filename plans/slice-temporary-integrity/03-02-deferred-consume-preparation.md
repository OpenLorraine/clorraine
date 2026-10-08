# Deferred Consume Preparation: Slice and Temporary Integrity

> **Document**: 03-02-deferred-consume-preparation.md
> **Parent**: [Index](00-index.md)
> **Defects**: DEF-72, DEF-73, DEF-74
> **AR**: AR-4

## Overview

`consume_place` prepares an element projection before deriving its place, but it prepares eagerly:
the base fill is emitted while the aggregate literal is being lowered, and an index that is already
deferred never gets its fill at all. The result is three filed shapes: a literal field that reads a
call result at a run-time index silently reads slot zero and never runs the index call (DEF-72); a
later field's base call runs before an earlier field's inline value, violating source order
(DEF-73); and a member-rooted consume is not prepared at all, so the call result's remaining fields
leak (DEF-74).

The fix keeps the preparation but defers it: the consumed value carries its pending fill and
move-out, and the consuming statement's guard emission runs them at that field's evaluation point,
in source order. This reuses the existing deferred `Materialize` carrier and its guard emitter.

## Architecture

### Current Architecture

- `consume_place` (`compiler/src/own/lower.rs:924-952`) prepares only `Index` values: it binds an
  ownerless root, settles the base (`:749-788`), pins the index chain (`:791-852`), derives the
  place (`:960-999`), and emits `Statement::Move` immediately.
- `materialize_index` accepts an existing deferred index as done (`:831-837`), so the fill is never
  emitted; `value_place` maps it to `ElementTemp` (`:990`) and the bare `Statement::Move` renders
  the place without `emit_guards` (`compiler/src/codegen/function.rs:405-449`).
- Deferred fills and move-outs already exist: `ValueKind::Materialize { temp, value, move_outs }`
  is filled by the guard emitter (`compiler/src/codegen/guards.rs:111-201`), including owned
  conditionals and fixed-array literals; the literal store path calls `emit_guards(field)` before
  each field store in source order (`compiler/src/codegen/function.rs:1516-1560`).
- The root binds (`bind_ownerless_projection_root` `:1349-1384`, `bind_guarded_projection_root`
  `:1398-1434`) already produce the eager and deferred root binding shapes for call roots.

### Proposed Changes

1. **A deferred consume value.** For a projection that needs preparation, `consume_place` returns a
   deferred `Materialize`: the fill is the prepared projection read, and `move_outs` carries the
   source place whose slot the consume empties. The temporary is **non-releasing**: the consuming
   destination adopts the value (the same ownership model as `materialize_take_place`), so no
   second owner appears. Every consumer empties the temporary at adoption — the literal and array
   consumers already do; the indexed-store consumer must zero it after the copy (preflight
   PF-006).
2. **The fill emitter gains the fixed-array object case.** When the consumed value is a fixed array
   (a row), the fill uses the fixed-array store core (`emit_fixed_array_assign_to`) instead of an
   assignment, which C rejects for array types.
3. **Member roots join the gate.** `consume_place` prepares `Member` roots as well as `Index`
   roots, through the same deferred root bind; the field projection then moves out of the bound
   temporary.
4. **Literals with a deferred fill use the store form.** `struct_literal_needs_stores` gains the
   predicate for a field carrying a deferred consume **or a deferred base fill of an element read**
   (duplicable or owning, preflight PF-009), so every field store runs its guards in source order;
   the indexed-assignment RHS already emits guards before its store and empties the consume
   temporary (item 1).
5. **The eager path remains for non-projection values**; owned conditionals, fixed-array literals,
   and call results keep their existing take/store behavior.

### Integration Points

- The index lowering already wraps a side-effecting index in a deferred `Materialize`
  (`compiler/src/own/lower.rs:48-50`); the consume fill must route through the guard emitter's
  `Index` recursion so the index statement is emitted before the read.
- `value_place` already maps a deferred index to `ElementTemp` (`:990`); the move-out place derived
  from a deferred chain must preserve the pinned index (the DEF-60 lesson: an unset index empties
  the wrong slot).
- The root bind's registered release frees the call result's remaining fields at statement end; the
  move-out empties only the selected projection.

## Code Examples

### Example 1: Impure index in a literal field (DEF-72)

```clorraine
// The filed generic form, instantiated with i32 or String:
Box<T> { v: make()[pick()] }   // pick() runs once; the chosen slot binds
```

### Example 2: Source order with an element projection (DEF-73/74)

```clorraine
let b: Box = Box { w: first(), v: make()[i] };      // first() runs before make()
let c: Box = Box { v: mk().a };                      // .a moves out; .b releases at statement end
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Deferred index already carried | Fill it at the consuming point through the guard emitter's `Index` recursion | AR-4 |
| Consumed value is a fixed array (row) | Fill through the fixed-array store core; no C array assignment | AR-4 |
| Member root over a fresh call | Deferred root bind; the move-out empties the selected field, the release drops the rest | AR-4 |
| A consume site that cannot emit guards | Extend that site's emission to run the guards at the value's evaluation point | AR-4 |
| Indexed-assignment adoption of a consume temporary | Zero the temporary after the copy, like the literal and array consumers (preflight PF-006) | AR-4 |
| A duplicable element read with a deferred base fill | The base fill is emitted at the field's turn; the store form is selected for the field (preflight PF-009) | AR-4 |
| `E10026` element-move decisions | Unchanged; forms rejected today stay rejected | AR-4 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity
> Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests ST-6 – ST-8 (`07-testing-strategy.md`): the impure-index field, the mixed
  field order (duplicable and owning instantiations), the member-rooted consume; host-ASan clean
  each.
- Implementation tests: the emitted fill and move-out for a literal field and for an
  indexed-assignment RHS; the consumer-side emptying at the indexed store; the store-form
  selection for a deferred consume and for a deferred base fill; the pinned index on the move-out
  place; byte-identical re-emission for a literal whose fields carry no deferred fill.
- Edge cases: a row (fixed-array) element consumed through a literal field; a nested index chain
  (`make()[i][j]`); a consume under a guarded operand (existing deferred path must keep working);
  a loop-form indexed assignment (`while (...) { out[i] = mk().a; }`) under host ASan.
