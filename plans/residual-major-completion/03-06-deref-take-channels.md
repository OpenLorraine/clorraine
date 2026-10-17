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

### 1. The deref operand's single materialization (`DEF-375`)

`v = *alt(p, q);` inside a generic `overwrite<T>` (with `T = String`) is accepted, aborts with an
ASan double free, and the emitted C calls `cl_alt__String(p, q)` twice
(`build/batchM-plan/out/m375_derefimpure.c:702-703`): the read binds the value from the first
call's pointee, and the emptying statement calls `alt` again (which advances its counter and
selects the other pointee), so one pointee is bound to `v` and later released twice. The store
lowering materializes the deref operand only when `owns_drop(ty)` (`own/statements.rs:490`) is
true of the **declaration-time** type; a generic `T` is not `owns_drop`, while the codegen empties
under the substituted type. Frozen: Ch 04 §3 (`spec/04-expressions.md:53`): "Evaluation is
strictly left-to-right and deterministic"; Ch 04 §7 makes `*p` a place read; Ch 17 §5
(`spec/17-codegen.md:182-192`): a move lowers to a copy plus a zero of the source, and the zero is
the drop flag, so a release can never run twice.

Fix: materialize the deref operand whenever the lowered value is a raw-deref read that will be
moved or emptied — the gate becomes the **substituted** type/ownership judgment, not the
declaration-time `owns_drop(ty)` alone. The read and the emptying statement share the one
materialized pointer temporary, so the side-effecting expression runs once and the emptied
pointee is the one that was read. The discarded read and the duplicable pointee keep their
current shapes (no emptiness, no materialization needed). Expected: `alt` called once; `r` owns
the read pointee; run exit 12 (`len("aa") + 1 * 10`); ASan/LSan-clean.

### 2. The guarded take's consumed source (`DEF-376`)

`g && (eat(take (unsafe *q)) == 1)` is accepted, aborts with an ASan double free, and the caller
keeps owning the pointee the callee freed: the `&&` right operand defers move-outs
(`own/lower.rs:228-236`), but the deferred channel carries **places** only, so the non-place deref
source keeps its inline call and no emptying statement is emitted on the taken path. Frozen: Ch 04
§3 (`:53`, the `&&`/`||` row: "Left operand first; the right operand only when the result is not
yet fixed"); Ch 04 §5 (`:108`: "only the chosen branch evaluates"); Ch 03 §3/O6 (a value moved on
any path counts as moved at a join; the zeroed source is the drop flag); Ch 17 §5 (one release
per owner).

Fix: extend the deferred move-out record to carry the consumed non-place source (the deref place's
pointer temporary and its emptying statement) and emit it inside the guard's evaluated path,
alongside the call. The eager positions keep their current shape; the guard's false path runs
nothing and leaves the owner intact. Expected: accepted; `g = true` run 0, `g = false` run 1;
ASan/LSan-clean for both selections. No new rejection.

## Integration Points

- `compiler/src/own/statements.rs:490` (the `owns_drop(ty)` gate and the deref store lowering),
  `compiler/src/own/lower.rs:3425` (`materialize_operand`), the pointer temporary.- `compiler/src/own/lower.rs:228-236` (the `&&`/`||` deferred right operand),
  `compiler/src/own/lower.rs:141-160` (the deferred move-out emission), the statement/guard
  carrier (`own/statements.rs`, `own/lower.rs`).
- Controls: `probes/m376_guardedtake.clo`'s two selections; the batch-L `DEF-329`/`DEF-376`
  deref shapes; the discarded-read and duplicable-pointee controls.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| An impure pointer expression in a deref read that is moved | Pin the pointer once; read and empty the same pointee | AR-20 |
| A discarded deref read | No emptiness; the current shape (control) | AR-20 |
| A duplicable pointee | A copy; no emptiness (control) | AR-20 |
| A guarded take of a deref source | Emit the emptying inside the evaluated guard path | AR-21 |
| The guard's false selection | Nothing runs; the owner stays intact | AR-21 |
| A guarded take of a place source | The existing place move-out channel (unchanged) | AR-21 |

## Testing Requirements

- **Spec cases** (`conditional_projection_residue_spec_test.rs`):
  - `should_evaluate_an_impure_deref_operand_once` (`DEF-375`): the overwrite shape builds and
    runs 12, `alt` called once, ASan/LSan-clean; the discarded and duplicable controls.
  - `should_carry_a_guarded_deref_take_source` (`DEF-376`): both guard selections build and run
    0/1, ASan/LSan-clean; the taken and skipped paths count their calls.
- **Impl pins** (`conditional_projection_residue_impl_test.rs`): the single `cl_alt` call in the
  emitted C, the read/empty pointer identity, the guarded emptying statement's position, the
  false-path absence, and a re-emission determinism pin.
- **Verification**: `make check`; the focused suite plain and under `RDC_ASAN=1`; the host
  `-fsanitize=address` runs of both shapes (both selections for `DEF-376`).
- **Risks/lenses**: all four (AR-27). The memory lens owns the double-free and the guard paths;
  the emission lens checks the pointer identity and the single call; the semantics lens owns the
  `&&` evaluation rule.
