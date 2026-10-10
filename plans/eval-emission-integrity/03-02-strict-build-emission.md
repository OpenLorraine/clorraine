# Strict-Build Emission Shapes: Evaluation and Emission Integrity

> **Document**: 03-02-strict-build-emission.md
> **Parent**: [Index](00-index.md)

## Overview

Phase 2 makes ordinary accepted programs build under the strict flags: the structural `len`
binding is skipped (DEF-76/97), owned structs get zero helpers (DEF-80), the freestanding profile
defines `bool` (DEF-90), views coerce to pointers at uncontracted externs (DEF-98), generated
names are seeded into the de-dup pass (DEF-112), complete-row slice typedefs follow the struct
definitions (DEF-120), and a callable-bound field call resolves through the instance path
(DEF-128).

## Architecture

### Current Architecture

- `len` of a fixed array folds to the constant extent (`codegen/calls.rs:235-239`), but a generic
  argument still binds `__cl_owned<N>` (`own/lower.rs:1491-1530` → `materialize_owned_argument
  :1621-1720`) and `unread_locals` counts the argument as a read (`codegen/function.rs:2441`), so
  the `sizeof` marker is not emitted and `-Wunused-but-set-variable` fails.
- `empty_return_value` spells `(ClP){0}` (`codegen/cleanup.rs:922-925`); the view arms already
  return `cl_zero_ClStr`/`cl_zero_ClSlice_*` helpers (`codegen/types.rs:48-62`).
- The freestanding preamble (`codegen/program.rs:88-99`) defines the fixed-width scalars but no
  `bool`, while `scalar_c_type` spells `bool` (`codegen/types.rs:267`).
- `call_arguments` passes a view struct by value to an uncontracted extern (`codegen/calls.rs:148-164,185`),
  whose prototype is `APTR` (`abi::ffi_type_with`, `abi.rs:41-69`); the pointer branch is disabled
  for ffi callees.
- Generated spellings (`__env`, `__cl_ret`, `__cl_tmpN`, `__cl_ownedN`, `__cl_itmpN`) are not
  seeded into `used_names` (`codegen/function.rs:83,185`; `codegen.rs:525-536`).
- Slice typedefs precede the struct definitions (`codegen.rs:368-369`); a multi-extent row element
  spells `ClP (*ptr)[2]` (`codegen/types.rs:238-254`) and needs the complete `ClP`.

### Proposed Changes

1. **Structural `len` (DEF-76/97).** In the intrinsic argument path, an argument to `len` whose
   type is a fixed array (all extents `Some`) skips the owned binding — matching the concrete
   non-owning drop and the compile-time rule (`spec/04-expressions.md:248`). DEF-97 is reproduced
   from `codegen_run_spec_test` first and fixed together when it is the same class.
2. **Struct zero helpers (DEF-80).** Emit `static inline <ClType> cl_zero_<ClType>(void)` per
   emitted struct after its definition (`program.rs:406-417`) and return it from
   `empty_return_value` (`cleanup.rs:922-925`); the view helpers stay.
3. **Freestanding `bool` (DEF-90).** Add `typedef unsigned char bool;` to the freestanding
   preamble (`program.rs:88-99`), matching the compiler's 1-byte bool (`types/table.rs:497`).
4. **Extern view coercion (DEF-98).** When the callee is ffi and the expected parameter's ABI type
   is a pointer, render the view's `.ptr` (the existing `slice_source` shape) at the call site;
   the prototype and the ABI tables are unchanged.
5. **Name seeding (DEF-112).** Seed `__env`, `__cl_ret`, `__cl_tmp0..`, `__cl_owned0..`, and
   `__cl_itmp0..` into `used_names` before params/locals resolve in `emit_function` and
   `emit_lifted_function`; no new rejections — a colliding source name simply de-duplicates.
6. **Typedef split (DEF-120).** `emit_slice_typedefs` splits: pointer-element slices (and their
   zero helpers) stay before `emit_struct_definitions`; slices whose innermost row element is a
   struct are emitted after the definitions (before the closures). The store-emission AR-8 order
   record is revised with the new evidence.
7. **Bound field calls (DEF-128).** The bounded field call resolves through the instance
   field/closure call path: the smaller lever between `func_field_receiver` accepting a
   callable-bound `Param` field (`own/call.rs:111-137`) and a codegen-time re-resolution of the
   receiver-as-first-argument shape via `instance_field` (`codegen/expr.rs:520-589`) is chosen by
   probe; the emitted C calls through the field value, and the concrete-field emission is
   unchanged.

## Implementation Details

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `own/lower.rs` | the `len` argument binding skip (P2's only own-stage change) |
| `own/call.rs` | the bound-field receiver lever (DEF-128, if chosen) |
| `codegen/{calls,types,program,cleanup,function,expr}.rs` | the emission-shape fixes |
| `compiler/runtime/shim_freestanding.c` | untouched (the preamble is the platform-neutral place) |
| the store-emission order pins | updated deliberately with the DEF-120 split |

## Code Examples

```clorraine
// DEF-76: builds under -Werror; the extent is structural
func g<T>(x: T): i32 { return i32(len(mkArr(x))); }

// DEF-80: no compound literal
func empty(): Pair { return; }        // emits cl_zero_ClPair()

// DEF-90: a freestanding bool builds
func flag(): bool { return true; }    // -ffreestanding -Werror green

// DEF-128: the bounded field call builds and runs
struct Holder<T> { call: T; }
func use<T: func(i32): i32>(h: Holder<T>): i32 { return h.call(5); }
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| `len` of a fixed array | accepted; no binding emitted | AR-B13 |
| Owned struct empty return | accepted; helper call | AR-B13 |
| Freestanding bool use | accepted; preamble type | AR-B13 |
| View to an uncontracted extern | accepted; `.ptr` coercion | AR-B13 |
| Source binding named like a generated local | accepted; de-duplicated | AR-B13 |
| Multi-extent row slice of a struct | accepted; typedef after the definition | AR-B13, AR-B16 |
| Callable-bound field call | accepted; instance-resolved emission | AR-B13 |

> **Traceability:** every error-handling strategy back-references its register entry. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification cases ST-8 – ST-16 ([07-testing-strategy.md](07-testing-strategy.md)): strict
  builds and runs for each shape, the freestanding leg for DEF-90, the corpus build for the
  typedef split, the bounded-field run, and the existing accepted controls.
- Implementation pins: the absent `__cl_owned` binding, the `cl_zero_*` spellings, the seeded
  de-dup spelling, the typedef order (pointer-element before definitions, row-element after), and
  byte-identical re-emission where unaffected.
