# Instantiation Checks: Generic Substitution Guards

> **Document**: 03-01-instantiation-checks.md
> **Parent**: [Index](00-index.md)

## Overview

One new module audits the monomorphization set that the ownership stage has
already collected. It re-applies the two declaration-site predicates to the
types that mention a type parameter, once per concrete instance, so the
substituted kind is visible. It changes no other stage: the type checker's
guards, the IR, and the emitter stay as they are.

## Architecture

### Current Architecture

The declaration-site guards decide on the type as written
(`compiler/src/sema/items.rs:39`, `compiler/src/sema/expr.rs:706-708`). The
ownership stage then collects every concrete instantiation:
`generics::collect`, `generics::collect_ir_instances`
(`compiler/src/own/generics.rs:30,439`), producing `Monomorphization`
(`compiler/src/ir/mono.rs:106`) with deduplicated `StructInstance`s and
`Instance`s.

### Proposed Changes

Add `compiler/src/own/instance_check.rs`. `own::check` calls it once, after the
monomorphization is complete and before `OwnershipOutput` is returned. The
module has one entry point and two private checks, and it reports through the
existing `Diagnostics` sink; the driver already skips code generation when the
ownership stage reports an error (`compiler/src/driver.rs:306-309`).

```text
own::check
  ├── ffi::check_items
  ├── program::check_globals / check_struct_declarations
  ├── FunctionChecker per function        → lowered IR
  ├── generics::collect                   → struct + function instances
  ├── generics::collect_ir_instances      → IR-named instances
  └── instance_check::check               → E10205 / E10215   (new)
```

## Implementation Details

### New Types/Interfaces

No new public types. The module exposes one function; `generics.rs` gains one
shared helper and raises one existing helper's visibility (AR-18):

```rust
// compiler/src/own/generics.rs
/// Every struct declaration, keyed by name.
///
/// The instance collectors and the instantiation audit both need the declared
/// fields of a recorded instance; one map keeps that lookup in one place.
pub(super) fn struct_declarations(loaded: &Loaded) -> BTreeMap<&str, &StructDecl>;

/// Collects a declaration's written type-parameter names.
pub(super) fn type_param_names(params: &[TypeParam]) -> Vec<String>;
```

```rust
// compiler/src/own/instance_check.rs
/// Audits the concrete instantiations for the two substitution-blind guards.
///
/// Both declaration-site guards read the type as written, so a field typed `T`
/// or a fill whose element is `T` cannot be judged until a type argument is
/// supplied. This runs once over the collected `Monomorphization` and checks
/// only the declared types that mention a type parameter, so a declaration-site
/// case is never re-reported.
pub fn check(
    loaded: &Loaded,
    program: &ProgramInfo,
    functions: &[ir::Function],
    sema: &SemaOutput,
    mono: &Monomorphization,
    diagnostics: &mut Diagnostics,
);
```

### View-field check (`check_view_fields`)

For each `StructInstance` in `mono.struct_instances()`:

1. Look up the declaration through `generics::struct_declarations`; skip when
   absent (defensive totality).
2. For each field, `resolve_ty` the declared type with the declaration's
   parameters, and skip the field unless `mentions_type_param` is true. This is
   what keeps the audit scoped to substituted types (R3).
3. Substitute the instance arguments with `substitute_instance`.
4. Report `E10205` exactly as the declaration-site guard does — same message,
   same help, span of the field name — when the substituted type is `str` or a
   slice (`matches!(substituted.kind, TyKind::Str) || substituted.is_slice()`).
   Nullability does not change the predicate, so `Wrap<str?>` is rejected too.

Nested instances are covered only when the collector records them:
`collect_struct_instances` expands the instances it records to a fixpoint
(`compiler/src/own/generics.rs:97-109`), but that fixpoint runs before
`collect_ir_instances`, so a nested instance reached only from an
IR-discovered outer instance is not expanded. That gap is out of scope
(AR-19); ST-4 spells its nested instance so the collector records it.

