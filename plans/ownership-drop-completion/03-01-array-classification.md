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
consumers through them.

## Implementation Details

### New Interfaces

```rust
impl Ty {
    /// True for a slice `T[]`: an array view with at least one omitted extent.
    ///
    /// A fixed array (`T[N]`) is an owned aggregate; a slice is a pointer and a
    /// length. The two share `TyKind::Array`, so this is the one place the
    /// distinction lives, and every stage asks here instead of re-deciding.
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
| `is_lend_candidate` (`borrow.rs:175`) | `… \| Array{..}` | keep `Raw`, `Handle`, `Struct("Array")`; `Array` only when `ty.is_slice()` | AR-5 |
| `duplicable_at` (`table.rs:382`) | recurses into the element | a slice is `true` (a view copies); a fixed array recurses into the element | AR-22 |
| `is_handle_or_aggregate` (`borrow.rs:195`) | `Handle \| Struct` | add a fixed array (an owned aggregate); a slice stays out (a view) | AR-5 |

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
- Checker tests that a fixed-array return compiles and that a moved element
  source is `E10020`.
- Emitter tests that a fixed array of duplicable elements copies and a fixed
  array of move-only elements moves.
