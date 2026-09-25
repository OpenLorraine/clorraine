# Ownership and Emission: Generic Field Integrity

> **Document**: 03-03-ownership-codegen.md
> **Parent**: [Index](00-index.md)

## Overview

This component fixes the ownership field read (DEF-10), drop emission for a
generic struct's handle field (DEF-08), and the nested generic zero value
(DEF-09). It also routes the emitter's existing place field read through the
shared helper.

## Architecture

### Current Architecture

- `own/expr.rs::member_type` reads declared fields without substitution
  (`compiler/src/own/expr.rs:204`).
- `codegen/drop.rs::emit_drop_expr` and `emits_drop` iterate declared fields
  without substitution (`compiler/src/codegen/drop.rs:40,69`).
- `codegen/expr.rs::struct_literal` fills an omitted field from the enclosing
  substitution only, so `Pair<Pair<i32>> {}` emits `{ 0, 0 }`
  (`compiler/src/codegen/expr.rs`).
- `codegen/expr.rs::field_type` already substitutes the instance for the DEF-05
  fix; it is consolidated onto `instance_field`.

### Proposed Changes

Route each site through `substitute_instance`/`instance_field`, then apply the
emitter's enclosing-scope `self.substitute` where a generic body can still be
open (AR-14).

## Implementation Details

### Ownership

```rust
/// The type of a field selected from `base`, following a raw pointer.
pub(super) fn member_type(&self, base: &Ty, field: &str) -> Ty {
    match &base.kind {
        TyKind::Struct(..) => self
            .sema.table
            .instance_field(base, field)
            .unwrap_or_else(Ty::unknown),
        TyKind::Raw(inner) => self.member_type(inner, field),
        _ => Ty::unknown(),
    }
}
```

### Drop emission

```rust
TyKind::Struct(name, args) => {
    let params = self.sema.table.struct_type_params(name).unwrap_or(&[]);
    for (field, declared) in fields {
        let field_ty = self.substitute(&substitute_instance(&declared, params, args));
        if !self.needs_drop(&field_ty) {
            continue;
        }
        self.emit_drop_expr(&format!("{c_name}.{field}"), &field_ty);
    }
}
```

`emits_drop` substitutes the same way when it asks whether any field drops.

### Emitter field and zero value

- `codegen/expr.rs::field_type` uses `self.sema.table.instance_field(base, field)`
  and still applies `self.substitute` for an open generic body. It keeps the
  `TyKind::Raw` recursion.
- `struct_literal`'s omitted-field branch uses `self.field_type(ty, field)` so
  the instance's arguments reach `empty_state`; for `Pair<Pair<i32>> {}` the
  result is `{ {0}, {0} }` (AR-12).

### Integration Points

- `needs_drop` already delegates to `TypeTable::duplicable`, so once DEF-07's
  substitution lands, `Box<String>` is correctly recognized as owning.
- The drop path uses the same `instance_field` substitution the checker used, so
  the move analysis and the emitted drop agree.

## Code Examples

### Example 1: Drop of a generic handle

```clorraine
struct Box<T> { value: T; }
func demo(): void {
    let b = Box<String> { value: stringOf("x") };
}
```

The emitted C contains `if (b.value) { FreeString(b.value); } b.value = NULL;`.

### Example 2: Nested generic zero value

```clorraine
let z = Pair<Pair<i32>> {};
```

The emitted C is `(ClPair__Pair__i32){ {0}, {0} }`, not `{ 0, 0 }`.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Open generic body in the emitter | Apply `self.substitute` after the instance substitution | AR-14 |
| Unknown struct/field in the emitter | Existing `Ty::unknown()` fallback | AR-14 |
| Nested generic zero value | Substitute before `empty_state` so a struct field gets `{0}`, not `0` | AR-12 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- `compiler/tests/v1_language_impl_test.rs`: emitted C for `Box<String>` contains
  `FreeString(` on the field path; emitted C for `Pair<Pair<i32>> {}` is the
  nested zero state and compiles.
- `compiler/src/sema/regressions.rs`: `Box<String>` copied twice reports `E10020`.
- A non-generic drop program's emitted C is unchanged (existing suites).
