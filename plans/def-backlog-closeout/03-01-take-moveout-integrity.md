# Take and Move-Out Integrity: Def Backlog Closeout

> **Document**: 03-01-take-moveout-integrity.md
> **Parent**: [Index](00-index.md)
> **Defects**: DEF-63, DEF-64, DEF-66
> **AR**: AR-2, AR-8, AR-9

## Overview

Three defects share the place-derivation and move-out machinery: a guarded take whose call-rooted
projection never reaches the deferred move-out (DEF-63), an aggregate-literal consume that discards
the root call and the run-time index (DEF-64), and an element read through a slice view over a
statement temporary (DEF-66, reproduction-gated). The design reuses the existing deferred
`Materialize` carrier, the store preparation, and the slice-aware root walk.

## Architecture

### Current Architecture

- `lower_value` wraps an impure index base in a `Materialize` temp via `materialize_guard_operand`
  (`compiler/src/own/lower.rs:646-679`).
- The store path prepares completely: `prepare_store` (`compiler/src/own/statements.rs:287-348`)
  calls `settle_projection_base` (`compiler/src/own/lower.rs:690-730`), `materialize_index_chain`
  (`:739-764`), and `value_place` (`:892-931`) before emitting
  `PreparedStore::AssignThenMoveOut` (`compiler/src/own/statements.rs:16`, `:351-359`).
- The take path prepares per mode in `materialize_owned_argument`
  (`compiler/src/own/lower.rs:1336-1432`), but under `defer_move_outs > 0` it skips the root bind
  (`:1342-1346`) and gates the second bind (`:1356`).
- `take_move_out_places` (`compiler/src/own/call.rs:346-380`) falls back to `value_place`, which
  has no `Call` or `Intrinsic` root arm (`_ => None` at `compiler/src/own/lower.rs:929`).
- `consume_place` (`compiler/src/own/lower.rs:863-884`) is the aggregate-literal path; it calls
  `value_place` directly and emits a bare `Statement::Move`.

### Proposed Changes

#### 1. DEF-63 — deferred root binding for a guarded take

When the take argument is a projection whose root is `Call`/`ClosureCall` and
`defer_move_outs > 0`:

1. Bind the root through the same deferred `Materialize` shape `materialize_guard_operand`
   already produces (`temp`, `value`, `move_outs`), registering the release on the evaluated
   path (`compiler/src/own/lower.rs:646-679` is the template).
2. Let the existing `take_move_out_places` derive the member place from the temp
   (`compiler/src/own/call.rs:360-368`); the deferred branch in `lower_value`
   (`compiler/src/own/lower.rs:99-121`) then records the move-out on the guard's evaluated path.
3. The guard emitter (`compiler/src/codegen/guards.rs:194-200`, `:822-824`) already fills the temp
   and runs `move_outs`; no emitter change is expected.

The second bind in `materialize_owned_argument` (`:1356-1369`) must not create a second release:
the deferred temp's release replaces it for this shape.

#### 2. DEF-64 — prepared consume for aggregate literals

`consume_place` gains the preparation the store path already has, in order:

1. `bind_ownerless_projection_root` (`compiler/src/own/lower.rs:1281-1316`) for a call-rooted
   projection.
2. `settle_projection_base` (`:690-730`) to emit the deferred base fill.
3. `materialize_index_chain` (`:739-764`) to pin a run-time index into a temp (so `value_place`
   records `ElementTemp`, not `Element(None)`).
4. `value_place` (`:892-931`) for the target place.

Because a bare `Statement::Move` cannot carry the fill, the emission uses either an explicit fill
statement before the move or the value-carrying store arm
(`PreparedStore::AssignThenMoveOut`, `compiler/src/own/statements.rs:16`, `:351-359`), whichever
keeps the place semantics identical. The concrete `E10026` element-move decisions
(`compiler/src/own/expr.rs:201-228`) are preserved: forms that are rejected today stay rejected.

#### 3. DEF-66 — slice-root place derivation (reproduction-gated)

Task 0 probes the let-store, take, and return forms under host ASan. If a form reproduces:

1. Extend `bind_ownerless_projection_root` (`:1281-1316`) to recognize an
   `Intrinsic { name: "slice" }` root, following the precedent already in
   `collect_lowered_temp_owners` (`:1112-1116`), or add the equivalent `value_place` root arm.
2. Ensure every place-deriving path (`prepare_store`, the take path, `consume_place`,
   `check_return`) settles the intrinsic base before `value_place`.

If every form is clean, add the regression pins and close the roadmap row as already-fixed. The
existing `E10030` view-outliving rejection (`residual_defect_hardening_impl_test.rs:875`) and the
`E10026` decisions stay unchanged either way.

## Implementation Details

### Integration Points

- The fix sits in the ownership lowering; no IR type changes are expected (`Materialize` and
  `Projection::ElementTemp` already exist, `compiler/src/ir.rs:271-279`,
  `compiler/src/ir/place.rs:131-137`).
- The emitter's place rendering (`compiler/src/codegen/expr.rs:126-141`) already handles
  `Element(Some)`, `ElementTemp`, and `Element(None)`; the fix makes the correct arm reachable.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Take argument is a projection over a call under a guard | Deferred temp bind; evaluated path only | AR-8 |
| Aggregate-literal field is a call-rooted projection with a run-time index | Prepare like stores; pin the index; explicit fill | AR-9 |
| Slice-root element read already rejected by `E10026` | Keep the rejection; do not widen acceptance | AR-2 |
| Slice view outliving the temporary | Unchanged `E10030` | AR-2 |
| DEF-66 form already clean | Regression pins; roadmap correction | AR-2 |

> **Traceability:** Every strategy references the Ambiguity Register entry (AR #) that resolved it.
> See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: ST-1 – ST-7 (`07-testing-strategy.md`).
- Implementation tests: emitted-shape assertions for the guarded member zeroing, the aggregate
  fill, the run-time index slot, and the slice-root regression pins.
- Host-ASan probes: the exact programs from DEF-63/64/66 under
  `ASAN_OPTIONS=detect_leaks=1:symbolize=0:handle_segv=0 timeout 30`.
