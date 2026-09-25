# Current State: Generic Field Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The DEF-05 fix made the checker's field access (`sema/expr.rs`), the dot-call
callable-field lookup (`sema/call.rs`), the struct-literal field check, and the
emitter's place tracking (`codegen/expr.rs`) substitute an instance's arguments
into a field type. The substitution logic — build a `BTreeMap` from the struct's
type-parameter names zipped with the instance's arguments, then call
`crate::types::substitute` — is copied at each site. The same pattern also exists
independently in `sema/items.rs::embeds_by_value` and `own/generics.rs`.

Every other stage that reads a struct's declared field type ignores the
instance's arguments, which is the shared root cause of DEF-06…DEF-10.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/types/mod.rs` | Type model and `substitute` | Add `substitute_instance(declared, params, args)` |
| `compiler/src/types/table.rs` | Struct field lookup, duplicability, size/offset | Add `instance_field`; substitute in `duplicable_at`, `size_of_at`, `offset_of` |
| `compiler/src/sema/mod.rs` | Overlay walk | Substitute in `overlay_reached` |
| `compiler/src/sema/expr.rs` | Field access, struct literal | Route field reads through `instance_field` |
| `compiler/src/sema/call.rs` | Dot-call field lookup | Route through `instance_field` |
| `compiler/src/own/expr.rs` | Ownership member type | Substitute in `member_type` |
| `compiler/src/codegen/drop.rs` | Drop emission | Substitute in `emit_drop_expr` / `emits_drop` |
| `compiler/src/codegen/expr.rs` | Struct literal, place field | Substitute the zero-value initializer and the place field |
| `compiler/src/consts/mod.rs` | `offsetof` folding | Pass the instance `Ty` to `offset_of` |

### Code Analysis

The duplicable worker ignores the instance and recurses on the declared field
(`compiler/src/types/table.rs:360`):

```rust
TyKind::Struct(name, _) => self.struct_fields(name).is_none_or(|fields| {
    fields.iter().all(|(_, field)| self.duplicable_at(field, depth + 1))
}),
```

The size worker does the same (`compiler/src/types/table.rs:399`), and the
overlay walk (`compiler/src/sema/mod.rs:315`) substitutes nothing:

```rust
TyKind::Struct(name, _) => {
    if self.table.is_overlay(name) { return Some(name.clone()); }
    self.table.struct_fields(name).and_then(|fields| {
        fields.iter().find_map(|(_, field)| self.overlay_reached(field, depth + 1))
    })
}
```

The drop emitter reads declared fields directly (`compiler/src/codegen/drop.rs:40`),
and the emitter's zero-value initializer applies only the enclosing substitution,
not the instance's (`compiler/src/codegen/expr.rs`, `struct_literal`).

## Gaps Identified

### Gap 1: Overlay bypassed through a generic field (DEF-06)

**Current Behavior:** `func takes(o: Outer<Regs>)` with `Outer<T>{inner:T}` and a
`volatile`-bearing `Regs` is accepted; the non-generic `Outer{inner:Regs}` is
`E10207`.
**Required Behavior:** `E10207`, identical to the non-generic form (`AR-9`).
**Fix Required:** Substitute instance arguments in `overlay_reached`.

### Gap 2: Generic struct with a handle is treated as duplicable (DEF-07)

**Current Behavior:** `let a = Box<String>{…}; let b = a; let c = a;` compiles
with no `E10020`.
**Required Behavior:** A move-only value; the second use is `E10020` (`AR-10`).
**Fix Required:** Substitute in `duplicable_at`.

### Gap 3: Generic struct's handle field is never dropped (DEF-08)

**Current Behavior:** `Box<String>` emits no `FreeString(b.value)`.
**Required Behavior:** The handle is released on scope exit (`AR-11`).
**Fix Required:** Substitute in `emit_drop_expr` and `emits_drop`.

### Gap 4: Nested generic zero value emits invalid C (DEF-09)

**Current Behavior:** `Pair<Pair<i32>> {}` emits `(ClPair__Pair__i32){ 0, 0 }`,
and `clorraine build` fails the C compile.
**Required Behavior:** Valid nested zero states (`AR-12`).
**Fix Required:** Substitute the instance argument in `struct_literal`'s
default-state path.

### Gap 5: Generic `sizeof`/`offsetof`, ownership and raw member reads (DEF-10)

**Current Behavior:** `const N = sizeof(Box<i32>)` is `E10224`;
`offsetof(Box<i32>, value)` does not fold; the ownership `member_type` returns the
bare parameter; `r.second` for `r: raw Pair<u8>` is silently `Unknown`.
**Required Behavior:** Concrete folding and correct field types (`AR-13`).
**Fix Required:** Substitute in `size_of_at`, `offset_of`, `own/expr.rs::member_type`,
and route `raw` member access through the same substitution.

## Dependencies

### Internal Dependencies

- `crate::types::substitute` (existing) — the core replacement used by the helper.
- Call sites in `sema`, `own`, `codegen`, and `consts` (all in the same crate).
- The DEF-05 fix already routed four sites through the checker's `field_type`; the
  plan consolidates those onto the new shared helper.

### External Dependencies

- None. The workspace stays std-only.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| A missed field-read site keeps drifting | Medium | Medium | Grep every `struct_field`/`struct_fields` read and route it through the helper; the review verifies the list |
| A live generic parameter is wrongly substituted in a generic body | Low | High | Zip only the declaration's own parameters; codegen applies the enclosing `self.substitute` afterward (AR-14) |
| Changing `offset_of`'s signature breaks a caller | Low | Low | Only `consts/mod.rs` and `table.rs` unit tests call it; both are updated in the same task |
| Non-generic emitted C changes | Low | High | The helper is a no-op without arguments; the existing suites pin byte output |
