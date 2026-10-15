# Fixed-Array Conditional Arms and Projections: Conditional, Projection, and Update Residue

> **Document**: 03-05-fixed-array-arms.md
> **Parent**: [Index](00-index.md)

## Overview

Two rows where an accepted fixed-array shape emits invalid C: a conditional whose arms are
fixed-array call results (the hidden out-parameter ABI is not passed), and a fixed-array
projection of a conditional root or of a `for` iterable (the address of a non-lvalue). Ch 17
requires valid C for every accepted shape; the destination-store shape already serves the direct
spelling.

## Design

### 1. The fixed-array conditional arm (`DEF-272`, AR-11)

`func a(): u8[2]` and `func b(): u8[2]`, then `c ? a() : b();` emits
`(void)((c ? cl_a() : cl_b()));`, but the fixed-array return ABI passes a hidden out-parameter
(`static void a(u8 __cl_ret[2])`, the compiler contract of `spec/17-codegen.md:59-84`), so the
strict build fails (`too few arguments`). A literal mixing a `str[2]` field with a `String` field

> **Measured note (2026-10-15):** the static-`str[2]`-field spelling of this claim does not
> reproduce (it is accepted and ASan-clean at the phase base and head); the red the phase pins is the
> **call-filled** field spelling (`Mixed { names: ok ? n1() : n2(), tag: … }`, `too few arguments`).
fails the same way. The deferred fill falls back to a value expression for a fixed-array
conditional (`codegen/guards.rs`), which cannot pass the hidden destination.

Route a fixed-array conditional arm through the per-arm destination store the discard path already
uses for a direct call: the conditional's destination is one temporary array, each arm fills it on
its path as the arm's own statement (the call arm passes the hidden destination; the literal arm
assigns element-wise), and the consumer reads the temporary. The per-arm destination store
replaces the value-expression fallback; the direct spelling `a();` keeps its shape.

### 2. The conditional-root and `for`-iterable projection (`DEF-275`, AR-11)

`let a: u8[4] = (c ? b1 : b2).xs;` and `for (x in mk().xs)` emit `&(…)` of a member of a
non-lvalue; the batch-H `bind_fixed_array_root` covers direct Member/Index projections, not a
conditional root or a `for` iterable.

- Conditional root: the projection's base is a conditional; the arm the conditional selects owns
  the storage, so each arm binds its own root on its path (through the existing arm-fill
  machinery) and the projection renders a per-arm **address** form (`(c ? &b1.xs : &b2.xs)`) — a
  value copy is illegal for a move-only root (the head emits `lvalue required`).
- `for` iterable: route the iterable through the same projection preparation the store and return
  paths use (bind the ownerless root before the loop's iterable value is taken); the iterable
  lowers in `own/loops.rs`.

Both fixes reuse the destination/root shapes from the direct spellings; no new C construct is
introduced.

## Integration Points

- `codegen`: the conditional-arm store shape (`codegen/guards.rs`'s deferred fill,
  `codegen/function.rs`'s arm stores, and the fixed-array destination rendering).
- `own/lower.rs`: `bind_fixed_array_root` (:1279) and the conditional arm binders (:2778/:3077).
- `own/loops.rs`: the `for`-iterable lowering.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A fixed-array conditional arm | The shared destination temporary, filled per path by the arm's own statement | AR-11 |
| A conditional root in a fixed-array projection | Each arm's root binds on its path; the per-arm address form | AR-11 |
| A move-only conditional root | The per-arm address form; never a value copy | AR-11 |
| A `for` iterable projecting an ownerless root | The root binds before the iterable is taken | AR-11 |
| The direct spellings | Re-emit identically | — |

## Testing Requirements

- Spec: the fixed-array conditional (statement discard and a consuming store), the mixed
  `str[2]`/`String` literal field, the conditional-root projection (store and return; a move-only
  root included), and the `for` iterable, all strict-built and run; the direct spellings keep
  their pinned shapes.
- Impl: the destination temporary shapes per arm, the per-arm address form, the bound roots, and
  the re-emission determinism pins.
