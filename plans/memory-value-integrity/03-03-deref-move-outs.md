# Raw-Deref Move-Outs: Memory and Value Integrity

> **Document**: 03-03-deref-move-outs.md
> **Parent**: [Index](00-index.md)
> **Rows**: `DEF-437` (critical), `DEF-439` (major), `DEF-440` (critical), `DEF-441` (critical)
> **Phase**: P3 · **Lenses**: correctness + semantics + memory-safety + emission-integrity (AR-31)

## Overview

Four ownership paths around a raw dereference remain incomplete after the batch-M `DEF-375`/
`DEF-376` work: a member projection of a raw pointee is not materialized (`DEF-437`), a guarded
take whose raw pointer aliases the conditional store's destination frees the consumed save
(`DEF-439`), a guarded borrowing intrinsic binds and releases the pointee (`DEF-440`), and a
guarded take of a member over a raw pointer records no pointee move-out (`DEF-441`). The fixes
extend the shared materializer and the existing `value_move_outs`/`MoveOutValue` channel; no new
IR channel, no new rejection (AR-13–AR-16).

## Frozen Rule

- Ch 04 §3 (`spec/04-expressions.md:53`): evaluation is strictly left-to-right.
- Ch 04 §5 (`:108`): only the chosen branch of `?:` evaluates; the `&&`/`||` right operand runs
  only when the left does not fix the result.
- Ch 04 §5 (`:112`): unary `*` yields a place of type `T`; §7 (the raw-dereference operation).
- Ch 16 §2: a read/lend does not move; a borrowing intrinsic does not consume its argument.
- Ch 17 §5 (`spec/17-codegen.md:184-193`): a move is a copy plus a zero of the source; the zeroed
  source is the drop flag; one release per owner.

## Root Cause

1. **Projection of a raw pointee (`DEF-437`).** `materialize_owning_deref_at`
   (`own/lower.rs:3699-3721`) matches only a top-level `Deref`; `let v: String = (*p).s` lowers to
   a `Member` over a `Deref`, so the pointer is not pinned and the pointee is copied without being
   emptied (or the pointer is evaluated twice). `prepare_store` (`own/statements.rs:495`) calls the
   materializer, but the shape does not match; the return/field/take positions share the gap.
2. **Guarded take aliasing the conditional target (`DEF-439`).** The conditional store saves the
   old target and frees the save after the branch (`codegen/function.rs`, the arm store).
   `arm_consumes_target` (`:2400-2418`) scans `Materialize.move_outs` (places) but not
   `value_move_outs` (a raw-deref pointee). The take's value move-out empties/frees the pointee;
   the save still holds the same handle and frees it again.
3. **Guarded borrowing intrinsic (`DEF-440`).** `bind_intrinsic_argument`
   (`own/lower.rs:4171-4242`) binds an owned intrinsic argument into a released temporary. `len`
   is a borrowing intrinsic; a `Deref` argument must stay a read. The take path's non-take rule
   (`own/lower.rs:5390-5401`) already returns a `Deref` unchanged; the intrinsic binding does not.
4. **Guarded take of a member over a raw pointer (`DEF-441`).** `take_pointee_move_outs`
   (`own/call.rs:804-821`) matches only a top-level `Deref`; `(*alt(p,q)).name` is a `Member` over
   a `Deref`, so the call records no `MoveOutValue` and the selected member stays owned after the
   callee freed it.

## Fix Approach

1. **`DEF-437`:** extend the materializer to a projection chain: walk `Member`/`Index` bases to a
   `Deref` root, pin the pointer once (using the existing eager/deferred rule), and keep the
   projection; the consuming store/return/field/take then moves the selected member and empties
   the pointee. Reuse `materialize_owning_deref_at`; do not add a second materializer. Under a
   guard, the fill stays deferred (the batch-M rule).
2. **`DEF-439`:** in `arm_consumes_target`, consult `Materialize.value_move_outs` with the same
   rendered-address comparison the place channel uses. If the consumed value's rendered address
   equals the target, skip the save/free. The runtime comparison is required because the pointer
   may alias or not.
3. **`DEF-440`:** in `bind_intrinsic_argument`, before any binding, return a `Deref` value
   unchanged (the borrowing intrinsic only reads; a later store/return still materializes it).
   Reuse the non-take rule; do not special-case `len` by name.
4. **`DEF-441`:** in `take_pointee_move_outs`, record the pointee move-out for an argument whose
   projection chain roots at a `Deref` (a `Member`/`Index` over `Deref`): push the root `Deref`
   value so the emitter zeroes the selected member through the pointer on the evaluated path.
   The existing `MoveOutValue` and guard placement carry it; no new channel.

## Specification-First Cases (ST-8–ST-11)

All go to `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (AR-30).

| ST | Row | Case | Expected |
|----|-----|------|----------|
| ST-8 | `DEF-437` | The store (`let v: String = (*p).s`), the return, the struct-literal field, and the take argument; the conditional-arm control (`d437b`) | Each moves the pointee once; the read-backs are correct; ASan/LSan clean; the conditional arm stays green |
| ST-9 | `DEF-439` | `v = (g && eat(take (unsafe *p))) ? x : y` with `p = &v`; `g = true` and `g = false`; a pure-pointer variant | Both selections run 0; ASan/LSan clean; the false path leaves v intact; the taken path does not free the consumed save |
| ST-10 | `DEF-440` | `while (i < 2 && len(*alt(p,q)) == 2)`; the `i32(len(...))` spelling; the impure-pointer call counter | Two iterations, no double free; `alt` once per condition evaluation; ASan/LSan clean |
| ST-11 | `DEF-441` | `g && (eat(take (*alt(p,q)).name) == 1)`; `g = true`/`false`; the member read-back from the other element | The taken member is emptied; the false path does not call; the other member stays owned; ASan/LSan clean |

The impl pins assert the single `cl_alt` call, the read/empty pointer identity, the
`MoveOutValue` position inside the evaluated path, the absent false-path call, and a re-emission
determinism pin.

## Verification

- `make check`; the residue suite plain and with `RDC_ASAN=1`.
- The host ASan/LSan runs of the four red shapes and both guard selections; the batch-M
  `m375_*`/`m376_*` controls stay green.
- The emission differential over the P3 probe set and the corpus (AR-32): the delta classes are
  the pointer pin, the `MoveOutValue` placement, and the intrinsic argument shape.

## Risks and Lenses

- **Memory-safety** owns the double-free/UAF matrix and the guard paths; the ASan report is the
  oracle.
- **Emission-integrity** owns the single-call property, the pointer identity, the move-out
  placement inside the guard, and the differential.
- **Semantics** owns the evaluation order (`&&`/`?:` only the chosen path) and the borrow reading
  (an intrinsic read does not consume).
- **Correctness** owns the blast radius: every `materialize_owning_deref` call, every
  `take_pointee_move_outs` callee kind, and every `arm_consumes_target` shape.
- **Risk:** a projection materializer must not move the pointee for a discarded read or a
  non-taking argument; the batch-M `b375_discard`/`b375_arg` controls pin the no-move cases.
- **Risk:** the raw-pointer address comparison must not skip the save for a non-aliasing pointer;
  the false/true selections and the separate-destination variants are the oracle.
