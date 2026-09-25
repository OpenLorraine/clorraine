# Types Substitution Foundation: Generic Field Integrity

> **Document**: 03-01-types-substitution.md
> **Parent**: [Index](00-index.md)

## Overview

This component owns the single substitution helper and the type-table queries
that read a struct's fields. It fixes DEF-07 (duplicability) and the
`sizeof`/`offsetof` half of DEF-10, and it provides the `instance_field`
convenience the checker, ownership, and emitter components call.

## Architecture

### Current Architecture

`duplicable_at` (`compiler/src/types/table.rs:360`), `size_of_at`
(`.../table.rs:399`), and `offset_of` (`.../table.rs:412`) match
`TyKind::Struct(name, _)` and recurse on the declared field types, discarding the
instance's arguments. `offset_of` takes a struct name only, so it cannot see an
instance at all.

### Proposed Changes

Add one pure helper and one table query, then use them in the three type-table
reads so an instance's arguments are applied before a field is inspected.

## Implementation Details

### New Functions/Methods

```rust
/// Substitutes a generic instance's type arguments into a declared type.
///
/// `declared` is written in terms of `params`; `args` are the instance's
/// arguments in the same order. A missing argument leaves the parameter in
/// place, so a partially written type is never corrupted. This is the one
/// place an instance's arguments are mapped onto a declaration.
pub fn substitute_instance(declared: &Ty, params: &[String], args: &[Ty]) -> Ty
```

```rust
impl TypeTable {
    /// The declared type of a struct field with the instance's arguments
    /// substituted. Returns `None` when the struct or field is unknown.
    pub fn instance_field(&self, base: &Ty, field: &str) -> Option<Ty>

    /// The byte offset of a struct field, substituting a generic instance's
    /// arguments into each preceding field's size. `base` is the instance type.
    pub fn offset_of(&self, base: &Ty, field: &str) -> Option<usize>
}
```

`substitute_instance` returns `declared` unchanged when `params` or `args` is
empty; otherwise it builds `BTreeMap<String, Ty>` from `params.zip(args)` and
calls the existing `substitute`. `instance_field` extracts the declaration's own
parameter names via `struct_type_params` and delegates to `substitute_instance`.

`duplicable_at` and `size_of_at` substitute each field before recursing:

```rust
TyKind::Struct(name, args) => self.struct_fields(name).is_none_or(|fields| {
    let params = self.struct_type_params(name).unwrap_or(&[]);
    fields.iter().all(|(_, field)| {
        self.duplicable_at(&substitute_instance(field, params, args), depth + 1)
    })
}),
```

### Integration Points

- `sema/expr.rs`, `sema/call.rs`, `own/expr.rs`, and `codegen/expr.rs` call
  `instance_field` (see `03-02` and `03-03`).
- `consts/mod.rs` passes the resolved instance `Ty` to the new `offset_of`.

## Code Examples

### Example 1: Generic sizeof folds

```clorraine
struct Box<T> { value: T; }
const N: usize = sizeof(Box<i32>);   // folds to 4; today E10224
```

### Example 2: Generic offsetof folds

```clorraine
struct Pair<T> { first: T; second: T; }
const OFF: usize = offsetof(Pair<i32>, second);   // folds to 4
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Empty or short argument list | Leave the parameter unchanged; unreachable after `E10300`/`E10301` | AR-15 |
| Unknown struct or field | `instance_field`/`offset_of` return `None`; the caller keeps its current fallback | AR-14 |
| Self-referential declaration | Existing depth limit (32) unchanged | AR-14 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Unit tests in `compiler/src/types/table.rs` (`#[cfg(test)] mod tests`):
  `duplicable_at` for `Box<String>` is `false`; `size_of`/`offset_of` for a
  generic instance are concrete; `offset_of` for a non-generic struct is
  unchanged; the existing non-generic tests still pass.
- The shared helper's empty-argument no-op is covered by an existing non-generic
  call path.
