# Store Lowering and Evaluation Order: Memory and Value Integrity

> **Document**: 03-04-store-lowering-order.md
> **Parent**: [Index](00-index.md)
> **Rows**: `DEF-421` (major), `DEF-423` (major), `DEF-424` (major), `DEF-436` (major)
> **Phase**: P4 · **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-31)

## Overview

Four store paths remain wrong after the batch-M `DEF-367`/`DEF-377` work: a `Deref`-rooted member
store is accepted and dropped (`DEF-421`), the whole-element store reads its target index after
the RHS (`DEF-423`), the fixed-array literal store clears the destination before the RHS reads it
and a constant-index self-copy emits drop + copy + zero (`DEF-424`), and the whole-element/
raw-target stores bypass the owning-deref materializer (`DEF-436`). The fixes complete the store
target builder, the pin rule, the RHS-before-drop ordering, and the materializer call sites. No
new place kind, no new store route, no new IR (AR-17–AR-20).

## Frozen Rule

- Ch 04 §3 (`spec/04-expressions.md:53`): evaluation is strictly left-to-right and deterministic.
- Ch 04 §4 (`:87`): the target must be a place — a local, a parameter, a struct field, an array
  element, or a dereferenced raw pointer — otherwise `E10221`. The unary `*` row (`:112`) makes a
  dereferenced raw pointer a place.
- Ch 04 §9/Ch 08 IX-1 (`spec/08-arrays-strings.md:105`): indexing is bounds-checked; out of range
  faults `E11211`.
- Ch 17 §5 (`spec/17-codegen.md:184-193`): a move is a copy plus a zero of the source; assignment
  to a live owner drops the old target first, then stores.

## Root Cause

1. **`Deref`-rooted member store (`DEF-421`).** `place_target` (`own/place.rs:1931-1990`) has no
   `Deref` arm. `check_assign` (`own/expr.rs:990`) handles a **bare** `*p` target through
   `raw_deref_target` (`:2765`, used at :1003-1073); a `Member`/`Index` over the `Deref` is not a
   raw-deref target and not a place, so `emit_assigned` (`:2280`) gets no place and the store is
   dropped. The `unsafe (*p).f` spelling is newly accepted by the batch-M `Unsafe` place arm and
   dropped the same way. The target's base/index is also not pinned before the RHS (the
   `pin_store_target` path at `:2393-2421` is only reached by the indexed-member route).
2. **Whole-element target order (`DEF-423`).** `indexed_target` (`own/expr.rs:2490-2549`) pins the
   base/index only when `pin_location` is true; `emit_indexed_store` passes
   `contains_update(value)` (`:2566`), which is false for a RHS **call** that changes the index or
   the condition. C does not sequence the assignment operands, so the target can be read after the
   call; the member route already pins (`indexed_member_target` :2361-2371).
3. **Fixed-array clear before RHS (`DEF-424`).** `emit_fixed_array_assign_to`
   (`codegen/function.rs:1533`) saves and clears the destination before the literal's element
   stores when `reads_target` (:1557-1592); a literal element that reads the destination
   (`a[i].names[0]`) then reads cleared storage. The same order breaks the raw self-alias
   `a[i].inner = *(&a[i].inner)`: the RHS move-out zeroes the just-stored target. The
   constant-index sibling `w.items[1] = w.items[1]` is a `Statement::Move`; the emission
   (`:659-690`) skips an identical source/target only for an `ElementTemp` source, so a
   constant-index identical move emits drop, self-copy, and zero.
4. **Owning-deref RHS (`DEF-436`).** The named store calls `materialize_owning_deref`
   (`own/statements.rs:502`). `emit_indexed_store` (`own/expr.rs:2603-2622`) and the raw-deref
   target (`:1055-1076`) lower the RHS via `consume_place`, so an impure `*alt(p,q)` is evaluated
   once for the read and once for the empty, and the wrong pointee is emptied. The raw-target form
   has two `alt` expressions (LHS and RHS); each must run once.

## Fix Approach

1. **`DEF-421`:** route the `Deref`-rooted `Member`/`Index` target through the raw-deref store path
   in `check_assign` (the store-specific route); the target place is the `Deref` root plus the
   member/index projections, so the store renders `(*p).f`/`(*p).arr[i].x`. Pin the target's
   base/index before lowering the RHS with `pin_store_target` (`own/expr.rs:2393-2421`), so
   `(*p).arr[idx] = bump()` stores at the pre-call index and calls `bump` once. Do **not** add a
   `Deref` arm to `place_target` (it broadens every place consumer). Reuse the existing raw-deref
   store emission (pointer pin, old-value save/drop, store, move-out) and keep the bounds guard
   for an indexed member. The `None` fallback in `emit_indexed_store` becomes a debug assertion —
   **no** `E10221` fallback. Keep the structural `.len` `E10221` branch
   (`own/expr.rs:1448-1461`): it is the remaining non-place target the new route must not swallow.
   The conditional-base rejection (`E10221`) is unchanged (batch-M AR-44).
2. **`DEF-423`:** always pin the whole-element base and index before lowering the RHS (the
   `pin_location` argument becomes unconditional for a store); keep the bounds guard and the drop
   order. The target expression is evaluated once; the RHS sees the pinned index. The `at1`
   post-store source read is a separate use-after-move; it is filed as a residual row at the
   phase-4 record, not fixed here.
