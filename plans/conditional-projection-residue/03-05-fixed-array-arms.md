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
(`static void a(u8 __cl_ret[2])`), so the strict build fails (`too few arguments`). A literal
mixing a `str[2]` field with a `String` field fails the same way.

Route a fixed-array conditional arm through the destination-store shape the discard path already
uses for a direct call: the conditional's destination is one temporary array, each arm fills it
on its path (the call arm passes the hidden destination; the literal arm assigns element-wise),
and the consumer reads the temporary. The direct spelling `a();` keeps its shape byte-identically.

### 2. The conditional-root and `for`-iterable projection (`DEF-275`, AR-11)

`let a: u8[4] = (c ? b1 : b2).xs;` and `for (x in mk().xs)` emit `&(…)` of a member of a
non-lvalue; the batch-H `bind_fixed_array_root` covers direct Member/Index projections, not a
conditional root or a `for` iterable.

- Conditional root: the projection's base is a conditional; the arm the conditional selects owns
  the storage, so the arm's root binds (each arm binds its own root on its path, through the
  existing arm-fill machinery) and the address names a local on each path.
- `for` iterable: route the iterable through the same projection preparation the store and return
  paths use (bind the ownerless root before the loop's iterable value is taken).

Both fixes reuse the destination/root shapes from the direct spellings; no new C construct is
introduced.

## Integration Points

- `codegen`: the conditional-arm store shape (`calls.rs`/`expr.rs`/`function.rs` conditional
  arms) and the fixed-array destination rendering.
- `own/lower.rs`: `bind_fixed_array_root` (:1273), the conditional arm binders (:2711/:2953), and
  the `for`-iterable lowering.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A fixed-array conditional arm | The shared destination temporary, filled per path | AR-11 |
| A conditional root in a fixed-array projection | Each arm's root binds on its path | AR-11 |
| A `for` iterable projecting an ownerless root | The root binds before the iterable is taken | AR-11 |
| The direct spellings | Byte-identical | — |

## Testing Requirements

- Spec: the fixed-array conditional (statement discard and a consuming store), the mixed
  `str[2]`/`String` literal field, the conditional-root projection (store and return), and the
  `for` iterable, all strict-built and run; the direct spellings stay byte-identical.
- Impl: the destination temporary shapes per arm, the bound roots, and the byte-identity controls.
