# Generics: Clorraine v1 language completion

> **Document**: 03-01-generics.md
> **Parent**: [Index](00-index.md)

## Overview

This component completes generics (G1-G8). Generic functions already work; the
missing pieces are generic structs that store a type parameter, callable bounds,
callable-only values, ephemeral type arguments, and the recursion cap over
struct instantiation. The design reuses the existing type representation and
monomorphization rather than adding a parallel mechanism (AR-24, AR-29).

## Architecture

### Current Architecture

- The type stage models an instantiation as `TyKind::Struct(name, args)`
  (`compiler/src/types/mod.rs:53`), so the arguments survive type checking.
- `Monomorphization` records **function** instances only
  (`compiler/src/ir/mono.rs:13-28`), and `own/generics.rs` collects them from
  call sites.
- `c_type` and `type_key` drop the arguments of a struct
  (`compiler/src/codegen/types.rs`), and `emit_struct` writes one
  `struct Cl{name}` (`compiler/src/codegen/program.rs:261-272`).
- `FuncSig.type_params` is `Vec<String>` (`compiler/src/types/table.rs:26`), so a
  parsed callable bound never reaches the checker.

### Proposed Changes

1. Carry the callable bound with each type parameter in the type stage.
2. Record struct instances in `Monomorphization`, alongside function instances.
3. Include struct arguments in `type_key` and `c_type`, and emit one struct per
   instance with substituted fields.
4. Check `E10303`, `E10304`, and `E10306`; keep `E10305` covering structs.

## Implementation Details

### New Types/Interfaces

```rust
/// A struct instantiation to emit, keyed for deduplication.
pub struct StructInstance {
    /// The struct name as declared.
    pub name: String,
    /// The concrete type arguments, in declaration order.
    pub args: Vec<String>,
}

/// A generic parameter with its optional callable bound.
pub struct TypeParamSig {
    /// Parameter name.
    pub name: String,
    /// `func(...)` bound, when declared.
    pub bound: Option<FuncTy>,
}
```

`Monomorphization` gains `struct_instances: Vec<StructInstance>` plus
`record_struct` and `struct_instances()` mirrors of the function methods
(`compiler/src/ir/mono.rs`).

### New Functions/Methods

- `own/generics.rs::collect_struct_instances` walks every module item's resolved
  field, parameter, and return types and records each `TyKind::Struct(name,
  args)` whose arguments are concrete (no `TyKind::Param`), recursing into the
  argument types. `collect_ir_instances` records the struct instances it sees in
  IR value and statement types so a type used only in a body is not missed.
- `codegen/types.rs::type_key` appends a struct's arguments, joined with `_`,
  when the arguments are non-empty (`Pair<i32>` → `Pair_i32`); the argument keys
  come from the same function recursively, so nested arguments compose (R16).
- `codegen/types.rs::c_type` emits `Cl{name}` for a non-generic struct and
  `Cl{name}__{typekey}` for an instantiated one.
- `codegen/program.rs::emit_struct` becomes instance-aware: for a declaration
  with type parameters it emits one struct per recorded instance, substituting
  the parameters into each field type; for a non-generic struct it is unchanged.
- `sema/call.rs::check_generic_call` looks up each declared type parameter's
  bound and, for every bound that is present, verifies the concrete argument is
  a `Func` with the bound's signature (or a closure type, AR-10); otherwise it
  reports `E10303`.
- `sema/call.rs` (and the struct-literal path) reports `E10304` when a value
  whose type is a callable-bound parameter is stored or returned. The checker
  carries the set of callable-bound parameter names for the function being
  checked.
- `sema` reports `E10306` whenever a resolved `Struct` type contains a closure
  type argument, recursively, at a generic call's substituted return type and at
  a generic struct literal.

### Integration Points

- The type stage owns the bound data (`types/table.rs`); the checker reads it
  when binding arguments.
- The monomorphization set is the single input codegen consumes; struct
  instances sit beside function instances.
- The codegen naming path (`type_key`, `c_type`) is the single owner of the C
  symbol, so the emitter, prototypes, and field declarations agree.

## Code Examples

### Example 1: A generic struct stores the parameter

```clorraine
struct Pair<T> { first: T; second: T; }

func demos(): i32 {
    let a = Pair<i32> { first: 1, second: 2 };
    let b = Pair<i32> { first: 3, second: 4 };
    let c = Pair<u8> { first: 1, second: 2 };
    return a.first + b.second + i32(c.first);
}
```

Emits `ClPair__i32` once (deduped for `a` and `b`) and `ClPair__u8` once.

### Example 2: Callable bound and callable-only value

```clorraine
func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);          // f is callable-only: call or pass
}

func keep<T, F: func(T): T>(x: T, f: F): F {
    return f;             // E10304: callable-only value cannot be returned
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Wrong number of type arguments | `E10300` (existing) | AR-15 |
| Arguments cannot be inferred | `E10301` (existing) | AR-15 |
| `export` on a generic declaration | `E10302` (existing, verified) | AR-15 |
| Argument violates a callable bound | `E10303`, naming the parameter and the bound | AR-15 |
| Callable-only value stored or returned | `E10304` | AR-15 |
| Instantiation recursion beyond the cap | `E10305`, extended to structs | AR-18 |
| Generic aggregate with an ephemeral argument | `E10306` | AR-16 |

> **Traceability:** every strategy cites the register entry that resolved it;
> see `00-ambiguity-register.md`.

## Testing Requirements

- Type and codegen unit tests for `type_key`/`c_type` on instantiated structs,
  including nested arguments.
- `Monomorphization` unit tests for struct-instance dedupe.
- Specification tests per the generics ST cases in `07-testing-strategy.md`
  (runnable struct instantiation, nested generics, dedupe, and the three new
  diagnostics).
