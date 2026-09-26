# Current State: Generic Substitution Guards

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

- **Declaration-site view-field guard:** `Checker::check_struct` rejects a field
  whose declared type is `str` or a slice (`compiler/src/sema/items.rs:39`). A
  field typed `T` passes, because the parameter is not a view.
- **Declaration-site fill guard:** `Checker::check_array_literal` rejects a fill
  whose context element is not duplicable, but deliberately skips a
  type-parameter element (`compiler/src/sema/expr.rs:706-708`).
- **Instance collection:** the ownership stage collects every concrete
  instantiation. `generics::collect_struct_instances`
  (`compiler/src/own/generics.rs:54`) records struct instances from declarations
  and expands each instance's fields to a fixpoint; `collect_ir_instances`
  (`compiler/src/own/generics.rs:439`) records function and struct instances
  named by the lowered IR, including inferred type arguments. The result is
  `Monomorphization` (`compiler/src/ir/mono.rs:106`).
- **Substitution and classification helpers:** `substitute_instance`
  (`compiler/src/types/mod.rs`), `mentions_type_param`
  (`compiler/src/own/lower.rs:698`), and `TypeTable::duplicable`
  (`compiler/src/types/table.rs:362`) already exist and are used by the type,
  ownership, and codegen stages.
- **Pipeline gate:** `own::check` runs only on a program that passed sema
  (`compiler/src/driver.rs:292-302`); a diagnostic it reports prevents code
  generation (`compiler/src/driver.rs:306-309`).

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/sema/items.rs` | Declared view-field guard | None |
| `compiler/src/sema/expr.rs` | Declared fill guard | None |
| `compiler/src/own/generics.rs` | Instance collection | Add a shared `struct_declarations` map helper |
| `compiler/src/own/instance_check.rs` | — | New: the instantiation-time audit |
| `compiler/src/own.rs` | Ownership stage entry | Declare the module and call the audit |
| `compiler/src/ir/mono.rs` | `Monomorphization` | None (read-only input) |
| `compiler/tests/ownership_drop_spec_test.rs` | Diagnostic specification suite | Add the `ST-` cases |
| `compiler/tests/ownership_drop_impl_test.rs` | Build-and-run implementation suite | Add the accepted-program controls |

### Code Analysis

The view-field guard decides on the type as declared, so a parameter hides the
substituted kind:

```rust
// compiler/src/sema/items.rs:38-46 (abridged)
let ty = self.resolve_type(&field.ty);
if matches!(ty.kind, TyKind::Str) || ty.is_slice() {
    self.error("E10205", format!("view-typed field '{}' is not allowed", field.name), ...);
}
```

The fill guard skips a parameter element, because duplicability is unknown at
the declaration:

```rust
// compiler/src/sema/expr.rs:704-717 (abridged)
if fill.is_some() {
    if let Some(element_ty) = result_element.as_ref() {
        if !element_ty.is_unknown() && !element_ty.is_param()
            && !self.table.duplicable(element_ty) {
            self.error("E10215", "array fill requires a fixed extent", ...);
        }
    }
}
```

## Gaps Identified

### Gap 1: `Wrap<str>` / `Wrap<u8[]>` compile (RV-401)

**Current Behavior:** `struct Wrap<T> { v: T; }` used as `Wrap<str>` or
`Wrap<u8[]>` compiles and emits a C struct whose field is a view. Reproduced on
the current compiler:

```clorraine
let w: Wrap<str> = Wrap<str> { v: "hi" };          // accepted
let b: Wrap<u8[]> = Wrap<u8[]> { v: bytes(s) };    // accepted
```

**Required Behavior:** `E10205` at the substituted field, because the frozen
rule bans a view-typed field (`spec/02-types.md:184`).

**Fix Required:** audit the recorded struct instances; substitute each field
that mentions a type parameter and apply the same predicate the declaration-site
check uses (R1).

### Gap 2: `fillIt<String>` compiles and double-frees (RV-402)

**Current Behavior:** the generic fill is accepted and the emitted C aliases one
handle into both slots. Reproduced on the current compiler; the built program
aborts under AddressSanitizer with
`attempting double-free on 0x... in thread T0`. `fillIt<i32>` is also accepted
and runs correctly (control).

**Required Behavior:** `E10215` for the non-duplicable instantiation; the
duplicable instantiation stays accepted (`spec/08-arrays-strings.md:50`, AL-1).

**Fix Required:** audit the recorded function instances; substitute a fill
literal's type that mentions a type parameter and check the substituted base
element for duplicability (R2).

## Dependencies

### Internal Dependencies

- `Monomorphization` must be collected before the audit runs; `own::check`
  already orders `generics::collect`, then `generics::collect_ir_instances`.
- `sema.table` owns `duplicable`, `struct_fields`, and `struct_type_params`.
- The audit reports through the existing `Diagnostics` sink, so the driver's
  codegen gate handles rejection.

### External Dependencies

None. The workspace is std-only; no crate, build script, or toolchain change.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| A duplicate diagnostic for a concrete illegal program | Low | Low | The declaration-site error stops the stage before the audit runs (`compiler/src/driver.rs:292-302`), and the audit only examines parameter-mentioning declared types (R3, AR-7); the parameter-mention gate is covered by module unit tests |
| Over-rejection of a legal program | Low | High | Controls `Wrap<String>`, `Wrap<static str>`, `Wrap<Buffer>`, `fillIt<i32>`, inferred duplicable fills (ST-6, ST-12) |
| An unrecorded instance (discarded-call result type, IR-only nested instance, lifted lambda body) is expected to be rejected | Low | High | Out of scope (AR-19); those programs already fail at the host C compile today. The audit covers exactly the recorded set, matching the structs codegen emits (`ClWrap__str` exists only when `Wrap<str>` is recorded) |
| Duplication between the two audits and existing walks | Low | Low | `check_fill_instances` walks values like `collect_ir_instances` already does; the shared `struct_declarations` helper avoids a second decl map (AR-18) |
