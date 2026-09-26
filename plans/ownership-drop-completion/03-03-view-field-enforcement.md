# View-Field Enforcement: Ownership and Drop Model Completion

> **Document**: 03-03-view-field-enforcement.md
> **Parent**: [Index](00-index.md)

## Overview

This component completes two checks. `E10205` must reject every view-typed struct
field except `static str`, which today means rejecting a slice field as well as a
`str` field. And the array fill form must reject a move-only element, because it
repeats one value and cannot move it more than once. It realizes R7 and R8.

## Architecture

### Current Architecture

`Sema::check_struct` (`compiler/src/sema/items.rs:35-44`) rejects only a field
whose type is exactly `TyKind::Str`; a slice field is accepted and emitted as a
view struct field. `Sema::check_array_literal` (`compiler/src/sema/expr.rs:645-679`)
reports `E10215` for a fill without a fixed extent or an over-long prefix, but it
does not look at the element's duplicability, so `[; a]` for `String[2]` is
accepted and emitted.

### Proposed Changes

1. In `check_struct`, reject a field that is a view: a `str` or a slice.
2. In `check_array_literal`, reject a fill form whose element type is not
   duplicable.
3. Extend the `AL` rule and the `E10215` registry text to name the fill rule.

## Implementation Details

### The View-Field Check (`E10205`)

Replace the `TyKind::Str`-only test with the view test from `03-01`:

```rust
if matches!(ty.kind, TyKind::Str) || ty.is_slice() {
    self.error(
        "E10205",
        format!("view-typed field '{}' is not allowed", field.name),
        "store an owner, or `static str` for literals",
        field.name_span,
    );
}
```

A nullable view field (`str?`, `T[]?`) keeps its `kind`, so it is rejected too.
`cstring` is a raw view that T4 explicitly allows as a field, so it is not
matched; `static str` is not a view. Fixed arrays are owned and are not matched.

### The Fill Guard (`E10215`)

In `check_array_literal`, after the element type is known from context, reject a
fill whose element is move-only (AR-10):

```rust
if fill.is_some() {
    if let Some(element_ty) = result_element.as_ref() {
        if !element_ty.is_unknown()
            && !element_ty.is_param()
            && !self.table.duplicable(element_ty)
        {
            self.error(
                "E10215",
                "array fill requires a fixed extent",
                "a fill cannot repeat a move-only element; construct each element",
                span,
            );
        }
    }
}
```

This mirrors the guards already used for consumption (`consume_place` skips a
duplicable or parameter type), so a fill of a duplicable element is unchanged.
Without a context element type the existing fill error already fires.

### Specification Text

Two small edits make the chapters state the behavior the code now enforces:

- `spec/08-arrays-strings.md` section 2: add the fill rule to the `AL` table (a
  fill value must be duplicable) and extend the `E10215` row in the diagnostics
  table (`spec/08-arrays-strings.md:291`).
- `spec/14-diagnostics.md`: extend the `E10215` description to include "or a fill
  value whose element type is not duplicable". The `Message` column keeps its
  single template (`array fill requires a fixed extent`) and the new trigger
  reuses that text, with the move-only reason in the help, so the registry and
  the `compile_fail_spec_test` message matcher stay consistent (PF-008).

No new diagnostic code is introduced (AR-10, AR-20). `spec/13-data-inclusion.md`
also references E10215 for `embed`; its statement ("never raised by `embed`")
stays true and needs no edit, but the edit note records the cross-reference.

### Integration Points

- `ty.is_slice()` is the predicate from `03-01`; this component depends on it.
- The fill guard reads `result_element`, which `check_array_literal` already
  computes from the context type.
- The existing `E10205` golden (`spec/tests/compile-fail/E10205_str_field.clo`)
  keeps its code and stays valid; no new `spec/` golden is added (AR-12).

## Code Examples

```clorraine
struct Entry {
    name: String;      // owned
    tag: static str;   // literal: allowed
    path: cstring;     // FFI pointer: allowed by T4
    row: u8[8];        // fixed array: owned, allowed
}

struct BadStr   { part: str; }     // E10205
struct BadSlice { part: u8[]; }    // E10205

func fillDemo(a: String): void {
    // let arr: String[2] = [; a];   // E10215: fill cannot repeat a move-only value
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| `str` or slice field | existing `E10205`, extended to slices | AR-4 |
| `cstring` or `static str` field | accepted (T4) | AR-4 |
| A fill of a move-only element | existing `E10215`, new message and chapter rule | AR-10 |
| A fill of a duplicable element | unchanged | AR-10 |

> **Traceability:** the design choices above cite the Ambiguity Register entries
> that resolved them — see `00-ambiguity-register.md`.

## Testing Requirements

- Checker tests: `struct { s: str; }` and `struct { xs: u8[]; }` are `E10205`;
  `struct { c: cstring; l: static str; a: u8[4]; }` is accepted.
- Checker tests: `String[2] = [; a]` and `String[2] = [a; b]` are `E10215`;
  `u8[4] = [; 0]` is accepted.
- A spec-text consistency check: `./spec/check.sh` passes with the extended
  `E10215` text.
