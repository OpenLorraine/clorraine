# Escaping Stores and Their Spellings: Conditional, Projection, and Update Residue

> **Document**: 03-03-escaping-stores.md
> **Parent**: [Index](00-index.md)

## Overview

Four rows on the store side of view lifetime: a view stored into storage that outlives its
backing (a global, a field, an out-slot) is not judged; the view-array store to a global struct
field emits invalid C; the out-slot store path judges no provenance; and the element-store rule
over-rejects a local-backed view. The frozen rule is BR-8 (Ch 03 §7): a view must not be stored
where the backing dies first; a storage that dies with the frame does not escape.

## Design

### 1. The escaping store judgment (`DEF-265`, `DEF-276`, AR-5)

`names = mk().v;` (a global `str[1]` and a single `str`), `fill(borrow slot: str) { slot = mk().v; }`,
and `slot = c ? b.v : a;` (the widened conditional spelling) are accepted and the stored view
outlives the backing — use-after-free at base and head.

Judge the stored value's provenance against the frame the write escapes, as the return path does:

- A whole-value store into a **global** binding or field: the stored view's derivation must name
  an owner that outlives the store (a module-level static or a parameter-backed value); a
  local-backed or fresh-derived view reports `E10033` (the existing borrow-escape code).
- A whole-value store into a `borrow T?` **out-slot**: the same judgment (the slot's storage is
  the caller's frame, which outlives the callee). The current local store check is gated on
  non-empty projections, so a whole-value store is never judged; the gate opens for the
  view-typed whole-value case.

The array spellings (`slot = [mk().v]`, `names = [mk().v]`) use the same judgment per element, so
the leak and the dangling both close.

### 2. The view-array store and capture spelling (`DEF-269`)

`g.names = [mk().v]` (a global struct field) emits `g.names = { … };`, which the strict build
rejects; a capturing lambda over a `str[1]` fails with `incompatible pointer type`.

Lower the store through the ordinary value-into-place path (the array literal's element-wise
assembly the local spelling already uses) and build the lambda capture through its conversion
(the environment field's view-array type spelled as the same C shape the local binding uses).

### 3. The local element store (`DEF-280`, AR-6)

`xs[1] = c ? b.v : a;` (a local `str[N]` and a local-backed arm) reports `E10033: borrow of 'xs'
escapes its scope` although the array dies with the same frame; the named spelling
`xs[1] = v;` already reports it.

Scope the element-store loan rule to storage that outlives the frame (a field or a global, per
BR-8): a local array element store accepts a local-backed view. The named and conditional
spellings change together, so their parity is preserved; the global/field spellings keep
reporting.

## Integration Points

- `own/statements.rs`: `prepare_store` (:357) — the whole-value store judgment hooks.
- `own/expr.rs`: the out-slot store path and the element-store rule (`E10033` at :463, :780, :835).
- `own/place.rs`: `view_provenance` (:605) and the owner walk (:826) supply the judgment.
- `codegen`: the element-wise store spelling and the view-array capture conversion.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A local-backed view stored into a global/field/out-slot | `E10033` at the store (one report) | AR-5 |
| A parameter-supported view stored into an out-slot | Accepted (the caller's frame outlives the callee's slot) | AR-5 |
| A local-backed view into a local array element | Accepted | AR-6 |
| A view-array store to a global field | Valid C via the element-wise store shape | AR-11-adjacent |

## Testing Requirements

- Spec: the global single-view and array stores (rejected, one report), the out-slot stores
  (rejected; the parameter-supported twin accepted), the local element store (accepted, runs),
  the global field store and the lambda capture (strict build, run), all with ASan on the accepted
  paths.
- Impl: the report spans and counts, the element-wise store shape, the capture's C shape, and the
  byte-identity controls for the already-correct spellings.
