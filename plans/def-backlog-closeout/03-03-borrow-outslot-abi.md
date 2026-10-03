# Borrow Out-Slot ABI: Def Backlog Closeout

> **Document**: 03-03-borrow-outslot-abi.md
> **Parent**: [Index](00-index.md)
> **Defects**: DEF-65
> **AR**: AR-3, AR-10

## Overview

A `borrow` out-slot of a nullable owned type is passed by value, so the callee's write and drop do
not reach the caller's storage: the caller keeps ownership while the callee frees the old value,
and the caller frees it again (double free); for `str?` the callee's write is lost. The fix widens
the single pointer-parameter predicate to cover nullable borrow out-slots and materializes view
conversions at the call site. The deliberately by-value duplicable-struct borrows stay unchanged
(AR-3).

## Architecture

### Current Architecture

- `is_pointer_param` (`compiler/src/codegen/types.rs:30-32`) returns
  `Mode::Borrow && matches!(ty.kind, TyKind::Struct(..)) && !table.duplicable(ty)`. A `String?`
  (`Handle`), `File?` (`Handle`), `str?` (`Str`), or `T[]?` (`Array`) never qualifies.
- Four consumers share the predicate:
  - `signature_with_symbol` (`compiler/src/codegen.rs:459-483`, decision `:473-477`);
  - `Emitter::emit_function` (`compiler/src/codegen/function.rs:44-51`);
  - `func_pointer_params` (`compiler/src/codegen/types.rs:101-115`);
  - `call_arguments` (`compiler/src/codegen/calls.rs:132-191`, decision `:165-182`).
- The out-slot kind is decided in `out_slot` (`compiler/src/own.rs:674-699`, condition `:678`:
  `param.mode == Borrow && ty.nullable`); the callee's assignment drops the old target when
  `needs_drop` holds (`compiler/src/codegen/function.rs:965-968`), based on
  `needs_drop(&ty)` (`compiler/src/codegen/drop.rs:261-267`), which ignores nullability.
- `place_base` (`compiler/src/codegen/expr.rs:147-168`) dereferences pointer parameters, so the
  callee side needs no change once the parameter is a pointer.

### Proposed Changes

1. `is_pointer_param` becomes
   `Mode::Borrow && (ty.nullable || (matches!(ty.kind, TyKind::Struct(..)) && !table.duplicable(ty)))`.
   Nullable borrows of any kind lower to `T *` per `spec/17-codegen.md:65`.
2. Call-site conversions (`compiler/src/codegen/calls.rs:203-224`): a pointer-lowered
   `borrow str?` receiving a `String`/`str` value needs the view conversion materialized into a
   temporary before passing its address; a place argument passes `&place`. The same rule applies to
   a nullable slice borrow and a nullable handle borrow.
3. The duplicate spellings (`func_pointer_params`, signatures, emitted function headers) follow the
   same predicate automatically; the component's task list includes a blast-radius pass over all
   four consumers and the `out_slot` condition.

No change is made to `needs_drop`, the drop ladder, or the null-start ownership rule.

## Implementation Details

### Integration Points

- A pointer parameter's `place_base` dereference (`compiler/src/codegen/expr.rs:161-164`) makes
  callee writes and drops operate on the caller's storage.
- The call-argument materialization uses the emitter's existing temporary mechanism; the exact
  helper is chosen during implementation, and the emission-integrity audit verifies the emitted
  shape.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Non-null `borrow String?` / `File?` slot written by the callee | Pointer parameter; callee drop runs on caller storage; caller sees the new value | AR-3 |
| `borrow str?` receives a `String` value | Materialize the view conversion; pass its address | AR-10 |
| Null initial slot | Unchanged behavior; the prior exemption case stays pinned | AR-3 |
| Duplicable struct borrow | Unchanged by-value path (W10001) | AR-3 |
| `func(...)` type with a nullable borrow parameter | `func_pointer_params` follows the same predicate | AR-3 |

> **Traceability:** Every strategy references the Ambiguity Register entry (AR #) that resolved it.
> See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: ST-12 – ST-15 (`07-testing-strategy.md`).
- Implementation tests: emitted declarator/argument shapes for each nullable kind, the freestanding
  compile, and the null-start regression.
- Host-ASan probes: the non-null out-slot double-free program for `String?` and `File?`.
