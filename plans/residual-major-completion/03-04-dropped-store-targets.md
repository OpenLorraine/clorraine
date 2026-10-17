# Dropped Store Targets: Residual Major Completion

> **Document**: 03-04-dropped-store-targets.md
> **Parent**: [Index](00-index.md)

## Overview

Two rows are store-target gaps: the critical runtime-indexed element field store is silently
dropped (the store and its RHS side effect vanish), and a member store over a conditional base is
accepted and dropped instead of reporting the frozen place rejection. The first is an emission
fix the frozen place list requires; the second is the `E10221` rejection the scalar twin already
reports.

## Design

### 1. The runtime-indexed element field store (`DEF-367`, critical)

`a[i].y = bump();` with a runtime `i` is accepted and emits no store and no RHS call (`calls ==
0`, run 251 in the filed probe; the reconstructed probe runs 191 with the local, borrow-parameter,
and generic-borrow spellings all dropped; the constant-index twin emits). The store lowering
routes a target with a dynamic element to `emit_indexed_store` (`own/expr.rs:2417`), whose
`indexed_target` (:2354) accepts only a top-level `ExprKind::Index` (:2363-2368); a `Member`
target over the index returns `None`, and the assignment is discarded without a report.

Frozen: Ch 04 §4 (`spec/04-expressions.md:87`) — "The target must be a place: a local, a
parameter, a struct field, an array element, or a dereferenced raw pointer, otherwise E10221."
`a[i].y` is a struct field of an array element, so it **is** a place; the rejection fallback in
the row note is non-conformant and declined (AR-6). Ch 04 §3 (`:53`) fixes the order (target
location, then RHS; the old value drops before the store per O3), and Ch 04 §9 keeps indexing
bounds-checked.

Fix: generalize `indexed_target` to a member chain rooted at a runtime `Index`: lower the root
index to its element place (the existing base/index materialization and `&element`), apply each
member projection to that place, and emit the store through the final member's address. The
element address keeps the runtime index, so the emitter's bounds guard (`E11211`) is retained;
the target location is evaluated before the RHS, and the RHS runs exactly once. For an owning
member, the overwritten owner drops before the store (O3). The local array, slice, borrow
parameter, and generic-borrow spellings share the shape; the constant-index twin is unchanged and
is the emission reference. The slice spelling is pinned compile-only (AR-33).

Expected: the reconstructed program's local, borrow-parameter, generic-borrow, and constant
stores emit; the RHS count is 4; every read-back is 7; run exit 0; ASan/LSan-clean. A runtime
index outside the element count still traps `E11211` (the `run_fault` case).

### 2. The conditional-base member store (`DEF-377`)

`(c ? a : b).f.g = x`, `(c ? a : b).f[i].x = x`, `(c ? a : b).b.ys[i] = x`, and
`(c ? a1 : a2)[i].x = x` are accepted and emit no store (runs 249/255). `is_place`
(`sema/ops.rs:402`) checks only the shallow kind of a `Member` base (:427): an outer `Member`
over a `Member`/`Index` passes even when the inner root is a conditional. Frozen: the same closed
place list; a conditional selection is not a place, and the batch-L `DEF-322` scalar twin
precedent reports `E10221`.

Fix: recurse the `Member` arm's base test through `is_place`, keeping the `Unsafe` wrapper
transparent to the recursion (so `unsafe *p` and `unsafe p.f` stay places while
`unsafe (c?a:b)` does not). The `Index` arm keeps its recursion. Expected: one `E10221` per store
at the target; no TU; the read spelling and the ordinary member/element stores (`a[i].y`,
`s.f.g`, `*p.f`, `unsafe p.f`) stay accepted. The rejection is the frozen rule, not a new
surface (AR-7).

## Integration Points

- `compiler/src/own/expr.rs:2274` (`emit_assigned`), `:2354` (`indexed_target`), `:2417`
  (`emit_indexed_store`); the element-address lowering and the bounds guard
  (`own/lower.rs:3425` `materialize_operand`, `own/expr.rs` the pointer build).
- `compiler/src/sema/ops.rs:402` (`is_place`), the `Member`/`Index` arms (:410-434); the
  assignment check (`:352`).
- Controls: the constant-index twin in `probes/m367_indexed.clo`, `probes/m377_control.clo`; the
  batch-L `DEF-322` residue cases.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A member-over-runtime-index store | Emit through the element address + projection; one RHS evaluation; the bounds guard | AR-6 |
| A runtime index out of range | The existing `E11211` trap (the `run_fault` case) | AR-6 |
| An owning member overwritten by the store | Drop the old owner before the store (O3) | AR-6 |
| A slice-typed base in the probe | Compile-only emission pin (no local slice value; AR-33) | AR-33 |
| A store whose root is a conditional (member or element) | One `E10221` at the target; no TU | AR-7 |
| A read of the same conditional selection | Accepted (reads need no place) | AR-7 |
| An `unsafe`-wrapped place | Transparent to the recursion; the inner place decides | AR-7 |

## Testing Requirements

- **Spec cases** (`conditional_projection_residue_spec_test.rs`; `run_fault_spec_test.rs` for the
  bounds fault):
  - `should_store_a_field_of_a_runtime_indexed_element` (`DEF-367`): the local, borrow, generic,
    and constant spellings read back 7, count the RHS (4), run 0, and are ASan/LSan-clean; the
    slice emission pin; the out-of-range trap `E11211`.
  - `should_reject_a_conditional_base_member_store` (`DEF-377`): the four/five store shapes each
    report one `E10221`; the read and ordinary store controls stay accepted.
- **Impl pins** (`conditional_projection_residue_impl_test.rs`): the emitted store sequence and
  the element address (the index retained), the single RHS call, the bounds guard, the drop order
  for an owning member, the `E10221` span per shape, and a re-emission determinism pin.
- **Verification**: `make check`; the focused suite; the strict builds; the ASan/LSan leg on the
  `DEF-367` store forms; the `run_fault` bounds case.
- **Risks/lenses**: all four (AR-27). The memory lens owns the store's drop and side-effect
  integrity; the semantics lens owns the place rules; the emission lens diffs the accepted store
  shapes.