### Fill check (`check_fill_instances`)

For each `Instance` in `mono.instances()`:

1. Resolve the declaration through `program.callee(&instance.function)`; skip
   when it is not a user function.
2. Find the lowered function by `ir::Function::name` in the `functions` slice;
   skip when absent.
3. Walk every statement and terminator value exactly as `collect_ir_instances`
   does (assignments, stores, calls, discards, returns, branch conditions,
   switch scrutinees and labels), carrying the enclosing instruction span.
4. On a `ValueKind::ArrayLit` with a fill, skip unless the literal's checked
   type mentions a type parameter. Substitute the instance arguments and read
   the base element of the resulting `TyKind::Array`. Report `E10215` with the
   declaration-site fill help
   (`a fill cannot repeat a move-only element; construct each element`,
   `compiler/src/sema/expr.rs:713`) at the enclosing instruction span when the
   element is neither unknown nor still a parameter and is not duplicable.

A multi-extent literal keeps its base element plus all extents
(`compiler/src/sema/expr.rs:686-698`), so the base element is exactly what the
declaration-site guard would have tested. Prefix+fill and the fill-only form
share this path. An inferred type argument is already in
`mono.instances()` (`collect_ir_instances` records it), so it is audited the
same way.

```clorraine
// Rejected: the substituted field is a view (E10205).
struct Wrap<T> { v: T; }
let w: Wrap<str> = Wrap<str> { v: "hi" };

// Accepted: the substituted field owns its value.
let w: Wrap<String> = Wrap<String> { v: stringOf("x") };

// Rejected: the substituted fill element is move-only (E10215).
func fillIt<T>(a: T): T[2] { return [; a]; }
let cs: String[2] = fillIt<String>(s);

// Accepted: the substituted fill element is duplicable.
let ns: i32[2] = fillIt<i32>(7);
```

### Integration Points

- `compiler/src/own.rs`: declare `mod instance_check;` and call
  `instance_check::check(loaded, &program, &functions, sema, &monomorphization,
  diagnostics)` after `generics::collect_ir_instances`.
- `compiler/src/own/generics.rs`: add `struct_declarations`, reuse it inside
  `collect_struct_instances`, and raise `type_param_names` to `pub(super)`.
- No change to the IR, the type checker, the emitter, or the driver.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A substituted field is `str` or a slice | `E10205`, declaration-site text and help, field-name span; continue with the next field | AR-3, AR-4 |
| A substituted fill base element is concrete and not duplicable | `E10215`, declaration-site fill text and help, enclosing instruction span; continue | AR-3, AR-2 |
| The substituted element is still a parameter or unknown | Skip; an unresolved element has no concrete kind to judge | AR-7 |
| The declared type is concrete (mentions no parameter) | Skip; the declaration-site guard owns the site, and its error stops the ownership stage before the audit runs | AR-7, AR-17 |
| The fill sits in a lifted lambda body, or the struct instance is not in the recorded set | Not audited; a pre-existing instance-discovery/gap, out of scope | AR-19 |
| A recorded instance has no declaration or no lowered function | Skip; keeps the audit total (cannot occur for a recorded instance) | AR-9 |
| The program already failed the type stage | The ownership stage does not run (`compiler/src/driver.rs:292-302`), so the audit cannot cascade | AR-12 |

> **Traceability:** every error-handling strategy and design choice above
> references the Ambiguity Register entry that resolved it. See
> `00-ambiguity-register.md`.

## Testing Requirements

- Unit tests in `instance_check.rs`: the parameter-mention gate, a
  declaration-named nested instance, a multi-extent fill, a legal duplicable
  fill, and a value reached through a terminator.
- Specification diagnostics in `compiler/tests/ownership_drop_spec_test.rs`
  (ST-1..ST-14).
- Build-and-run controls in `compiler/tests/ownership_drop_impl_test.rs`
  (ST-15, ST-16): a legal instantiation builds and runs cleanly, with no
  double-free abort.
