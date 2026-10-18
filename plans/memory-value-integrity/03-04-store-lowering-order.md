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

1. **`Deref`-rooted member store (`DEF-421`).** `place_target` (`own/place.rs:1931-1976`) has no
   `Deref` arm. `check_assign` (`own/expr.rs:990`) handles a **bare** `*p` target through
   `raw_deref_target` (`:2765`, used at :1003-1073); a `Member`/`Index` over the `Deref` is not a
   raw-deref target and not a place, so `emit_assigned` (`:2280`) gets no place and the store is
   dropped. The `unsafe (*p).f` spelling is newly accepted by the batch-M `Unsafe` place arm and
   dropped the same way.
2. **Whole-element target order (`DEF-423`).** `indexed_target` (`own/expr.rs:2490-2556`) pins the
   base/index only when `pin_location` is true; `emit_indexed_store` passes
   `contains_update(value)` (`:2564`), which is false for a RHS **call** that changes the index or
   the condition. C does not sequence the assignment operands, so the target can be read after the
   call; the member route already pins (`indexed_member_target` :2361-2377).
3. **Fixed-array clear before RHS (`DEF-424`).** `emit_fixed_array_assign_to`
   (`codegen/function.rs:1533`) saves and clears the destination before the literal's element
   stores when `reads_target` (:1556-1605); a literal element that reads the destination
   (`a[i].names[0]`) then reads cleared storage. The constant-index sibling
   `w.items[1] = w.items[1]` is a `Statement::Move`; the emission (`:659-690`) skips an identical
   source/target only for an `ElementTemp` source, so a constant-index identical move emits drop,
   self-copy, and zero.
4. **Owning-deref RHS (`DEF-436`).** The named store calls `materialize_owning_deref`
   (`own/statements.rs:495`). `emit_indexed_store` (`own/expr.rs:2599-2613`) and the raw-deref
   target (`:1060-1070`) lower the RHS via `consume_place`, so an impure `*alt(p,q)` is evaluated
   once for the read and once for the empty, and the wrong pointee is emptied.

## Fix Approach

1. **`DEF-421`:** route the `Deref`-rooted `Member`/`Index` target through the raw-deref store path
   in `check_assign` (the store-specific route); the target place is the `Deref` root plus the
   member/index projections, so the store renders `(*p).f`/`(*p).arr[i].x`. Do **not** add a
   `Deref` arm to `place_target` (it broadens every place consumer). Reuse the existing raw-deref
   store emission (pointer pin, old-value save/drop, store, move-out) and keep the bounds guard
   for an indexed member. The `None` fallback in `emit_indexed_store` becomes a debug assertion —
   **no** `E10221` fallback. The conditional-base rejection (`E10221`) is unchanged (batch-M
   AR-44).
2. **`DEF-423`:** always pin the whole-element base and index before lowering the RHS (the
   `pin_location` argument becomes unconditional for a store); keep the bounds guard and the drop
   order. The target expression is evaluated once; the RHS sees the pinned index.
3. **`DEF-424`:** for a literal whose element expressions read the target, evaluate the element
   expressions into statement temporaries **before** the save/clear (the frozen left-to-right
   order), then perform the save/clear and store the temporaries; the save still releases the old
   owners once after the store. The "read the saved copy" alternative is not chosen. For the
   constant-index self-copy, make the `Statement::Move` emission skip the identical source/target
   in both the runtime and constant paths (extend the batch-M F3 rule); the concrete constant-index
   `E10026` is a checker constraint and is unchanged.
4. **`DEF-436`:** call the shared `materialize_owning_deref` on the RHS in `emit_indexed_store`
   and in the raw-deref target path, exactly as `prepare_store` does. The materializer pins the
   pointer once and empties the same pointee; the store then moves the value.

## Specification-First Cases (ST-12–ST-15)

All go to `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (AR-30); the bounds
case uses the inline `assert_traps` helper (precedent `borrow_store_integrity_spec_test.rs:363`).

| ST | Row | Case | Expected |
|----|-----|------|----------|
| ST-12 | `DEF-421` | `(*p).f`, `(*p).f.x`, `(*p).arr[i].x`, `unsafe (*p).f`; the block controls; the conditional-base rejection (`d421d`) | Each store emits and reads back the stored value (run 0); the conditional base keeps one `E10221`; the block controls stay green; hosted-amiga compiles the accepted forms |
| ST-13 | `DEF-423` | `a[idx] = mkrow()` (mkrow sets idx = 5); `a[idx] = set_idx() ? a[idx] : Elem {...}` with a destination read-back (`at2` without a source read); the out-of-range store; the member-route control (`d423d`) | The stores land in the pinned in-bounds element; the conditional self-store is a legal move with a correct destination read; no double free; the out-of-range store traps `E11211` on the plain and ASan builds; the member control unchanged |
| ST-14 | `DEF-424` | `a[i].names = [stringOf(a[i].names[0]), ...]`; `a[i].inner = *(&a[i].inner)`; the generic constant-index self-copy; the concrete constant-index control | The self-read stores run 0 with their values; the self-copy runs 0 with the value intact; the concrete `E10026` is unchanged; ASan/LSan clean |
| ST-15 | `DEF-436` | `xs[i] = *alt(p,q)`; `unsafe *alt(rp,rq) = *alt(p,q)`; the named-store control; the duplicable control | `alt` is called once; the stored value is correct; ASan/LSan clean; the controls stay green |

The impl pins assert the emitted target projection, the index pin, the store/RHS order, the
self-copy skip, the single `cl_alt` call, and a re-emission determinism pin.

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
  order is the ASan/LSan subject.
