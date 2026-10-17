# Deref and Take Channels: Residual Major Completion

> **Document**: 03-06-deref-take-channels.md
> **Parent**: [Index](00-index.md)

## Overview

Two rows are move-out gaps around a raw dereference: an owning deref read through an impure
generic pointer expression evaluates that expression twice and empties the wrong pointee (a double
free), and a `take` of a non-place deref source inside a guarded `&&` operand keeps the inline
call and never empties the pointee (a double free). Both fixes extend the existing
materialization and deferred move-out channels; neither adds a rejection.

## Design

### 1. The owning deref source's single materialization and move (`DEF-375`)

`v = *alt(p, q);` inside a generic `overwrite<T>` (with `T = String`) is accepted, aborts with an
ASan double free, and the emitted C calls `cl_alt__String(p, q)` twice
(`build/batchM-plan/out/m375_derefimpure.c:702-703`): the read binds the value from the first
call's pointee, and the emptying statement calls `alt` again (which advances its counter and
selects the other pointee), so one pointee is bound to `v` and later released twice. The
store-only gate is `owns_drop(ty)` in `prepare_store` (`own/statements.rs:490`), true of the
**declaration-time** type; a generic `T` is not `owns_drop`. The same defect exists outside the
store: `return *alt(p, q)` (`m375_ret`), `Box { s: *alt(p, q) }` (`m375_field`), the array
element `[*alt(p, q)]` (`b375_arr`), and the conditional arm `flag ? *alt(p, q) : x`
(`b375_cond`) all abort with ASan double-free. The return path (`own/statements.rs` `check_return`,
the `lower_value` at :1446), the struct-literal field path (`own/lower.rs:307`), the array-literal
element, and the conditional arm have no materialization at all. The take-argument path
(`own/lower.rs:4866-4888`) already materializes under `owns_drop || maybe_owns || unknown`; that
predicate is the reference. Frozen: Ch 04 §3 (`spec/04-expressions.md:53`): "Evaluation is
strictly left-to-right and deterministic"; Ch 04 §7 makes `*p` a place read; Ch 17 §5
(`spec/17-codegen.md:182-192`): a move lowers to a copy plus a zero of the source, and the zero is
the drop flag, so a release can never run twice.

Fix: one shared "materialize an owning deref source" helper with the take-path predicate
(`owns_drop || maybe_owns || unknown`), used by the **store** (`prepare_store`), the **return**
(`check_return`), the **aggregate/struct-literal field**, the **array-literal element**, and the
**conditional arm**. A field/aggregate source must be **moved** (the pointee zeroed), not merely
pinned: the current field C either calls `alt` twice (impure pointer) or once without emptying
(place pointer), so the caller and the field can both own the pointee. Under a guard the fill/materialization is deferred to the evaluated path.
Maybe-owning values materialize; a concrete duplicable pointee is the control (`m375_dup`,
`T = i32`: one call, no emptiness, run 0). The discarded read (`b375_discard`) and the non-taking
argument (`b375_arg`) stay clean (no move, no empty). A generic duplicable instance may emit one
extra temporary versus its concrete twin; that emission delta is acceptable and named. Expected:
`alt` called once in all five consuming shapes; the store probe runs 12, the return/field
read-backs run 12, the array/conditional probes run 12; the controls run 0; ASan/LSan-clean.

### 2. The guarded take's consumed source and pointer fill (`DEF-376`)

`g && (eat(take (unsafe *q)) == 1)` is accepted, aborts with an ASan double free, and the caller
keeps owning the pointee the callee freed: the `&&` right operand defers move-outs
(`own/lower.rs:228-236`), but the deferred channel carries **places** only, so the non-place deref
source keeps its inline call and no emptying statement is emitted on the taken path. The
take-deref materialization at `own/lower.rs:4866-4888` also runs at **statement entry** even
under `defer_move_outs`, so `g = false` still calls an impure pointer
(`preflight/m376_impure_false`: calls = 1, run 10; expected calls = 0, run 0). Frozen: Ch 04 §3
(`:63`, the `&&`/`||` row: "Left operand first; the right operand only when the result is not yet
fixed"); Ch 04 §5 (`:108`: "only the chosen branch evaluates"); Ch 03 §3/O6 (a value moved on any
path counts as moved at a join; the zeroed source is the drop flag); Ch 17 §5 (one release per
owner).