3. **`DEF-424`:** evaluate the literal's self-reading element expressions into statement
   temporaries **before** the save/clear (the frozen left-to-right order), then perform the
   save/clear and store the temporaries; the save still releases the old owners once after the
   store. The temporaries have one statement lifetime, start zeroed (one zeroed temp array), are
   stored once, and each is moved out once (`emit_move_zero`). The "read the saved copy"
   alternative is not chosen. For the raw self-alias `*(&x)`/`*(&a[i].inner)`, treat the identical
   source/target as a no-op (reuse `is_exact_self_assign`, `own/expr.rs:1481-1493`). For the
   constant-index self-copy, make the `Statement::Move` emission skip the identical source/target
   in both the runtime and constant paths (extend the batch-M F3 rule); the concrete
   constant-index `E10026` is a checker constraint and is unchanged.
4. **`DEF-436`:** call the shared `materialize_owning_deref` on the RHS in `emit_indexed_store`
   and in the raw-deref target path, exactly as `prepare_store` does. The materializer pins the
   pointer once and empties the same pointee; the store then moves the value. Each pointer
   expression runs once (one for the element form, two total for the raw-target form).

## Specification-First Cases (ST-12–ST-15)

All go to `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (AR-30); the bounds
case uses the inline `assert_traps` helper (precedent `borrow_store_integrity_spec_test.rs:363`).

| ST | Row | Case | Expected |
|----|-----|------|----------|
| ST-12 | `DEF-421` | `(*p).f`, `(*p).f.x`, `(*p).arr[i].x`, `unsafe (*p).f`; `(*p).arr[idx].y = bump()` with a call counter; the block controls; the conditional-base rejection (`d421d`); the structural `.len` target | Each store emits and reads back the stored value (run 0); the indexed form stores at the pre-call index and calls `bump` once; the conditional base keeps one `E10221`; the structural `.len` target keeps its `E10221`; the block controls stay green; hosted-amiga compiles the accepted forms |
| ST-13 | `DEF-423` | `a[idx] = mkrow()` (mkrow sets idx = 5); `a[idx] = set_idx() ? a[idx] : Elem {...}` with a destination read-back (`at2` without a source read); the out-of-range store; the member-route control (`d423d`); the `at1` source read as a documented residual | The stores land in the pinned in-bounds element; the conditional self-store is a legal move with a correct destination read; no double free; the out-of-range store traps `E11211` on the plain and ASan builds; the member control unchanged; `at1` is recorded as an accepted use-after-move residual (no fix here) |
| ST-14 | `DEF-424` | `a[i].names = [stringOf(a[i].names[0]), ...]`; the two-element self-read; `a[i].inner = *(&a[i].inner)`; `x = *(&x)`; the generic constant-index self-copy; the concrete constant-index control | The self-read stores run 0 with their values; the self-copy and the raw self-alias are no-ops with the value intact; the concrete `E10026` is unchanged; ASan/LSan clean; the RHS temporaries are zeroed and moved out once |
| ST-15 | `DEF-436` | `xs[i] = *alt(p,q)`; `unsafe *alt(rp,rq) = *alt(p,q)`; the named-store control; the duplicable control | Each pointer expression is called once (one for the element form, two total for the raw-target form); the stored value is correct; ASan/LSan clean; the controls stay green |

The impl pins assert the emitted target projection, the index pin, the store/RHS order, the
self-copy/self-alias skip, the per-pointer-expression call count, and a re-emission determinism
pin.

## Verification

- `make check`; the residue suite plain and with `RDC_ASAN=1`.
- The strict builds; the `assert_traps` bounds case.
- The emission differential over the P4 probe set and the corpus; the accepted store forms
  compile on hosted-amiga.
- The batch-M `m367_*`/`m377_*` controls and the batch-L store controls stay green.

## Risks and Lenses

- **Semantics** owns the place rule (`DEF-421` emits; a conditional base rejects) and the
  evaluation order (`DEF-423`/`424`).
- **Emission-integrity** owns the store shape, the target projection, the pin, the call count, and
  the differential.
- **Memory-safety** owns the old-owner drop, the self-read/self-copy, and the ASan/LSan matrix.
- **Correctness** owns the blast radius: every `place_target` caller, every
  `indexed_target`/`emit_indexed_store` caller, the fixed-array assign forms, and the
  `Statement::Move` emission.
- **Risk:** adding a `Deref` arm to `place_target` broadens every consumer; the conditional-base
  and `E10221` controls are the guard. If a broad arm changes an accepted read, narrow the arm to
  the store path.
- **Risk:** pinning the whole-element target may change the emitted C for an accepted store; the
  corpus and the constant-index twin are the byte-identity oracles (only impl pins may be
  re-pinned).
- **Risk:** RHS-before-clear must not leak the old owners or free them twice; the save/release
  order is the ASan/LSan subject. The raw self-alias is a no-op, not a clear: the `x = *(&x)` and
  `a[i].inner = *(&a[i].inner)` controls pin it.
- **Risk:** the `at1` source read stays accepted after the store; do not let the pin fix "repair"
  it silently. File it as a residual use-after-move row at the phase-4 record and keep ST-13 on
  the destination read (AR-18).
