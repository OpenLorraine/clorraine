# Loop Conditions, Update Order, and Store Lowering: Conditional, Projection, and Update Residue II

> **Document**: 03-04-loop-conditions-update-order.md
> **Parent**: [Index](00-index.md)

## Overview

Five rows in the loop-condition and store-lowering machinery: the mixed-arm loop receiver
(host-toolchain sequence-point warning), the loop-condition member-projection double-free, the
guarded update operand, the array fill re-evaluation, and the dropped bare raw-deref store. The
frozen rules are the strictly left-to-right evaluation of Ch 04 §3, the fill's single
evaluation (`spec/04-expressions.md`), and the dereferenced raw pointer as a valid store target
(spec/04 §7). The update pin and the per-iteration `Materialize` fill already exist; the rows
are trigger, routing, and lowering gaps (batch-I AR-9 practice).

## Design

### 1. The mixed-arm loop receiver (`DEF-294`, AR-10)

`while (i < 1 && (c ? mkH() : a).call(5) > 0)` emits a comma-sequenced receiver expression; the
pinned container (GCC 12.2) accepts it and runs exit 0, while the host GCC 15 rejects it with
`-Werror=sequence-point` (measured; the emission is byte-identical to the phase-4 tree where the
row was filed). The fix direction is the `DEF-277` pattern: pin the receiver position so the
emission is explicit.

Pin the dot/func-field receiver when its expression carries an arm's call or an update (settle
it at argument position zero), so the receiver's parts become statement-position fills. The
phase measures both toolchains, pins the emission shape, and records the outcome: if the pinned
emission changes, the row closes fixed; if only the host warning remains false-positive on a
standard-conforming spelling, the row closes as a control with the dual-toolchain evidence
(AR-10, the batch-I re-decision precedent).

### 2. The loop-condition member projection (`DEF-295`)

`while (i < k && i32(len(mkH().s)) > 0)` double-frees (probe: run abort 134, ASan
`attempting double-free`): the member projection and the root each release the same storage.

Bind the projection's root once and read the member from it: the fresh root materializes inside
the per-iteration `Materialize` fill (the loop-condition machinery), the member read projects
from the bound root, and exactly one release runs per iteration/exit. The store/argument twins
of the same projection keep their batch-I shapes.

### 3. The guarded update operand (`DEF-308`, residual)

`g && f(i++, i)` and `c ? f(i++, i) : 0` keep the deferred spelling and fail
`-Werror=sequence-point` (probe-confirmed, container and host); the plain loop-condition sibling
`while (f(i++, i) < 30)` is fixed and green (AR-3). The batch-I design's error-handling table
declared the deferred spelling as the boundary; the row makes it the work.

Pin the guarded operand's parts inside the guard path: a guarded `&&`/`||` operand or a `?:`
arm that contains an update materializes its updating parts through the deferred per-iteration
`Materialize` fill precedent (the loop-condition route), so the update completes at its source
position on the evaluated path only. The unguarded and plain loop-condition spellings keep
their pinned emission.

### 4. The array fill evaluation (`DEF-309`)

`[; i++]` runs the fill expression once per element (three iterations for a three-element array)
where spec/04 requires the fill to be evaluated once.

Pin the fill into a temporary before the element loop (the existing pin machinery), so the fill
evaluates once and every element receives the same value; the fill-free literal re-emits
identically.

### 5. The bare raw-deref store (`DEF-310`)

`unsafe { *q = 9; }` and `*q = i++;` compile and the store emits nothing (probe: exit 254, the
write is dropped); `p.field = v;` through a raw works and a bare deref read works. spec/04 §7
names a dereferenced raw pointer a valid target.

Lower the bare raw-deref store target: the deref target produces the addressed place (the
pointer value plus the store through it, the `StorePointer` shape the indexed path uses), the
update pin applies to the stored value when it carries an update, and the read path is
unchanged.

## Integration Points

- `own/lower.rs`: `pin_update_value` (:1457), `contains_update` (:4553), the array-literal
  lowering, `bind_ownerless_projection_root` (:3122) at the loop condition.
- `own/call.rs`: the call receiver/argument loop and its pin positions.
- `own/expr.rs`: the assignment/store lowering (`check_place_raw` :1976-adjacent) and the update
  triggers.
- `own/loops.rs`: the loop-condition lowering and the per-iteration `Materialize` fill.
- `codegen/guards.rs`/`codegen/control.rs`: the deferred fill and the loop header emission.
- `ir.rs`: `Statement::Assign`/`StorePointer` (:299) — the deref target's statement shape.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A receiver without a call/update part | Re-emits identically (the pin is a no-op) | AR-10 |
| A loop-condition projection of a place root | Stays a read; only a fresh root binds | — |
| A guarded operand without an update | Keeps the existing deferred spelling | — |
| A fill value with no side effect | Pins once (byte-identity not required; one evaluation) | — |
| A deref store whose value carries an update | The update pins at the value's position, then the store | — |
| A field write through a raw | Unchanged (the existing working path) | — |

## Testing Requirements

- Spec: the mixed-arm loop receiver (both selections; strict build on both toolchains and the
  run values — the container red is the emission pin where the host warning is not reproducible,
  AR-10), the loop-condition projection (ASan, single-shot and re-executed), the guarded update
  spellings (`&&`, `||`, `?:`; strict build and the run values; the plain loop-condition green
  control), the array fill (an observing counter: one evaluation), and the bare raw-deref store
  (read-back of the stored value, with the update twin). The batch-I update pins stay green.
- Impl: the receiver pin shape, the loop-condition root binding, the guarded fill positions, the
  fill pin, the deref store statement, and the re-emission determinism pins for the untouched
  spellings.
