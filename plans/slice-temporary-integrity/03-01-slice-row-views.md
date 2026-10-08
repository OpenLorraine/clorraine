# Slice Row Views: Slice and Temporary Integrity

> **Document**: 03-01-slice-row-views.md
> **Parent**: [Index](00-index.md)
> **Defects**: DEF-70
> **AR**: AR-3, AR-9

## Overview

A slice over a multi-extent source has rows as elements: `T[][M]` elements are `T[M]` arrays
(`spec/08-arrays-strings.md:28`). Today the view is emitted as a flat pointer over the base scalar
(`ClSlice_T { T *ptr; usize len; }`), so row moves copy one scalar while the caller still owns the
row, and nested accesses mis-type. This component makes the view's element type the row end to end:
the slice result type, the type key, the emitted typedef, the builder, and the access type all
agree, and the per-site type re-derivation disappears with them.

## Architecture

### Current Architecture

- `slice`'s result type is built from `element_of` (`compiler/src/own/call.rs:400-405`), which
  returns the immediate element and drops later extents (`compiler/src/own/call.rs:294-318`).
- The element-type recovery in index lowering is gated to fixed single-extent sources
  (`compiler/src/own/lower.rs:683-693`) because the emitted view disagrees with the recorded
  element; multi-extent bases keep the unknown type.
- The type key and C spelling flatten to the base element
  (`compiler/src/types/keys.rs:53,68-79`, `compiler/src/codegen/types.rs:97-103,345-347`).
- The typedef declares `{element_c} *ptr` (`compiler/src/codegen/program.rs:144-165`), while the
  builder computes a row pointer (`T (*)[M]`) and casts it back to the base-element pointer
  (`compiler/src/codegen/calls.rs:52-67,84-119`).

### Proposed Changes

1. **View element from the row.** The slice result type uses the source's iterated element
   (`Ty::iterated_element`, `compiler/src/types/mod.rs:375-386`): a scalar source keeps `T[]`; a
   multi-extent source yields `T[M][]`. Both row representations (`T[][M]` and a `T[]` whose
   element is `T[M]`) produce the same element type.
2. **One view element helper.** A single helper derives a view type's element (the row) and every
   consumer uses it: `type_key` (slice marker keyed by the row), `collect_slice_types`,
   `zero_view_helper`, `slice_source`, the intrinsic builder spellings, and the index recovery
   (preflight PF-011).
3. **Row-typed storage.** The emitted typedef's `ptr` field is the pointer-to-array declarator for a
   row element (`T (*ptr)[M]`, reusing `pointer_to_array_declarator`,
   `compiler/src/codegen/types.rs:254-267`) and `T *` for a scalar element. The builder stores the
   row pointer directly; the base-element cast pair (`store_cast` and the row cast in `index_text`)
   is removed.
4. **Recovery gate replaced by agreement.** With the view carrying its element, the index-lowering
   recovery always agrees with the emitted view; the `element_recovery_agrees` gate is removed and
   the `ty.is_unknown()` recovery applies to any slice base whose iterated element is known.
5. **Explicit type-argument agreement.** When the call writes an explicit type argument, it must
   equal the source's row element; otherwise the existing `E10200` is reported at the call site
   directly from `lower_intrinsic_call`. Intrinsic registration and the sema conversion table stay
   out of scope (preflight PF-002).

## Implementation Details

### New Types/Interfaces

No new source-level type. The view element is exactly `Ty::iterated_element`
(`compiler/src/types/mod.rs:375-386`): the row for a multi-extent array, the element for a
single-extent one, and `Ty::unknown()` for non-arrays. The implementation may call the method
directly; if a wrapper is added it must be a direct alias with that contract (preflight PF-013).

### New Functions/Methods

- `type_key` slice arm: `format!("slice_{}", type_key(&view_element(...)))`; the `slice_base_element`
  unwrapping helper is removed and its callers use the view element.
- `collect_slice_types`: register the view element keyed by `type_key`, skipping an element that
  mentions a type parameter (concrete instances register their own typedefs).
- `emit_slice_typedefs`: `ptr` spelled by `pointer_to_array_declarator(&element, "ptr", table)`
  (`compiler/src/codegen/types.rs:254-267`) when the element is a fixed array, else
  `{c_type(element)} *`.
- `slice_source`: the stored pointer is the view's element pointer for both fixed and view sources;
  the length stays the outer count (rows).
- `lower_intrinsic_call`: result element = the source's view element; the explicit type argument,
  when present, must equal it (`E10200` otherwise, reported directly at the call site).

### Integration Points

- Ownership (`compiler/src/own/`): element moves, borrow checks, and `E10026` rules are unchanged;
  they now see the correct element type, which is what makes the row move a row move.
- Emission (`compiler/src/codegen/`): the typedef, builder, and access paths share one key and one
  pointer type; the strict `-Werror` host build is the guard that no site regressed to a scalar
  re-derivation.
- Instantiation: a generic body whose view element mentions a parameter registers no typedef; each
  concrete instance registers the substituted row.

## Code Examples

### Example 1: Row slice over a fixed grid

```clorraine
let grid: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
let row: i32[3] = slice<i32[3]>(grid, 0, 2)[1];   // [4, 5, 6]
```

Emitted view type: `ClSlice_arr3_i32 { i32 (*ptr)[3]; usize len; }`.

### Example 2: Nested access and disagreement

```clorraine
let xs: i32[2][2] = [[1, 2], [3, 4]];
let v: i32 = slice<i32[2]>(xs, 0, 2)[1][0];        // 3

// error E10200: cannot convert 'i32[2][2]' to 'i32[]' implicitly
sink(slice<i32>(xs, 0, 2));
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Explicit `slice<T>` disagrees with the source's row element | Report the existing `E10200` at the call site; no new code | AR-9 |
| Row move through a borrow-backed view | Concrete instantiations keep `E10026`; the generic form is a pre-existing escape filed as a follow-up (preflight PF-001) | AR-3 |
| Row move through a view over a statement temporary | The maybe-owning/generic store form moves the row out and empties the source slot (preflight PF-001) | AR-3 |
| Row slice bounds violation | Existing `E11211` runtime trap, unchanged | — |
| Generic body with a parameter-typed row | Skip the typedef registration; concrete instances register the substituted row | AR-3 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity
> Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests ST-1 – ST-5 (`07-testing-strategy.md`): row move through a view over a
  temporary in the filed generic form, concrete borrow-view rejection, nested access, row-typed
  parameter round trip, explicit mismatch.
- Implementation tests: typedef key and pointer declarator, builder spelling, index access
  spelling, `zero_view_helper` key, a nullable row view and an empty `return;` from a
  view-returning function, migrated flat-name pins, byte-identical re-emission of a program without
  multi-extent slices.
- Host-ASan probes: row move, nested access, sub-slice, bounds trap.
- Migrated pins: `compiler/tests/residual_defect_completion_impl_test.rs:380-402`,
  `compiler/src/codegen/types.rs` unit tests for the shared slice key, and the
  `array_value_integrity_impl_test.rs:552-577` row-builder spelling.
