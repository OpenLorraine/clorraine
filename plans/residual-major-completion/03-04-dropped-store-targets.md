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

### 1. The runtime-indexed element/member store (`DEF-367`, critical)

`a[i].y = bump();` with a runtime `i` is accepted and emits no store and no RHS call
(`m367_indexed` runs 191, calls = 1; the local, borrow-parameter, and generic-borrow spellings are
all dropped; the constant-index twin emits). The store lowering routes a target with a dynamic
element to `emit_indexed_store` (`own/expr.rs:2417`), whose `indexed_target` (:2354) accepts only
a top-level `ExprKind::Index` (:2364-2366); a `Member` target over the index returns `None`, and
the assignment is discarded without a report. The probe matrix adds four further failures:
compound `a[i].y += 3` (`m367_compound`, run 253), update `a[i].y++` (`m367_update`, run 255),
the deep chain `a[i].b.y = 7` (`m367_chain`, run 249), and target order
`a[idx].y = set_index()` (`m367_order`, run 247). The generic-borrow owning-element store
`w.items[i] = Elem { ... }` emits but leaks the overwritten owner (LSan `m367_gen_elemstore`),
because `element_ty` is taken from `self.ty(target)` at :2360 and is unknown at the declaration,
so `StorePointer`'s owning/fixed-array branch never runs. The generic-borrow out-of-range index
`setg<Holder>(borrow h, 5)` runs 0 (`m367_bounds`) because the store is dropped before the guard.

Frozen: Ch 04 §4 (`spec/04-expressions.md:87`) — "The target must be a place: a local, a
parameter, a struct field, an array element, or a dereferenced raw pointer, otherwise E10221";
unary `*` at `:112` makes a dereferenced raw pointer a place. `a[i].y` is a struct field of an
array element, so it **is** a place; the rejection fallback in the row note is non-conformant and
declined (AR-6). Ch 04 §3 (`:53`) fixes the order (target location, then RHS; the old value drops
before the store per O3), and Ch 04 §9 (`:98-111`, IX-1) keeps indexing bounds-checked.

Fix: (a) resolve the element/member type per instance — read the substituted place type
(`codegen/expr.rs:290` `resolved_place_ty`) instead of the declaration-time `self.ty(target)`, so
the owning/fixed-array store branch runs for a generic borrow and the overwritten owner drops;
(b) generalize `indexed_target` so the plain store, compound assignment (`+=`), update
(`++`/`--`, through `emit_indexed_update` :2491), and deep member chains all reach the element
address and apply the member projections; the target builder must produce a place for every
supported root, and the `emit_indexed_store` `None` fallback must not silently drop — after the
generalization it is reachable only for a target the checker already rejected with `E10221`, and
the phase proves that (an exhaustive match/debug assertion) or reports the frozen `E10221` at the
target, with no new diagnostic; (c) always materialize the runtime base/index for the new route
so the target location is evaluated before the RHS (the `contains_update` pin is insufficient;
`m367_order` is the case); (d) keep the bounds guard (`E11211`) and the drop order. The local
array, slice, borrow-parameter, and generic-borrow spellings share the shape; the constant-index
twin is the emission reference. The slice spelling is pinned compile-only (AR-33). The
bounds-fault case is in the residue suite with the inline `assert_traps` helper (AR-26), not in
`run_fault_spec_test`.

Expected: the basic program's stores emit; the RHS count is 4; every read-back is 7; run exit 0;
compound/update/deep-chain/target-order all read back their values (0); the generic owning store
is LSan-clean; the out-of-range generic-borrow index traps `E11211`; ASan/LSan-clean. There is no
park fallback: a profile that refuses the route is a phase blocker and user ruling (AR-34(a)).

### 2. The conditional-base member store (`DEF-377`)

`(c ? a : b).f.g = x`, `(c ? a : b).f[i].x = x`, `(c ? a : b).b.ys[i] = x`, and
`(c ? a1 : a2)[i].x = x` are accepted and emit no store (runs 255/249/249/249/255); the
`unsafe`-wrapped forms `(unsafe (c ? a : b)).v.x = 7` and `(unsafe (c ? a : b))[i].x = 7` are
also accepted (run 0). `is_place` (`sema/ops.rs:402`) checks only the shallow kind of a `Member`
base (:423): an outer `Member` over a `Member`/`Index` passes even when the inner root is a
conditional; the `Index` arm (:412) carries a shallow `|| matches!(base.kind, ExprKind::Unsafe(_))`
bypass. Frozen: the same closed place list; a conditional selection is not a place, and the
batch-L `DEF-322` scalar twin precedent reports `E10221`.

