# Conditional Materialization: Critical/Major DEF Closeout

> **Document**: 03-C-conditional-materialization.md
> **Parent**: [Index](00-index.md)

## Overview

Batch C closes the conditional read-base, nested-literal, and maybe-target
rows. The defects are all cases where a value is materialized before the
conditional or guard decides which storage is live. The fixes extend the
existing per-arm store and deferred-take helpers; they do not add a new
materialization layer.

## Architecture

### Current Architecture

- `codegen/guards.rs` owns guarded evaluation, embedded guards, and the
  read-base helpers.
- `codegen/function.rs` owns the function-body struct-literal and conditional
  struct stores (`struct_literal_needs_stores`,
  `conditional_struct_needs_stores`, `emit_struct_value_store_arm`), fixed-array
  conditional assignment, and call materialization.
- `codegen/program.rs` owns module-initializer literals and projection roots.
- `own/instance_check.rs` owns the per-instance move and conditional duties; a
  destination-aware duty must be recorded at the definition because a concrete
  arm is currently dropped before the audit.
- `own/statements.rs::prepare_binding_effect`/`apply_binding_effect` is where
  the destination type is known; `own/place.rs::mark_moved_if_owning` records
  the duty, and `ir/mono.rs::RequiresNotLend` carries its payload.
- `own/expr.rs`/`place.rs` own the definite-move and raw-deref judgments.

### Proposed Changes

| DEF | Change | Existing helper |
|---|---|---|
| 470 | Revalidate, then close as stale with the current `E10020` | Phase 0 |
| 471 | Add the `Deref` arm to `resolved_value_ty` so the division guard sees the pointee type | `codegen/expr.rs::resolved_value_ty` |
| 472 | Record the concrete lend arm and the generic destination, then judge the pair per instance; report the concrete twin once. Thread the destination type from `prepare_binding_effect` through `BindingEffect::MoveIfOwning`/`mark_moved_if_owning` into the `RequiresNotLend` payload and judge it in `check_not_lend_requirements` | `own/statements.rs`, `own/place.rs`, `ir/mono.rs`, `own/instance_check.rs` |
| 473 | Record the concrete local move on the maybe-target path; report `E10020` at the later use; keep the `T = str` view control accepted. The same destination-carrying duty distinguishes the owning instance | `own/statements.rs`, `own/place.rs`, `ir/mono.rs`, `own/instance_check.rs` |
| 475 | Bind the hidden destination or distribute the projection into the arms | guarded read-base materialization |
| 476 | Emit the per-arm struct store on the evaluated path for a duplicable conditional | `guards.rs` materialize arms |
| 477 | Recurse into conditional arms in `conditional_struct_needs_stores` and `emit_struct_value_store_arm` in `codegen/function.rs` | struct-literal store |
| 478 | Add the duplicable struct conditional to the deferred take predicate | guarded take helper |
| 479 | Hoist nested literal fields in source order, including the scalar nested literal; pin a struct declaration order that differs from the source order, so the scalar case is red before the fix | guarded literal emission |
| 480 | Bind the fresh root, move the selected member, release the remaining fields | fresh-root projection |
| 481 | Bind and release the chosen call arm's root | call-arm read base |

The common rule is: evaluate the condition first, then evaluate and materialize
only the selected arm, and release the selected root exactly once. A hidden
destination must be a real temporary, not a re-read of a moved source.

## Implementation Details

### New Types/Interfaces

No new public type. The existing `ValueKind`/`Materialize` helpers gain the
missing arm handling.

### New Functions/Methods

- `resolved_value_ty` gains `ValueKind::Unary { op: UnaryOp::Deref }` resolution
  through the pointee.
- `conditional_struct_needs_stores`/`emit_struct_value_store_arm` in
  `codegen/function.rs` recurse into nested conditionals.
- A destination-aware maybe-target duty records the concrete arm place/type and
  the generic destination type. The destination is threaded from
  `prepare_binding_effect` (`own/statements.rs`) through the move effect and
  `mark_moved_if_owning` (`own/place.rs`) into the `RequiresNotLend` payload
  (`ir/mono.rs`); `check_not_lend_requirements` (`own/instance_check.rs`)
  judges the substituted pair.
- A fresh-root helper binds the projection base once and releases the unselected
  fields after the member move.
- A call-arm helper binds the selected call result and registers its release on
  the evaluated path.


### Integration Points

The instance audit and the emitter must agree on the selected arm and the moved
source. Batch C's tests use the existing `emit_twice`/strict-C helpers and ASAN
for the release rows.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| Raw-deref division | Emit `E11210` guard through the resolved pointee | AR-5 |
| Concrete lend in a maybe-target conditional | One concrete-twin rejection (`E10034`); the `T = str` view control is accepted | AR-5 |
| Definite local moved on a maybe-target path | `E10020` at the later read; the `T = str` view control is accepted | AR-5 |
| Owned mixed read base | Valid C with a bound destination | AR-5 |
| Duplicable conditional read base | Per-arm store on the evaluated path | AR-5 |
| Nested conditional arms | Recurse; no eager arm evaluation | AR-5 |
| Deferred take | Evaluate only the selected arm | AR-5 |
| Fresh/call root | Bind and release once | AR-5 |
| Scalar nested literal order | Source order, not declaration order; pin a trace | AR-5 |

## Testing Requirements

- One specification case per DEF; the concrete twin provides the expected code.
- `DEF-472`/`473` add the `T = str` safe control so a destination-blind fix
  fails; the owning instance reports one concrete-twin diagnostic.
- ASAN/LSan for 472's safe control, 473's safe control, 475, 480, 481.
- Strict C for 475, 476, 477, 480.
- Evaluation-order probes for 478 and 479 (a skipped arm must not run; the
  scalar nested-literal source order differs from the declaration order).
- Determinism pins for accepted conditional emission.

