# Escaping Stores and Their Spellings: Conditional, Projection, and Update Residue

> **Document**: 03-03-escaping-stores.md
> **Parent**: [Index](00-index.md)

## Overview

Four rows on the store side of view lifetime: a view stored into storage that outlives its backing
(a global, a field, an out-slot) is not judged; the view-array store to a global struct field emits
invalid C; the out-slot store path judges no provenance; and the element-store rule over-rejects a
local-backed view. The frozen rule is BR-8 (Ch 03 §7): a view must not be stored where the backing
dies first — the judgment follows the destination kind, and a storage that dies with the frame does
not escape.

## Design

### 1. The escaping store judgment (`DEF-265`, `DEF-276`, AR-5, AR-20)

`names = mk().v;` (a global `str[1]` and a single `str`),
`fill(borrow slot: str) { slot = mk().v; }`, and `slot = c ? b.v : a;` (the widened conditional
spelling) are accepted and the stored view outlives the backing — use-after-free at base and head.

Judge the stored value's provenance against the frame the write escapes, per destination kind:

- A whole-value store into a **global** binding or field: the stored view's derivation must name
  a module-level static owner; a parameter-backed, local-backed, or fresh-derived view reports
  `E10033` (the existing borrow-escape code). A parameter does not outlive a global.
- A whole-value store into a `borrow T?` **out-slot** (both the `borrow slot: str` and
  `borrow slot: str?` spellings): the slot's storage is the caller's frame, so a static- or
  parameter-backed view is accepted (the caller's frame outlives the callee) and a local-backed or
  fresh-derived view reports `E10030` (the loan-outlives code; the slot is a parameter, so
  `E10033`'s field/global trigger does not fit). The current local store check is gated on
  non-empty projections, so a whole-value store is never judged; the gate opens for the view-typed
  whole-value case.

The array spellings (`slot = [mk().v]`, `names = [mk().v]`) use the same per-element judgment, so
the leak and the dangling both close.

### 2. The view-array store and capture spelling (`DEF-269`)

The fresh-element spellings are judged in §1, so the emission fix covers the accepted shapes:
`g.names = [s]` (a global struct field with a static- or parameter-backed element) must lower
through the ordinary value-into-place path — the array literal's element-wise assembly the local
spelling already uses — instead of the initializer-list C (`g.names = { … };`) the strict build
rejects; a capturing lambda over a `str[1]` must build through its conversion (the environment
field's view-array type spelled as the same C shape the local binding uses).

### 3. The local element store (`DEF-280`, AR-6, AR-21)

`xs[1] = c ? b.v : a;` (a local `str[N]` and a local-backed arm) reports
`E10033: borrow of 'xs' escapes its scope` although the array dies with the same frame. The
acceptance as stated is too narrow: the store must be judged against the *destination-expected*
type (a handle→view widening counts, so the widened spelling is not ungated), and the stored
view's owner loan must be recorded exactly as the binding path records it. Without the loan,
re-executed loop stores (`while (i < 3) { xs[1] = mk().v; }`) and nested-scope backings stay
accepted and read freed storage.

Scope the element-store rule to the backing's scope: a store accepts a view whose owner's scope
covers the array's, and a backing from a loop iteration or a nested block reports `E10030` (the
array outlives the backing). The named and conditional spellings change together, so their parity
is preserved; the global/field spellings keep reporting.

## Integration Points

- `own/statements.rs`: `prepare_store` (:357) — the whole-value store judgment hooks.
- `own/expr.rs`: the out-slot store path (`E10030`) and the element-store rule (`E10033` at :564,
  :885, :940; the scoped acceptance).
- `own/place.rs`: `view_provenance` (:605) and the owner walk (:826) supply the judgment.
- `codegen`: the element-wise store spelling and the view-array capture conversion.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A local-backed or fresh-derived view stored into a global or field | `E10033` at the store (one report) | AR-5, AR-20 |
| A parameter-backed view stored into a global or field | `E10033` (a parameter does not outlive the storage) | AR-20 |
| A fresh- or local-derived view stored into an out-slot | `E10030` at the store (one report) | AR-20 |
| A static- or parameter-backed view stored into an out-slot | Accepted (the caller's frame outlives the callee's slot) | AR-5, AR-20 |
| A local-backed view into a local array element (the backing's scope covers the array's) | Accepted, with the binding path's loan | AR-6, AR-21 |
| A loop-iteration or nested-scope backing into a local array element | `E10030` (one report per site) | AR-21 |
| A view-array store to a global field | Valid C via the element-wise store shape | AR-11-adjacent |

## Testing Requirements

- Spec: the global single-view and array stores (rejected, `E10033` once; the parameter-backed
  spelling as the rejection control), the out-slot stores (fresh-derived rejected `E10030` once;
  the static/parameter-backed twins accepted and run), the widened stores, the loop and
  nested-scope element stores (rejected `E10030` once), the local element store (accepted, run
  exit 0), the global field store with a static/parameter-backed element and the lambda capture
  (strict build, run exit 0), all with ASan on the accepted paths.
- Impl: the report spans and counts, the element-wise store shape, the capture's C shape, and the
  re-emission determinism pins for the already-correct spellings.
