# Move-Out Integrity: Def Backlog Completion

> **Document**: 03-02-move-out-integrity.md
> **Parent**: [Index](00-index.md)

## Overview

Close DEF-60 and DEF-59. DEF-60: an element move-out loses the run-time index, so it empties a
different slot than the callee took. DEF-59: a store or return of an ownerless element read copies
the element without moving it out of the statement-owned temporary, so the temporary release frees
storage the destination still reads. This document owns both behaviors; the reproduction evidence is
in [02-current-state.md](02-current-state.md) §Reproductions, and the rulings are AR-1 and AR-5.

## Architecture

### Current Architecture

A lowered element read is `ValueKind::Index { base, index }`; `value_place`
(`compiler/src/own/lower.rs:771-806`) turns it into a `Place` whose projection is
`Projection::Element(Option<usize>)` — a constant when the index is an integer literal, `None`
otherwise. Codegen renders `Element(None)` as `[0]` (`compiler/src/codegen/expr.rs:126-137`), with
the documented rationale that a run-time index "does not survive on a projected place". That holds
for bounded assignment targets, but the `take` move-out (`own/call.rs:346-380`) and the store move
(`own/statements.rs:279-321`) derive places for reads whose index does survive in the value.

The store path additionally gates the projection move on `!projects_from_place`
(`own/lower.rs:863-884`), which is true for a destination `Temp` and a `Materialize` root, so an
ownerless element read from statement-owned storage is stored as a plain copy; the temporary release
then frees the slot. The return path has the same gap in the emitter's no-ladder branch
(`compiler/src/codegen/cleanup.rs:343-356`): it stores into a block-local and runs the releases
without emptying the source first, while the result-var branch (`:327-341`) already calls
`emit_arm_source_moves`.

### Proposed Changes

1. **An index-preserving element projection.** Add `Projection::ElementTemp(TempId)` beside
   `Element(Option<usize>)`: an element addressed by a run-time index held in a compiler temporary.
   `Element(Some(n))` and `Element(None)` keep their current rendering; the new variant renders the
   temporary's name through the existing `index_text` helper.
2. **Materialize a run-time index once.** A lowering helper in `materialize_owned_argument`'s take
   block (`compiler/src/own/lower.rs:1193`, a `&mut` producer that both the call lowering and the
   statement path reach) turns a non-constant, non-temp index value into a compiler temporary
   (`materialize_value`, a plain `Statement::Assign`) and rebuilds the `ValueKind::Index` around
   it, so the call argument and the move-out render the same temporary and the index expression is
   evaluated once. The helper mirrors `materialize_take_place` (`own/lower.rs:691-715`): when
   `defer_move_outs > 0` it wraps the index in a `ValueKind::Materialize` destination instead of
   assigning eagerly, preserving short-circuit evaluation, and it recurses through `Member` chains
   and nested `Index` levels so multi-level indices keep every level (`mk()[i][j]`, PF-023). A
   bare discarded call (`consume<T>(mk()[i]);`) reaches the same producer through
   `own/statements.rs:180`, so both move-out call sites are covered by construction.
3. **`value_place` uses the temporary.** The `Index` arm maps an integer literal to
   `Element(Some(n))`, an already-materialized index (`Temp` or `Materialize`) to
   `ElementTemp(id)`, and anything else to the existing `Element(None)` fallback.
4. **The store moves out of statement-owned storage.** In `prepare_store`, allow the move path when
   the ownerless projection roots at a statement-owned destination temporary
   (`is_destination_temp`, `compiler/src/own.rs:999-1005`) in addition to the fresh-call-result
   case. The destination temporary is already owned and registered, so the source place is derived
   directly (no re-binding), and the existing `PreparedStore::Move` copy-and-zero arm empties the
   slot.
5. **The return moves the source first.** In the no-ladder return branch, when the returned value
   has a storage root and its type needs a drop, call `emit_arm_source_moves` after the block-local
   store and before `emit_temp_releases`, mirroring the result-var branch.

### Known Limitation (filed, not fixed)

`consume_place` (`own/lower.rs:742-763`), the aggregate-literal consume path, also derives a
move-out place, and its runtime-index form (`Box<T> { v: mk()[i] }`) still moves slot zero and
segfaults — the root call is never emitted on that path, so the index helper alone does not repair
it. This is a distinct defect (PF-009) filed as DEF-64 in the roadmap; this plan documents the
limitation and does not extend the aggregate-literal path.

## Implementation Details

### New Types/Interfaces

```rust
/// One selection applied to a place.
pub enum Projection {
    /// A named struct field.
    Field(String),
    /// An array or slice element with a known constant index.
    Element(Option<usize>),
    /// An array or slice element reached by a run-time index that is held in a
    /// compiler temporary, so the move-out empties the slot the read selected.
    ElementTemp(TempId),
}
```

