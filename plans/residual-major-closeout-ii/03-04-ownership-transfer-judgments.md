# Ownership Transfer Judgments: Residual Major Closeout II

> **Document**: 03-04-ownership-transfer-judgments.md
> **Parent**: [Index](00-index.md)

## Overview

Two rows are ownership-judgment gaps in `own/expr.rs`: a view-like fixed-array copy transfers no
element loans, so a later element store through the copy writes freed storage; and an owned raw
deref read creates a second owner because the deref read bypasses the ordinary read's
move/duplicate judgment. Both were filed with host-ASan reproductions (heap-use-after-free and
attempting double-free); both fixes extend the existing loan/move judgments rather than adding a
pass.

## Design

### 1. The view-like copy's element loans (`DEF-315`)

`let ys: str[2] = xs;` (or `ys = xs;`) transfers no element loans: the carrier-copy transfer is
guarded by `!is_view_like(&target)`, so a view-like array is excluded, and a later
`fill(ys[0], borrow slot)` is accepted while the caller reads freed storage
(`memory-audit/final-audit/hp_alias_copy.clo`, ASan heap-use-after-free). The single-view twin
(`let s: str = xs[0];`) records the shared loan and reports the backing write with `E10031`.

Cover view-like containers in the copy transfer/judgment exactly as the single-view twin: a
copy of a view-like array records each element's loan (or re-routes per element), so a later
store through the copy either transfers the loan or reports the frozen conflict. The loan
lifecycle must match the existing carrier path (overwrite drops the stale loan — the batch-J
`DEF-290` fix's per-position drop), and the accepted read/copy spellings stay byte-identical
controls (`hp_alias_return`/`hp_alias_param`'s twins drive the matrix).

### 2. The owned raw deref read (`DEF-329`)

`let v: String = unsafe *q;` is accepted and both `v` and the pointee owner release the same
buffer (ASan attempting double-free; `rawreadown.clo`). The discarded read is clean and the
ordinary read (`let v = a;`) moves. The deref read lowers through the unary `Deref` arm
(`own/expr.rs:515`) and the IR `Deref` value (`:963`) without running the owned read's
move/duplicate judgment.

Route the owned raw deref read through the ordinary owned read's judgment: the read either moves
out of the pointee (emptying it through its own address, the batch-J FINAL-1 shape for the store
source) or reports the frozen `E10020` where a move is not available; a duplicable pointee
copies. The `unsafe` gate stays required; the discarded read and the duplicable twins are the
controls, and the batch-J `rawself_use` documented boundary (AR-18 of batch J) is checked for
parity rather than silently widened.

## Integration Points

- `compiler/src/own/statements.rs:365` (the copy transfer's `!is_view_like(&target)` guard),
  `compiler/src/own/place.rs:1112` (`move_element_loans`), `compiler/src/own/expr.rs` (the loan
  records, `view_provenance`, the deref read at `:515`/`:963`), the owned read judgment.
- The batch-J loan-drop helper (`DEF-290`'s per-position drop) and the `E10031` report path.
- Controls: `memory-audit/final-audit/{hp_alias_return,hp_alias_param,hp_alias_assign}.clo`, the
  concrete single-view loan cases, the batch-J `rawself`/`rawmove_use`/`rawdistinct_read` probes.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A copy whose element loans conflict with the source's live loans | The existing conflict report (`E10031`), one per site | — |
| A deref read at an emptied/possibly-null pointee | The existing null/`E10010` path for the nullable spelling | — |
| A move that cannot empty the pointee (an immutable view of the storage) | The frozen `E10020` (or the existing unsafe-boundary parity ruling) | — |
| A loan record that changes an accepted read/copy emission | Byte-identity controls pin the read forms | — |

## Testing Requirements

- Spec: the view-like copy matrix (local + parameter sources, `let` and assignment, the later
  store through the copy; ASan-clean), the loop/overwrite variants, the owned raw deref read
  (let form, discarded form, duplicable pointee, the move report where applicable; ASan-clean),
  and every recorded control.
- Impl: the loan records for the view-like copy (the report code/span/count), the deref read's
  move-out/emptying shape, the discarded and duplicable spellings byte-identical; a re-emission
  determinism pin.