Fix: gate the eager take-deref materialization on `defer_move_outs == 0`, and carry the pointer
fill on the deferred record: add `value_move_outs: Vec<Value>` to `Materialize` (`ir.rs:298`)
while `move_outs` stays `Vec<Place>` for places; the pointer-temp fill is `Materialize.value`.
The existing pointee channel is `take_pointee_move_outs -> Vec<Value>` (`own/call.rs:768`) emitted
as `MoveOutValue` (`ir.rs:377`); emit `MoveOutValue` in both guard forms in `codegen/guards.rs`,
inside the guard's evaluated path alongside the call. The exhaustive-match sites to audit are
`ir.rs` (`Materialize`), `codegen/guards.rs`, `codegen/function.rs`, `codegen/cleanup.rs`,
`own/generics.rs`, `own/instance_check.rs`, and `own/loops.rs`. The eager positions keep their
current shape; the guard's false path runs nothing, does not call the impure pointer, and leaves
the owner intact. Expected: accepted; `g = true` run 0, `g = false` run 0 (calls = 0);
ASan/LSan-clean for both selections. No new rejection.

## Integration Points

- `compiler/src/own/statements.rs:440` (`prepare_store`, the gate :490), `check_return` (:1358,
  the `lower_value` :1446); `compiler/src/own/lower.rs:307` (the struct-literal field), the
  array-literal element and conditional-arm paths, `:4866-4888` (the take predicate), `:3425`
  (`materialize_operand`), the pointer temporary.
- `compiler/src/own/lower.rs:228-236` (the `&&`/`||` deferred right operand), `:141-160` (the
  deferred move-out emission); `compiler/src/ir.rs:298` (`Materialize` with `move_outs` and the
  new `value_move_outs`), `:377` (`MoveOutValue`); `own/call.rs:768`
  (`take_pointee_move_outs`);
  `codegen/guards.rs`, `codegen/function.rs`, `codegen/cleanup.rs`; `own/generics.rs`,
  `own/instance_check.rs`, `own/loops.rs` (the exhaustive-match sites).
- Controls: `probes/m376_guardedtake.clo`'s two selections; `preflight/m376_impure_false.clo`,
  `m376_impure_true.clo`; `preflight/m375_ret.clo`, `m375_field.clo`, `b375_arr.clo`,
  `b375_cond.clo`, `b375_discard.clo`, `b375_arg.clo`, `m375_dup.clo`; the batch-L
  `DEF-329`/`DEF-376` deref shapes; the discarded-read and duplicable-pointee controls.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| An impure pointer expression in a deref read that is moved | Pin the pointer once; read and empty the same pointee | AR-20 |
| A discarded deref read | No emptiness; the current shape (control) | AR-20 |
| A duplicable pointee | A copy; no emptiness (control) | AR-20 |
| A guarded take of a deref source | Emit the emptying inside the evaluated guard path; the pointer fill is on the deferred record | AR-21 |
| The guard's false selection | Nothing runs, the impure pointer is not called, and the owner stays intact | AR-21 |
| A return, array element, conditional arm, or struct-literal field of an owning deref source | The shared materializer pins the pointer once **and moves the pointee (zeroes it)**; the pointer pin alone is the old defect | AR-20 |
| A guarded take of a place source | The existing place move-out channel (unchanged) | AR-21 |

## Testing Requirements

- **Spec cases** (`conditional_projection_residue_spec_test.rs`):
  - `should_evaluate_an_impure_deref_operand_once` (`DEF-375`): the store/overwrite, return,
    struct-literal-field, array-literal-element, and conditional-arm shapes each run 12 (their
    read-backs), `alt` is called once in all five, ASan/LSan-clean; the discarded and non-taking
    argument controls and the concrete duplicable control (a named generic duplicable emission
    delta is allowed).
  - `should_carry_a_guarded_deref_take_source` (`DEF-376`): `g = true` runs 0 and `g = false`
    runs 0 with calls = 0 (the impure pointer is not called on the false path), ASan/LSan-clean.
- **Impl pins** (`conditional_projection_residue_impl_test.rs`): the single `cl_alt` call in the
  emitted C, the read/empty pointer identity, the guarded emptying statement's position, the
  deferred pointer fill, the false-path absence, and a re-emission determinism pin.
- **Verification**: `make check`; the focused suite plain and with `CMD="RDC_ASAN=1 cargo test
  --test conditional_projection_residue_spec_test --test
  conditional_projection_residue_impl_test"`; the host `-fsanitize=address` runs of all five
  `DEF-375` shapes and both `DEF-376` selections.
- **Risks/lenses**: all four (AR-27). The memory lens owns the double-free and the guard paths;
  the emission lens checks the pointer identity, the single call, and the base/head differential
  over `m375_*`/`m376_*`; the semantics lens owns the `&&` evaluation rule (AR-37).
