# Borrow Out-Slot ABI: Def Backlog Closeout

> **Document**: 03-03-borrow-outslot-abi.md
> **Parent**: [Index](00-index.md)
> **Defects**: DEF-65
> **AR**: AR-3 (AR-10 superseded by preflight PF-002; nullable-struct sub-case corrected at runtime by AR-19)

## Overview

A `borrow` out-slot of a nullable owned type is passed by value, so the callee's write and drop do
not reach the caller's storage: the caller keeps ownership while the callee frees the old value,
and the caller frees it again (double free); for `str?` the callee's write is lost. The fix widens
the shared pointer-parameter predicate to cover nullable borrow out-slots and keys the call-site
pointer decision on the substituted parameter type, with an exact-typed place required for every
out-slot argument. The deliberately by-value duplicable-struct borrows stay unchanged
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
   Nullable borrows of any kind lower to `T *` per `spec/17-codegen.md:65`. Structs cannot be
   nullable (TY-4, `spec/02-types.md:118-124`), so the `ty.nullable` clause covers handles, `raw`,
   `cstring`, and views (including `T[]?` slices) only; a duplicable struct lend therefore always
   keeps its deliberate by-value deviation.
2. The named-call pointer decision (`compiler/src/codegen/calls.rs:132-191`) uses the substituted
   **parameter** type, not the argument type, so a nullable parameter always receives its pointer
   regardless of how the argument was written.
3. An out-slot argument must be an exact-typed place: the caller passes `&place`. An argument that
   would need an implicit conversion or nullability widening into the slot is rejected with the
   existing implicit-conversion diagnostic `E10200`; a converted temporary cannot carry the
   callee's write back to the caller, and accepting one would reintroduce the lost-write bug.
4. The declaration spellings (`func_pointer_params`, signatures, emitted function headers) follow
   the predicate automatically; the component's task list includes a blast-radius pass over all
   four predicate consumers, the `out_slot` condition, and the sema argument check.

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
| A `String` value passed to a `borrow str?`/`String?` out-slot | Rejected: the out-slot needs an exact-typed place; the implicit conversion is not allowed (`E10200`) | AR-3 |
| Null initial slot | Unchanged behavior; the prior exemption case stays pinned | AR-3 |
| Duplicable struct borrow | Unchanged by-value path (W10001); structs cannot be nullable (TY-4/E10011), so no nullable struct out-slot exists | AR-3, AR-19 |
| `func(...)` type with a nullable borrow parameter | `func_pointer_params` follows the same predicate | AR-3 |

> **Traceability:** Every strategy references the Ambiguity Register entry (AR #) that resolved it.
> See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: ST-12 – ST-15 (`07-testing-strategy.md`), including the rejected widened
  argument.
- Implementation tests: emitted declarator/argument shapes for each nullable kind, the freestanding
  compile, and the null-start regression.
- Host-ASan probes: the non-null out-slot double-free program for `String?` and `File?` and the
  `str?` write-through program.