### Integration Points

| Site | Change |
| ---- | ------ |
| `ir/place.rs` | The `ElementTemp` variant |
| `own/lower.rs` | The guard-aware index-materialization helper in `materialize_owned_argument`'s take block; `value_place`'s `Index` arm; the store preparation |
| `own/call.rs` | Unchanged: `take_move_out_places` derives the place through `value_place`, which now carries the materialized index |
| `own/statements.rs` | `prepare_store` accepts a statement-owned destination root |
| `codegen/cleanup.rs` | The no-ladder return branch moves the source before the releases |
| `own/expr.rs` | `check_place_raw` gains an explicit `ElementTemp` arm mirroring `Element(_)` |
| `own/place.rs` | `mark_written` and `move_place` gain explicit `ElementTemp` arms mirroring `Element(_)` |
| `own.rs` | `projected_type` maps `ElementTemp` to `ty.iterated_element()` |
| `codegen/expr.rs` | `place()` renders `ElementTemp` through `index_text` |
| `compiler/tests/residual_defect_completion_impl_test.rs` | The stale pin asserting `__cl_tmp0[0] = NULL` for `__cl_tmp0[i]` is updated to the corrected shape |

### Behavior Rules

- The element take's emitted shape is unchanged for a constant index (`xs[3]`) and for the
  documented constant-zero take regression (ST-11), so the take path's existing pins stay
  byte-identical.
- A constant-index **store or return** of an ownerless element read changes from a copy to a move
  (the fix's purpose): the emitted shape gains the copy-and-zero, and the run changes from a
  use-after-free to clean (ST-20).
- A run-time index is materialized once; the call/read and the move-out share one temporary, and no
  index expression is evaluated twice. Under a guard (`&&`/`||`/`?:`) the materialization is itself
  deferred, so short-circuit evaluation is preserved.
- Multi-level indices (`mk()[i][j]`) materialize every non-constant level and empty the exact slot
  (ST-25).
- The move-out empties exactly the selected slot; sibling elements stay owned by the temporary and
  are released with it.
- The guards on a run-time index (bounds, division in the index expression) keep their current
  placement and count.
- The store and return moves apply only when the destination type owns at this instantiation; a
  duplicable instance keeps its copy semantics.
- `Element(None)` remains only for checker-bounded assignment targets that never derive a move-out
  place; the aggregate-literal consume form that still reaches it is the filed DEF-64 limitation
  (PF-009).

## Code Examples

```clorraine
// DEF-60: the callee takes slot i; the caller's temporary empties slot i, not 0.
func probe<T>(mk: func(): T[2], i: i32) { sink<T>(mk()[i]); }
```

```clorraine
// DEF-59: the store and the return both move the element out before the
// statement temporary's release.
func storeFirst<T>(mk: func(): T[2]): T { let s: T = mk()[0]; return s; }
func first<T>(mk: func(): T[2]): T { return mk()[0]; }
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Run-time index on a move-out | Carry the index in `ElementTemp` and materialize it once | AR-1 |
| Index value that is already a temporary | Reuse it; do not create a second temporary | AR-1 |
| Index under a guard or in a `Member` chain | Defer through `ValueKind::Materialize`; recurse the chain | PF-006, PF-023 |
| Projection rooted at caller storage (`p.v`, `xs[0]`) | Unchanged: the existing `value_place` path already empties it | AR-5 |
| Projection rooted at a fresh call result | Unchanged: bound to an owned temporary and moved (`bind_ownerless_projection_root`) | AR-5 |
| Projection rooted at a statement-owned destination temp | Move out directly; the temporary's release sees the zeroed slot | AR-5 |
| Bounded assignment target with no surviving index | `Element(None)` keeps the `[0]` fallback; the aggregate-literal form is DEF-64 (PF-009) | AR-1, PF-009 |
| Duplicable instance | The emitted move falls back to a plain copy; nothing is emptied | AR-9 |

> **Traceability:** Every error-handling strategy references the Ambiguity Register entry (AR #) that
> resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification cases in [07-testing-strategy.md](07-testing-strategy.md): ST-7 – ST-11 (unguarded
  and guarded runtime-index takes, store and return element reads, the constant-index take
  regression), ST-20 (constant-index store/return move), and ST-25 (multi-level indices).
- Memory cases run with the `RDC_ASAN` opt-in and a host AddressSanitizer probe; the close-out
  runner includes DEF-59/DEF-60 probes with a timeout and `handle_segv=0` (PF-025).
- Implementation tests: the emitted `__cl_tmp[i] = NULL` shape, the return branch's source move
  ordering, the single index evaluation, the deferred form under a guard, and byte-identical
  re-emission for the constant-index take cases.
- Full verification: `make check`.
