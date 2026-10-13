# Arm Roots and Per-Instance Duty: Conditional, Projection, and Update Residue

> **Document**: 03-01-arm-roots-and-duty.md
> **Parent**: [Index](00-index.md)

## Overview

The batch-H arm binders handle the concrete spellings. The phase's rows: the per-instance duty
for a generic arm projection, the fresh-root sibling binding in a conditional of projections, the
guarded `take` carrier, and the struct-literal field's projection root in guarded positions. The
struct-literal arm root (`DEF-259`) is incidentally closed at the planning baseline (AR-23) and
stays a green control. Every fix extends the existing per-arm machinery; none adds a pass
(AR-17).

## Design

### 1. Per-instance arm duty (`DEF-257`, AR-3)

The concrete-phase fix routes a consumed arm projection through `bind_ownerless_projection_root`
and marks the projected source; the checker deliberately skips an arm whose type still mentions a
type parameter, so the monomorphized instance copies without a move-out and double-frees.

Route the judgment through the existing per-instance duty: an arm projection whose *instance*
type owns storage (per the monomorphization's substituted type) records the projected source and
moves the member out exactly as the concrete spelling does. The duty is evaluated per instance —
no new machinery, and the concrete path keeps its pinned shapes.

### 2. Fresh-root sibling binding (`DEF-258`, AR-11-adjacent)

`let s: String = c ? a.v : mkBox().v;` projects both arms, so the conditional is not routed
through the arm-fill binder (that binder serves conditionals whose *value* is the aggregate). The
fresh arm's root (`mkBox()`) never binds, and its un-moved sibling `w` leaks.

Extend the conditional-of-projections path: when an arm's projection roots at fresh ownerless
storage, bind that root through the existing arm-fill machinery (`hoist_ownerless_conditional_arms`
/ `own_ownerless_conditional_arms`), so the root's un-moved fields release at the statement end
and the *moved* member transfers to the consumer. A place-rooted arm stays untouched.

### 3. Struct-literal arm root (`DEF-259`, AR-23) — incidentally closed

`(c ? a : Box { v: …, w: … }).v` left the literal root unbound in the pre-fix behavior, so `w`
leaked. Batch H's binder work incidentally closed the probed spellings: the reproduction is green
at the planning baseline, so no fix is required and ST-3 pins the shape as a green control. The
earlier fix direction (teach the guarded projection binder the struct-literal root) is withdrawn;
if a spelling still leaks, the executor files it as a new row with base == head evidence (the
batch convention).

### 4. Guarded `take` carrier (`DEF-260`, AR-4)

A guarded position (`if (g && consume((c ? a : mkBox()).v) > 0)`) bypasses the eager carrier:
`take_move_out_places` finds no caller place for a conditional projection, so the callee's take
release is followed by the caller's own release — a double-free.

Route the guarded `take` of a conditional projection through the eager per-arm carrier design:
each fresh arm fills the shared carrier on the evaluated path, the call's post-call move-out
empties the carrier, and the caller's release of the root frees only its un-moved fields. The
`&&`/`||`/`?:` short-circuit semantics stay: the carrier fills only where the guard runs.

### 5. Struct-literal field projection root in guarded positions (`DEF-264`, AR-4-adjacent)

`H { call: addOne, s: mkBox().v }.call(5)` in an `if` leaks the `mkBox()` root's sibling `w`
(33 B over three loop iterations after the batch-H fix): the struct-literal binder never binds a
*field's* projection root, so only the projected member is owned.

The binder walks each field value; a field whose value is a projection of an ownerless root binds
that root before the literal is built, in every position (plain, guarded, loop condition). (The
array-literal element variant closes with the P4 loop-condition walk — `03-04` §1.)

## Integration Points

- `own/lower.rs`: `bind_ownerless_projection_root`, `hoist_ownerless_conditional_arms`,
  `own_ownerless_conditional_arms`, `materialize_owned_argument`, `defer_discarded_arm`.
- `own/expr.rs`: the projected-source collector (the per-instance mark).
- `own/call_check.rs`: the take-argument judgment.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| An arm's instance type owns nothing | The binder returns the value unchanged (no temporary) | AR-3 |
| A guarded arm is not taken | The carrier stays empty; the post-call move-out is a no-op | AR-4 |
| A place-rooted arm | Stays a read; the consumer never empties the place | AR-4 |
| A struct-literal arm root (`DEF-259`) | Green at the baseline; ST-3 keeps the shape as the incidental-closure control | AR-23 |

## Testing Requirements

- Spec: the generic-spelling store/return cases (instance `String`), the conditional-of-projections
  fresh arm (store and take), the guarded take under ASan, and the struct-literal field root in an
  `if` and a loop condition; the struct-literal arm root is the green incidental control; the
  concrete twins keep their pinned shapes.
- Impl: the emitted carrier/binding shapes, the release counts, and the re-emission determinism
  pins.
