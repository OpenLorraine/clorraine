# Null-Shaped Destinations and Capture Slot Stores: Residual Major Closeout

> **Document**: 03-05-null-destinations-capture-stores.md
> **Parent**: [Index](00-index.md)

## Overview

Two destination judgments: a null-shaped value at a nullable view or handle destination
(`DEF-245`), and a writable `[borrow x]` capture stored into a nullable owning out-slot
(`DEF-251`). The first is a lowering gap — the shape renders its own `void *` where the
destination's type is known (Ch 17 requires `T?` to lower to the same C type as `T` with null as
zero/NULL); the second is a store-judgment gap — the out-slot carve-out returns before the
capture-move rule and the accepted program double-frees. Both fixes extend existing judgments
(the null-value renderer, the lend/capture store rule); neither adds a pass (AR-14).

## Design

### 1. The null-shaped value at a nullable destination (`DEF-245`)

`let x = null; let s: str? = x;`, `show(x)` at `str?`, and `let fl: File? = x;` are accepted
and fail the strict build (`void *` copied/assigned into the view struct or the `BPTR` handle;
the `return x` view position already reports `E10035` and is closed). The shape's own C type is
`void *` (`codegen/types.rs:315`), and the store/argument/return render it from the value's own
type instead of the destination's.

Lower the shape to the destination's null value wherever the destination type is known (the
register's `DEF-245` decision: the typed lowering over a rejection): a view
struct destination gets its zero view (`cl_zero_ClStr()`-style, the existing `null_value`
machinery, `codegen/function.rs:2055-2056`, `codegen/types.rs:315`), a handle/pointer
destination gets `NULL`, and the accepted spellings build on every profile (the hosted-amiga
`File?` case included). The comparisons, the copy between null-shaped locals, and the null
literal keep their pinned acceptance and shapes; the concrete-into-null-shape and
null-into-concrete rules (batch G Phase 4) are unchanged. Stop condition (AR-13): if the typed
lowering conflicts with a pinned acceptance, the phase stops and parks the row with the probes.

### 2. The capture source at the nullable owning out-slot store (`DEF-251`)

`wrap<String?>(borrow out, s)` with `([borrow x, borrow out] () => { out = x; })()` is accepted;
`out` is the out-slot, `x` is a writable capture borrow, and the emitted store moves out of
`(*__env->x)` (zeroing it) while the caller still owns the value — the plain run aborts 134 and
ASan reports `attempting double-free`. `check_lend_target_store` returns early for every
`BindingKind::OutSlot` target (`own/expr.rs:1881-1899`, the out-slot carve-out), so the capture
move at the store is never judged.

The carve-out must not exempt a capture source: when the out-slot store's right side is a
capture (`ir::PlaceBase::EnvField`, the `is_caller_slot` family, `own/expr.rs:1578-1583`), the
store judges the capture move — the frozen capture rule (`E10401`, `sema/expr.rs:265-300` and
the per-instance audit `own/instance_check.rs:65-115`) — instead of accepting. The writable
non-capture out-slot stores (the pinned `put<String?>(borrow d, s)` family) and the read-only
source rejection keep their behavior; the exact-self-assignment behavior through an out-slot is
unchanged.

## Integration Points

- `codegen/expr.rs:344-345` (`is_null_shaped`), `codegen/function.rs:2046-2078` (`store_text`),
  `codegen/types.rs:315` (`null_value`) — the typed null rendering at stores, arguments, and
  returns.
- `types/mod.rs:688` (`is_null_shaped`) — the phase-4 predicates the lowering consults.
- `own/expr.rs:1881-1899` (`check_lend_target_store`), `:1572-1583` (`is_caller_slot`) — the
  out-slot carve-out and the capture-source recognition.
- `sema/expr.rs:265-300` (the capture move rule), `own/instance_check.rs:65-115` (the
  per-instance capture duty) — the judgment the store routes through.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A null shape at a nullable view/handle destination | The destination's zero view/NULL lowers; the program builds and runs | — |
| A null shape at a concrete destination | The batch-G Phase 4 rule reports (unchanged) | — |
| A null shape at a comparison/copy/literal position | The pinned acceptance and shape (unchanged) | — |
| A capture source stored into the nullable out-slot | The frozen capture rule (`E10401`) reports once; the run no longer double-frees | — |
| A writable non-capture out-slot store | Accepted (the existing carve-out) | — |
| A read-only capture source | The existing capture/lend report | — |

## Testing Requirements

- Spec: the view store, the view argument, and the `File?` store and return (accepted, strict
  build, the run reads the null back, ASan; the hosted-amiga build for the handle spellings), the
  pinned null comparison/copy controls, and the capture store (rejected once with the frozen
  capture code; the writable non-capture out-slot store accepted as the control).
- Impl: the zero-view/`NULL` spellings at each destination, the `BPTR`/handle spelling on the
  alternate profile, the capture-store report span and count, the non-capture control
  byte-identical, and a re-emission determinism pin.