Fix: add `ExprKind::Unsafe(inner) => self.is_place(inner)` and **remove** the Index arm's
`|| matches!(base.kind, ExprKind::Unsafe(_))` bypass, then recurse the `Member` arm's base test
through `is_place`. The verified green controls are `s.f.g = 7`, `s.arr[i] = 7`, and the raw
member chain `p.f.g = 7` inside an `unsafe { }` block (all run 0). The prefix spelling
`unsafe p.f = ...` reports `E10221` at the head already and is **not** claimed as accepted. The
dropped store through `(*p).f` is filed as a residual row during execution (a note, not a planned
fix here). Expected: one `E10221` per conditional-rooted store (including the two `unsafe`-wrapped
forms); no TU; the read spelling and the three green controls stay accepted. The rejection is the
frozen rule, not a new surface (AR-7).

## Integration Points

- `compiler/src/own/expr.rs:2274` (`emit_assigned`), `:2354` (`indexed_target`, the Index gate
  :2364-2366), `:2417` (`emit_indexed_store`), `:2491` (`emit_indexed_update`); `element_ty`
  :2360 and `codegen/expr.rs:290` (`resolved_place_ty`); `codegen/function.rs:714` (the
  `StorePointer` branch reads `pointer_ty = substitute(&pointer.ty)`); the element-address
  lowering and the bounds guard (`own/lower.rs:3425` `materialize_operand`).
- `compiler/src/sema/ops.rs:402` (`is_place`), the Index (:412) and Member (:423) arms; the
  assignment check (:354).
- Controls: the constant-index twin in `probes/m367_indexed.clo`, `preflight/m377_controls.clo`,
  `m377_unsafe_ptr.clo`; the batch-L `DEF-322` residue cases.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A member-over-runtime-index store | Emit through the element address + projection; one RHS evaluation; the bounds guard | AR-6 |
| A runtime index out of range | The existing `E11211` trap; the residue-suite `assert_traps` case | AR-6 |
| An owning member overwritten by the store | Drop the old owner before the store (O3) | AR-6 |
| A slice-typed base in the probe | Compile-only emission pin (no local slice value; AR-33) | AR-33 |
| A member-over-index target the builder cannot lower | The fallback reports the frozen `E10221` or is proven unreachable; never a silent drop | AR-6 |
| A conditional-rooted store through `Unsafe` | One `E10221` (the Unsafe arm recurses; the Index bypass is gone) | AR-7 |
| A store whose root is a conditional (member or element) | One `E10221` at the target; no TU | AR-7 |
| A read of the same conditional selection | Accepted (reads need no place) | AR-7 |
| An `unsafe`-wrapped place | Transparent to the recursion; the inner place decides | AR-7 |

## Testing Requirements

- **Spec cases** (`conditional_projection_residue_spec_test.rs`, with the inline `assert_traps`
  helper for the bounds case — the frozen `run_fault_spec_test` data is untouched):
  - `should_store_a_field_of_a_runtime_indexed_element` (`DEF-367`): the local, borrow, generic,
    constant, compound, update, deep-chain, and target-order spellings read back their values and
    run 0; the RHS count is 4 in the basic probe; the generic owning store is LSan-clean; the
    slice emission pin; the out-of-range generic-borrow index traps `E11211` through `assert_traps`.
  - `should_reject_a_conditional_base_member_store` (`DEF-377`): the four/five store shapes and
    the two `unsafe`-wrapped forms each report one `E10221`; the read and the three green controls
    (`s.f.g`, `s.arr[i]`, `p.f.g` inside an `unsafe { }` block) stay accepted.
- **Impl pins** (`conditional_projection_residue_impl_test.rs`): the emitted store sequence and
  the element address (the index retained), the per-instance owning store and its drop, the single
  RHS call, the compound/update forms, the target-order materialization, the bounds guard, the
  `E10221` span per shape (including `Unsafe`), and a re-emission determinism pin.
- **Verification**: `make check`; the focused suite plain and with `CMD="RDC_ASAN=1 cargo test
  --test conditional_projection_residue_spec_test --test
  conditional_projection_residue_impl_test"`; the strict builds; the ASan/LSan leg on the
  `DEF-367` store forms; the residue-suite `assert_traps` bounds case.
- **Risks/lenses**: all four (AR-27). The memory lens owns the store's drop and side-effect
  integrity; the semantics lens owns the place rules; the emission lens runs the base/head
  differential over `m367_*`/`m377_*` and diffs the accepted store shapes (AR-37).
