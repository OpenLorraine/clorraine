# Raw Store Integrity: Critical/Major DEF Closeout

> **Document**: 03-B-raw-store.md
> **Parent**: [Index](00-index.md)

## Overview

Batch B closes eleven pre-existing raw/conditional store and alias defects. Their
common shape is that a move, take, or alias is decided from the written spelling
rather than from the address actually used at run time. The fixes stay in the
existing ownership and codegen helpers.

## Architecture

### Current Architecture

- `codegen/function.rs::emit_fixed_array_conditional_assign` emits the
  destination save, arm stores, and release without a take capture.
- `codegen/function.rs::struct_literal_needs_stores`, the struct-literal
  emitters, and `conditional_struct_needs_stores` decide whether a guarded
  literal needs per-field stores; the guarded call/receiver path is
  `codegen/guards.rs::struct_literal_needs_guard_stores` and the plain literal
  is `codegen/expr.rs::struct_literal`.
- `codegen/function.rs` and `codegen/guards.rs` use `__cl_rhs`, `__cl_save`, and
  `unique_local` temporaries for conditional ownership.
- `codegen.rs` carries one `Option<TakeCapture>`; nested materializations
  replace it and lose the outer flag.
- `own/place.rs` owns the alias test, move-out, and raw-deref helpers.
- `own/expr.rs` owns post-store source invalidation and the conditional store
  source walk.

### Proposed Changes

| DEF | Change | Existing helper |
|---|---|---|
| 454 | Recurse the raw-deref owning field into the guarded struct-literal argument/receiver store; clear the pointee after the move | `struct_literal_needs_guard_stores`, guarded struct store |
| 455 | Install/clear the take capture around `emit_fixed_array_conditional_assign`'s destination and arms | `emit_fixed_array_conditional_assign` |
| 456 | Test containment in **both** directions over the source and **destination** integer addresses, then clear only the taken subrange of the save copy | conditional-save alias test |
| 457 | Replace the single take capture with a stack so a nested materialization keeps the outer destination flagged | nested materialization |
| 458 | Apply the runtime take-ran capture to place move-outs; a skipped `\|\|` right must not zero the save | `arm_consumes_target` / place move-out |
| 460 | On the raw self-alias path, restore the save into the destination and zero the save (or suppress that release) | `own/place.rs` raw-deref helpers, `codegen/function.rs` |
| 461 | Use the adjusted array address, pin it before the call, and apply the guard in `function.rs` | `is_array_param_place` / `array_source_address` |
| 462 | Pin the intermediate raw-field lvalue before evaluating a side-effecting RHS | `own/expr.rs` pin site, guarded store core |
| 464 | Mark the conditional store's source moved and report the later read; keep the destination read accepted | `own/expr.rs` post-store walk |
| 465 | Move the `__cl_rhs` evaluation outside the `needs_drop` gate for a duplicable fixed-array self-read, including the fill twin | fixed-array store core |
| 466 | Add a runtime target/source address test before the element owning move; skip the source release on the self-alias path | element store core |

The containment test (456) is one structural change, and its two directions are
**source vs destination**, not source vs the save copy. The source may contain
the destination at a nonzero offset (a super-object take into a member
destination), or the destination may contain the source (a first-member take
into a whole-struct destination). The save is a copy of the destination, so its
local address never participates in the containment test; the test compares the
source address with the destination address. Implement both directions with an
integer address type (`unsigned long` or a compiler-internal address type; the
freestanding unit has no `<stdint.h>`), compute the taken subrange in the
destination's coordinate space, and clear only that subrange of the save copy
before the release. Do not add a general alias-analysis pass.

## Implementation Details

### New Types/Interfaces

- A private `SavedRange` in `codegen/function.rs` carrying the save copy's base
  and byte length, the destination address, and the taken source's
  offset/length in the destination's coordinate space.
- The `TakeCapture` entry gains the save range (or a parallel `CurrentSavedRange`
  is saved/restored through nesting); the `(flag, target)` pair alone cannot
  select a subrange at the post-branch site, so the plan no longer claims that
  shape is unchanged.
- A `Vec<TakeCapture>` stack in `Codegen` (replacing the single `Option`).
- No public API change. The ownership IR keeps its current shapes.

### New Functions/Methods

- `emit_conditional_save`/`alias_within_saved_range` in `codegen/function.rs`:
  tests both containment directions over the source and destination integer
  addresses, computes the taken subrange in the destination's coordinate space,
  maps it onto the save copy, and decides the clear granularity.
- `emit_fixed_array_conditional_assign`: adds the take capture parameter/flag.
- `emit_struct_literal_store`: recurses a `ValueKind::Unary { op: Deref }` field
  and empties the pointee after the move.
- `own/expr.rs` post-store invalidation: records the conditional source and
  reports `E10020` on a later read; the destination read stays accepted.
- `emit_take_alias` and `deref_move_zero_expr`: walk the capture stack and set
  every matching destination.
- The DEF-460 alias path restores the save into the destination and zeros the
  save before the normal release.
- The DEF-465 fixed-array path evaluates a self-reading RHS into `__cl_rhs`
  before the clear, whether or not the element type owns.

### Integration Points

The ownership IR's move annotations must match the codegen decisions: when the
callee or the store takes a source, the same source is marked moved for the
later-read check. The tests use existing `RDC_ASAN=1` and strict-C helpers.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| Guarded raw-deref owning field | Move, do not copy; ASAN double-free must disappear | AR-5 |
| Conditional fixed-array destination take | Capture the take before the arm stores | AR-5 |
| Subobject containment | Both directions; clear the taken subrange only; preserve siblings | AR-5 |
| Nested temporary materialization | Preserve every live outer capture via the stack | AR-5 |
| Skipped `\|\|` right | The runtime take-ran flag keeps the save alive | AR-5 |
| Raw self-alias | Restore the save into the destination and zero the save | AR-5 |
| Post-store source read | `E10020` once at the later read; the destination read is accepted | AR-5 |
| Duplicable self-read | RHS-before-clear ordering, including the fill twin | AR-5 |

## Testing Requirements

- One specification case per DEF, each traced in `07-testing-strategy.md`;
  `DEF-458` is ST-35.
- Unsafe reproductions reject or run clean; accepted controls run exit 0.
- ASAN/LSan is required for 454, 455, 456, 457, 458, 460, 461, 466.
- `DEF-454` includes a skipped-guard control that reads the pointee and is
  LSan-clean; `DEF-461` includes the `&items` address-of spelling.
- `DEF-464` includes the accepted destination read (`a[0]`) and the rejected
  later source read (`a[1]`).
- `DEF-456` includes the super-object→member case (a struct with a leading
  scalar so the member offset is nonzero) and the first-member→whole case (a
  whole-struct destination with an owning sibling); pin ASAN/LSan and the exit
  value. `DEF-460` reads the destination after the store under ASAN.
- Strict-C builds are required for every emitted shape.
- Determinism pins for accepted outputs that change.
