# View Owners, Loans, and Stores: Conditional, Projection, and Update Residue II

> **Document**: 03-01-view-owners-loans-stores.md
> **Parent**: [Index](00-index.md)

## Overview

Seven rows in the view owner/loan/store judgments batch I left open: the duplicable-projection
residual binding, the runtime-index element store, the cross-frame out-slot pass-through, the
module-global fresh initialization, the container-escape element loans, the stale literal-carrier
loan, and the all-null composite store. The frozen rules are the single-view parity, one owner
per created value, BR-8 (Ch 03 §7: a view must not be stored where the backing dies first), and
the batch-I per-component store judgment (`storage_atoms`). Every fix extends the existing
owner/provenance walk and store preparation; none adds a pass (AR-14).

## Design

### 1. Duplicable projections over a residual root (`DEF-281`)

`holdNum(c ? a.n : mkBox().n)` (a `take`/call argument) and its return twin leak 11 B per fresh
`mkBox()`; the read positions `(c ? mk() : b).ys[i]` and `mk().ys[i]` leak 48 B/4 allocations
across the probe's four element reads (12 B per element read). The carrying gates skip a duplicable result (`!duplicable ||
maybe_owns`), so the fresh root never binds even though it still owns residual fields.

Bind the fresh root for its residual release in every carried position while the selected
duplicable member stays a copy: the root's un-taken fields release at the statement/return edge,
the copied scalar does not transfer ownership, and a place-rooted arm stays untouched. The gate
decision moves from "the selected type owns" to "the root owns residual storage".

### 2. The runtime-index element store (`DEF-285`)

`xs[j] = b.v;` emits `__cl_tmp0 = b.v; b.v = NULL; xs[j] = view(__cl_tmp0)` — the owner is
stranded (11 B per store) and the element loan is skipped, so `while { xs[j] = mk().v; }` stays
accepted and reads freed storage (33 B over three iterations). The constant-index twin copies
and is clean.

Route the runtime index through the constant index's copy/owner/loan path: the stored view's
backing adopts (or retains) its owner exactly as the constant-index and binding spellings do,
and the element loan record runs on the same path. The destination-expected type judges the
store as batch I's scoped element-store rule established (a handle→view widening counts).

### 3. The cross-frame out-slot pass-through (`DEF-286`, AR-6)

`outer(borrow slot) { let b = …; fill(b.v, borrow slot); }` stores a view whose backing lives in
`outer`'s frame into a slot in `outer`'s caller's frame — the slot is read after `outer` returns
(ASan heap-use-after-free at base and head). The callee cannot see the frame relation, so the
judgment belongs at the call site: when a view argument's backing is a frame local of the
caller and the callee's parameter is a lend that may write an out-slot, propagate the argument's
backing frame relation so the store rejects like its direct twin with one `E10030`
report (the direct twin's code; BR-8). The
parameter-backed and same-frame twins keep their acceptance.

Stop condition (AR-6): if the rule cannot be expressed without rejecting those accepted twins,
stop and park the row with the probe evidence — a recorded limit changes the accepted surface
and is a reserved decision.

### 4. The module-global fresh initialization (`DEF-287`)

A module binding holding a view of a fresh owner (`let g: str = stringOf("x")`-shaped) never
releases the allocation (11 B). The storage stays valid, so this is a pure leak. Adopt the fresh owner at module
initialization through the existing init-owner path (the
`module-initializer-integrity` release machinery): the **view destination retains
the owner for the module's lifetime**, exactly as the pinned rule does for the
`cstring` and view-array module destinations (a release at the init function's
end would dangle the module view — the committed retention tests record that
outcome as the defect they close).

### 5. The container-escape element loans (`DEF-288`)

`xs[1] = b.v; return xs;` (and the alias spelling) and `xs[1] = b.v; g = xs;` are accepted and
the caller reads freed storage, while the single-view twin `return s` reports one `E10035` and
the parameter twins are accepted clean. The view-like return and the global store's view-like
branch do not consult the binding's element loans.

Have both destinations consult the binding's element loans exactly as the carrier path does
(the batch-I two-step carrier return record): the frame-backed element loan rejects with the
destination's code (`E10035` for the return, `E10033` for the global store), one report per
site; the parameter-backed twins keep their acceptance.

### 6. The stale literal-carrier loan (`DEF-290`)

`let h = Holder { names: [b.v] }; h.names = ["AA"]; b.v = …` (and the whole-carrier
reassignment) reports a false `E10031` at the backing write: the literal carrier's loan
survives the overwrite, while the constant-index element twin replaces its loan cleanly.

Drop the literal carrier's loans on the field overwrite and on the whole-carrier reassignment
(the overwrite replaces the stored view, so the old loan has no live view to protect); the
backing write then compiles, and a still-live view keeps its loan. The decision mirrors the
batch-I element-loan replacement (P3S-5).

### 7. The all-null composite store (`DEF-291`)

`slot = c ? null : null;` into a `borrow str?` out-slot reports a false `E10030` (probe
confirmed; the global twin `E10033`), while a single `null` and null-arm mixes are handled: the
composite provenance yields an empty storage-atom set and the store judgment reports.

Treat an empty storage-atom set like the scalar `null` case — a value naming no storage cannot
escape — so the store is accepted and the emitted C carries the null view. The single-null and
mixed-arm spellings keep their pinned judgments.

## Integration Points

- `own/place.rs`: `view_provenance` (:605), `storage_atoms` (:700), `collect_view_owners`
  (:1028), `call_provenance` (:1190), `call_view_source` (:1250).
- `own/statements.rs`: `prepare_store` (:412) and `check_return` (:1253) — the element-loan
  consultation and the `LiteralViews` loan lifecycle.
- `own/expr.rs`: the out-slot store rule and the element-store rule (:564/:885/:940).
- `own/lower.rs`: `bind_ownerless_projection_root` (:3122), `materialize_owned_argument`
  (:4199), `value_place` — the residual-root binding gate.
- `own/call_check.rs`: `argument_owner` (:680) — the cross-frame call-site rule.
- `own/program.rs`/`own.rs`: the module-initializer owner path.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A duplicable member of a residual-owning fresh root | The root binds for its residual release; the copy transfers nothing | — |
| A runtime-index element store with a frame-backed view | Accepted with the binding path's copy and loan (batch I's scoped element-store rule) | — |
| A cross-frame out-slot pass-through | Rejected with the frozen BR-8 judgment (one report); the twins accepted (stop-gated) | AR-6 |
| A module-global view of a fresh owner | The owner releases at init end (or is retained for module storage) | — |
| A frame-backed element loan in an escaping container | Rejected per destination (`E10035` return, `E10033` global store) | — |
| A literal carrier overwritten | The stale loans drop; a live view keeps its loan | — |
| An all-null composite store | Accepted (names no storage), like the scalar `null` | — |

## Testing Requirements

- Spec: the duplicable-projection cases (take, return, both read-position twins; ASan), the
  runtime-index store (owner released, loan recorded, the writable-alias mutation reports like
  the constant twin; ASan), the cross-frame out-slot case with its accepted parameter/same-frame
  twins (ASan on the twin), the module-global fresh init (ASan), the container escape (return and
  global spellings rejected with the exact codes and counts; the single-view/parameter twins as
  controls), the stale loan (accepted after overwrite; the live-view control keeps its report),
  and the all-null store (accepted; the single-null and mixed-arm controls).
- Impl: the binding gates, the loan record/replace shapes, the report counts and spans, and the
  re-emission determinism pins for the untouched twins.
