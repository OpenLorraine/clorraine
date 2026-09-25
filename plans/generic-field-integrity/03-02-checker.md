# Checker Field Reads: Generic Field Integrity

> **Document**: 03-02-checker.md
> **Parent**: [Index](00-index.md)

## Overview

This component fixes the overlay walk (DEF-06) and completes the checker side of
DEF-10 by following a `raw` base when typing a member. It also routes the
checker's existing substituted field reads (`field_type`, the dot-call lookup, and
the struct-literal field check) through the shared `instance_field` so the copies
of the zip-substitute block are removed.

## Architecture

### Current Architecture

`Checker::field_type` (`compiler/src/sema/expr.rs`) inlines the zip-substitute
block; `sema/call.rs::dot_call` and the struct-literal field check call it. The
overlay walk `overlay_reached` (`compiler/src/sema/mod.rs:315`) reads declared
fields without substitution, so an overlay behind a generic field is missed.

### Proposed Changes

1. Replace `field_type`'s body with a call to `TypeTable::instance_field`, and
   follow a `TyKind::Raw` base so `r.second` is typed from the pointee.
2. Substitute in `overlay_reached` using the instance's arguments.
3. Leave `dot_call` and the struct-literal field check unchanged except that they
   now inherit the shared helper.

## Implementation Details

### New/Changed Functions

```rust
/// The type of a named field on a struct value, following a raw pointer.
///
/// The instance's type arguments are substituted into the declaration, so
/// `Pair<i32>.first` is `i32`. A raw base peels to its pointee first, so a
/// field reached through a pointer is typed, not left unknown.
pub(super) fn field_type(&self, base_ty: &Ty, field: &str) -> Option<Ty> {
    match &base_ty.kind {
        TyKind::Struct(..) => self.table.instance_field(base_ty, field),
        TyKind::Raw(inner) => self.field_type(inner, field),
        _ => None,
    }
}
```

`overlay_reached` substitutes each field before recursing:

```rust
TyKind::Struct(name, args) => {
    if self.table.is_overlay(name) { return Some(name.clone()); }
    let params = self.table.struct_type_params(name).unwrap_or(&[]);
    self.table.struct_fields(name).and_then(|fields| {
        fields.iter().find_map(|(_, field)| {
            self.overlay_reached(&substitute_instance(field, params, args), depth + 1)
        })
    })
}
```

### Integration Points

- `member_type` (`sema/expr.rs`) calls `field_type`; the raw peel means
  `ExprKind::Member` over a `raw`-typed base is typed, and the existing
  "not a member" report still fires for a struct base that lacks the field.
- The unsafe gate for a raw dereference stays in the ownership stage; the checker
  only types the field (AR-13).

## Code Examples

### Example 1: Overlay behind a generic field

```clorraine
struct Regs { r: volatile u16; }
struct Outer<T> { inner: T; }
func takes(o: Outer<Regs>): void { }   // E10207, as for non-generic Outer
```

### Example 2: Field through a raw pointer

```clorraine
struct Pair<T> { first: T; second: T; }
func demo(p: Pair<u8>): i32 {
    unsafe {
        let r: raw Pair<u8> = &p;
        return takesI32(r.second);   // r.second is u8, so this is E10200
    }
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Overlay reached through a generic field | `E10207`, same message as the non-generic form | AR-9 |
| Unknown field on a struct base | Existing `E10212`, unchanged | AR-19 |
| Raw member access | Typed from the pointee; the unsafe gate remains an ownership concern | AR-13 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- `compiler/src/sema/regressions.rs`:
  - `func takes(o: Outer<Regs>)` reports `E10207`.
  - `Pair<u8>.first` passed where `i32` is required reports `E10200` (already
    covered by the DEF-05 regression; keep it).
  - `r.second` for `r: raw Pair<u8>` is typed `u8` (a following `i32` use is
    `E10200`, not silently accepted).
- `compiler/tests/v1_language_impl_test.rs`: the existing field-bound and
  callable-field cases still pass after `field_type` delegates to `instance_field`.
