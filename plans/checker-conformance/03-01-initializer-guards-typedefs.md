# Phase 1: Initializer Guard Operands and Typedef Collectors

> **Document**: 03-01-initializer-guards-typedefs.md
> **Parent**: [Index](00-index.md)

## Overview

Four defects, two mechanisms: the module initializer's guard-operand materialization re-reads an
impure dividend and an impure index base (DEF-136), and the body-side slice-typedef collector
misses three sources — a value's own slice result (DEF-142), a lifted-lambda specialization's
written type argument (DEF-215), and a generic struct-instance element (DEF-216).

## Current Arc

### DEF-136 — the initializer path re-reads two operands

The body path stores every impure guarded operand once before its guard runs:
`own/lower.rs:70-77` materializes the index base and the index, `:207-210` the division and
modulo operands, `:227` the float-to-integer cast operand — through
`materialize_guard_operand` (`own/lower.rs:994-1020`), which allocates a temporary, registers a
pending release for an owning value, and leaves the store placement to the guard walk.

The initializer path re-implements a smaller version. `codegen/program.rs:1428` documents its
operand set — "an impure index, divisor, or cast operand stores once into an init local" — and
the probe matrix confirms the dividend and the index **base** are missing:

```clorraine
let g: i32 = d() / 1;              // guard: d() == MIN && 1 == -1 ... store: d() / 1  → d() twice
let e: i32 = v(sl)[1];             // guard: ... >= (v(sl)).len ... access: v(sl).ptr[1] → v() twice
```

The body twins are verified single at head (`__cl_tmp0 = d();` once; `__cl_tmp0 = v(s);` once),
so this phase fixes the initializer path only and pins the parity.

### DEF-142/215/216 — three missing typedef sources

The body collector (`codegen/program.rs:212-314`) records slice types from locals,
temporaries, signatures, instance substitutions, and written `Intrinsic.type_arg`s (the batch F
walk). Three sources are outside it:

- **DEF-142** — `len(slice(mkS(), 0, 2))`: the slice type is the `slice` builder's **result
  type** (a value's own `ty`), not a written type argument. The emission renders
  `ClSlice_String` and the strict build fails.
- **DEF-215** — a lifted-lambda specialization (`makeSize<i32>()` returning
  `() => sizeof(i32[])`): the specialization body is emitted as `cl_lambda_0__i32`, outside the
  per-instance walk that covers the declared body.
- **DEF-216** — `sizeOf<Wrap<i32>>()`: the slice typedef's element names the generic struct
  instance `ClWrap__i32`, which is never declared because the instance appears only as the
  slice's element.

## Proposed Changes

### 1. Initializer guard operands (DEF-136) — `codegen/program.rs`

Extend the initializer materialization to the two missing operands, reusing the existing
helper that stores an impure operand once:

- the dividend of a `Div`/`Rem` initializer expression, stored before the guard so the guard
  and the operation read one value;
- the index **base** when it is impure (a call returning a view), stored once before the
  bounds guard.

An owning operand (a fixed array of owned elements as an index base) registers with the
existing init-temporary machinery (`init_owners`, the batch F Phase 3 registry) so its
elements release at the init function's end like every other owning init temporary. A numeric
dividend and a view-typed index base own nothing, so the common cases add no release.

### 2. Typedef collector (DEF-142/215/216) — `codegen/program.rs`

- **DEF-142**: in `collect_value_slice_types`, also record the substituted `value.ty` when it
  is a slice — a value's own slice-typed result defines its typedef wherever the value lands.
- **DEF-215**: walk `monomorphization.lifted_specializations()` through the same per-instance
  walk the declared bodies use (body function + substitution), so a specialization's written
  type argument collects.
- **DEF-216**: when a collected slice element is a generic struct instance, ensure the
  instance's own typedef is emitted before the slice typedef (the instance's declaration joins
  the early typedef group, or the collector additionally records the instance's identity).

All three additions only **grow** the typedef set, so unaffected units emit byte-identically
(AR-16); the per-phase corpus differential classifies every delta.

## Tests

| Case | Shape | Expectation |
|------|-------|-------------|
| `initdividend` | the DEF-136 initializer repro | builds strictly; runs; the emission pin shows one `d()` call |
| `initbase` | the index-base initializer repro | builds strictly; runs; one `v()` call |
| `bodyparity` | the two body twins | byte-identical vs the phase baseline |
| `inlinearg` | `len(slice(mkS(), 0, 2))` (DEF-142) | builds strictly; runs; the typedef pin |
| `lambdaspec` | the DEF-215 specialization | builds strictly; the typedef pin |
| `structelement` | `sizeOf<Wrap<i32>>()` (DEF-216) | builds strictly; the instance typedef precedes the slice typedef |
| `initownbase` | an owning index base (a fixed array of `String` from a call) | builds; runs under AddressSanitizer with no leak |

## Acceptance

- The four rows' reproductions build strictly and run; each phase test failed at the phase
  baseline (`ddc42f0` tree) with the exact filed failure.
- Unaffected initializers and bodies emit byte-identically; the corpus differential shows only
  intended typedef and materialization additions.
- The ASan probe of the owning index base shows no leak or double free.
