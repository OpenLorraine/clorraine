# Array Classification: Ownership and Drop Model Completion

> **Document**: 03-01-array-classification.md
> **Parent**: [Index](00-index.md)

## Overview

This component gives the compiler one way to tell a slice from a fixed array and
routes every consumer through it, then makes an array literal consume its
elements the way a struct literal already does. It realizes R1, R2, and R4.

## Architecture

### Current Architecture

A slice `T[]` and a fixed array `T[N]` are both `TyKind::Array { element,
extents }`; an omitted extent (`None`) marks a slice. Four predicates currently
ignore that difference and match the whole `TyKind::Array`:

| Site | Predicate | Consequence |
| ---- | --------- | ----------- |
| `borrow.rs:151` | `is_view` | a fixed array is a view, so `needs_drop` is false and its elements are copied in loans |
| `borrow.rs:162` | `is_elided_view` | a fixed-array return needs an elision candidate (`E10036`) |
| `borrow.rs:175` | `is_lend_candidate` | a fixed array is treated as a reaching view |
| `types/table.rs:382` | `duplicable_at` | a slice of a move-only element is wrongly move-only |

`borrow::is_handle_or_aggregate` (`borrow.rs:195`) also omits arrays, so a fixed
array is not kept away from an uncontracted extern (X4).

### Proposed Changes

Add two predicates to `Ty` beside the existing shape helpers, then route the
shape-sensitive consumers through them: `is_view`, `is_elided_view`,
`duplicable_at`, `is_handle_or_aggregate`, and `is_nullable_capable`.
`is_lend_candidate` stays unchanged (a fixed array is borrowable storage).

The predicate is the single decision for the array shape. Other stages still
test the extents inline (`codegen/calls.rs:57,156`, `codegen/render.rs:32`,
`codegen/guards.rs:331`, `codegen/cleanup.rs:329`, `own/lower.rs:499,601`,
`codegen/program.rs:840`, `abi.rs:58`); they already use the same
`all(Some)`/`any(None)` test and are left as they are, but the plan does not claim
that every stage calls the predicate (PF-011).

A fixed-array **return** is not handled by classification alone: C cannot return
an array. Its by-value lowering is designed in
[03-04](03-04-fixed-array-return.md) (PF-001).

## Implementation Details

### New Interfaces

```rust
impl Ty {
    /// True for a slice `T[]`: an array view with at least one omitted extent.
    ///
    /// A fixed array (`T[N]`) is an owned aggregate; a slice is a pointer and a
    /// length. The two share `TyKind::Array`, so this is the one place the
    /// distinction lives for the view, elision, nullability, and aggregate
    /// consumers; they ask here instead of re-deciding.
    pub fn is_slice(&self) -> bool {
        matches!(&self.kind, TyKind::Array { extents, .. } if extents.iter().any(Option::is_none))
    }

    /// True for a fixed array `T[N]` with every extent present.
    pub fn is_fixed_array(&self) -> bool {
        matches!(&self.kind, TyKind::Array { extents, .. } if extents.iter().all(Option::is_some))
    }
}
```

### Consumer Changes

| Consumer | Today | After | AR |
| -------- | ----- | ----- | -- |
| `is_view` (`borrow.rs:151`) | `Str \| CString \| StaticStr \| Array{..}` | `Str \| CString \| StaticStr`, plus `Array` when `ty.is_slice()` | AR-5 |
| `is_elided_view` (`borrow.rs:162`) | `Str \| CString \| Array{..}` | `Str \| CString`, plus `Array` when `ty.is_slice()` | AR-5 |
| `is_lend_candidate` (`borrow.rs:175`) | `… \| Array{..}` | **unchanged** — a fixed array is borrowed storage a returned view may borrow, so it stays a candidate (PF-006) | AR-5 |
| `duplicable_at` (`table.rs:382`) | recurses into the element | a slice is `true` (a view copies); a fixed array recurses into the element | AR-22 |
| `is_handle_or_aggregate` (`borrow.rs:195`) | `Handle \| Struct` | add a fixed array (an owned aggregate); a slice stays out (a view) | AR-5 |
| `is_nullable_capable` (`borrow.rs:203`) | `… \| Array{..}` | `Array` only when `ty.is_slice()`; a fixed array is not nullable (PF-004) | AR-23 |

`needs_drop` (`codegen/drop.rs:98-104`) consults `is_view`, so it becomes correct
for fixed arrays without a direct edit.

### Array-Literal Element Consumption

An array literal owns its elements, so each owned element moves into the array
exactly like a struct-literal field (AR-6, `spec/08-arrays-strings.md:164-171`):

- `compiler/src/own/expr.rs:157-164` — after `self.eval(element)`, call
  `self.consume_owned_place(element, element.span)`, mirroring the struct-literal
  arm at `:151-156`.
- `compiler/src/own/lower.rs:170-175` — map each element through
  `self.consume_place(value)`, mirroring the struct-literal arm at `:158-168`.

`consume_place` / `consume_owned_place` already skip a duplicable element and a
type that mentions a generic parameter, so a copy stays a copy and an
instantiation is left to the monomorphized body. The fill element is **not**
consumed here; a non-duplicable fill is rejected by `03-03` (R7).

### Integration Points

- The ownership walk (`own/expr.rs`) and the lowering (`own/lower.rs`) must agree
  on which elements move; both changes are made together in one task.
- `place.rs` and `statements.rs` read `is_view` for loan and return handling, so
  the classification change reaches them without further edits.

## Code Examples

```clorraine
import { String, stringOf } from std.text;

func pair(): String[2] {                 // by-value return, not a view return
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    return [a, b];                       // a and b move into the array
}

func demo(): void {
    let a: String = stringOf("a");
    let arr: String[1] = [a];            // a moves into arr
    // use(a);                           // E10020
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Reading a moved element source after the literal | existing `E10020` (`spec/16-borrow-rules.md:204`) | AR-6 |
| Moving a nested place out of an owned array | existing `E10026`, unchanged (element moves stay deferred) | AR-1 |
| Fixed array passed to an uncontracted extern | existing FFI rule through `is_handle_or_aggregate` (X4) | AR-5 |

> **Traceability:** the design choices above cite the Ambiguity Register entries
> that resolved them — see `00-ambiguity-register.md`.

## Testing Requirements

- Unit tests on `Ty::is_slice` / `Ty::is_fixed_array` for slices, fixed arrays,
  multi-dimensional arrays, and empty extents.
- A regression test that a view return derived from a fixed-array parameter is
  still accepted (`func head(arr: u8[2]): u8[] { return slice(arr, 0, 1); }`),
  which pins that `is_lend_candidate` keeps fixed arrays (PF-006).
- A checker test that a fixed array is not nullable: `let a: String[2]? = null;`
  is rejected (PF-004).
- Checker tests that a moved element source is `E10020`; the fixed-array return
  is covered by [03-04](03-04-fixed-array-return.md).
- Emitter tests that a fixed array of duplicable elements copies and a fixed
  array of move-only elements moves.
