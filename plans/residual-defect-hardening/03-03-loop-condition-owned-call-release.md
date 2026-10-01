# Loop-Condition Owned-Call Release: Residual Defect Hardening

> **Document**: 03-03-loop-condition-owned-call-release.md
> **Parent**: [Index](00-index.md)

## Overview

This component fixes DEF-48: a loop condition that holds an owned call result but needs no runtime
guard prints the call directly in the header, and the caller never releases the value. The fix
materializes every owned call result in a loop condition into a destination temporary and routes
the loop through the existing guarded path, which already drops the previous value before a
re-store and releases the last value after the loop. Per AR #5 and AR #10, only owned call results
are materialized; non-owning calls keep the inline spelling.

## Architecture

### Current Architecture

- Conditions are lowered at `compiler/src/own/loops.rs:48-49` (`while`), `:133-134`
  (`do while`), and `:228-233` (`for`).
- `materialize_guard_operand` (`compiler/src/own/lower.rs:602-638`) turns a side-effecting guarded
  operand into a `ValueKind::Materialize` with a fresh temp, marks it a destination temp, and
  registers it in the statement's pending-release set. It is called from `Index`, division, and
  float-cast lowering, but never from a plain `Member` read, so `grow().count` leaves the call
  inline.
- `value_needs_guards` (`compiler/src/ir/guards.rs:19-84`) mirrors guard placement. `Materialize`
  currently recurses into its inner value and reports false for a release-only materialization.
- `codegen/control.rs` selects the guarded loop form from that predicate: `while (1)` with
  `emit_guards` and a break test (`:344-357`), the flag-first `for (;;)` for `do while`
  (`:362-389`), and `for (;; ; step)` (`:398-410`). The guard emitter drops the previous temp
  value before re-storing it (`codegen/guards.rs:538-593`), and the statement's pending-release
  drain releases the final value after the loop (`own.rs:965-989`).

### Proposed Changes

1. **Materialize owned call results in every loop condition.** A new helper in
   `compiler/src/own/loops.rs` walks the lowered condition and replaces each `ValueKind::Call` or
   `ValueKind::ClosureCall` whose type `owns_drop(ty)` accepts with
   `materialize_guard_operand(value)`; the substitution-aware phase widens this predicate to
   `owns_drop(ty) || maybe_owns(ty)` (task 4.2.5), which is what lands ST-18. It runs at the three
   condition lowering points, after
   `lower_value` and before the branch terminator is built. A call that is already the direct
   stored value of a `Materialize` is left alone: the existing guard path has already stored it
   once, and wrapping it again would emit an array assignment between two temporaries and
   register two releases. The walk still descends into a `Materialize` whose stored expression
   merely contains an owned call elsewhere.
2. **Report a materialized condition as needing the guarded path.** `value_needs_guards` returns
   `true` for `ValueKind::Materialize` instead of recursing, because the materialize store and its
   drop are statements that cannot live in a C loop header. This keeps the predicate exact: any
   materialized operand moves the test into the body. The predicate's contract becomes "the loop
   header cannot carry this value", not only "a runtime guard is needed"; its second consumer
   `for_step_is_structured` (`own/loops.rs:586-598`) picks up the same change, so a `for` step
   holding a materialize drops to the flat label-and-`goto` form.
3. **No codegen change.** With the predicate true and the condition carrying a `Materialize`, the
   existing guarded forms run: the previous value is dropped before each re-store, and the last
   value is released by the pending-release drain after the loop.

The walk follows the same value shapes `value_needs_guards` traverses so no call is missed: binary
operands, index base and index, cast value, unary operand, conditional parts, member base, call
receiver and arguments, intrinsic arguments, struct-literal fields, array-literal elements and
fill, template placeholders, nested materialize, closure captures, and closure-call parts. It
checks the ownership predicates on the call's type, so a call returning a view, a slice, a
duplicable value, or an unknown type is left inline. For a non-owning call, the header spelling is
unchanged (AR #10).

A condition whose call result mentions a generic type parameter lands in the substitution-aware
phase: that phase widens this walk's predicate with `maybe_owns` and removes the materializer's
in-helper skip (`own/lower.rs:619-621`), so the temporary is declared with the substituted type
per instance. The generic-loop specification case ST-18 lands in that phase's red/green set,
because the predicate widening and the skip removal must ship together.

### Integration Points

- `compiler/src/own/loops.rs` — the helper and its three call sites.
- `compiler/src/ir/guards.rs:69` — `Materialize` reports true.
- `compiler/src/codegen/control.rs` — unchanged; it consumes the predicate.
- `compiler/src/own/lower.rs:602-638` — the materializer; its param-typed skip is removed by the
  substitution-aware phase (`03-04`).
- `compiler/src/own/loops.rs:586-598` — `for_step_is_structured`, the predicate's second consumer;
  a step holding a materialize drops to the flat form.

## Code Examples

```clorraine
func grow(): Buffer { /* ... */ }
let i: i32 = 0;
while (grow().count > 0 && i < 3) { i = i + 1; }
```

Before: the emitted header calls `grow()` every test and leaks every buffer. After: the condition
carries a materialized temp; the emitted C is a `while (1)` loop whose body drops the previous
temp, breaks on the test, and releases the last temp after the loop.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Call returns a non-owning value | Not materialized; the header spelling is unchanged | AR #10 |
| Call appears under `&&`/`||`/`?:` | The materialize guard runs only on the path that evaluates the operand, preserving short-circuit behavior | AR #5 |
| Condition also has a runtime guard | Both are already materialized/guarded; the loop form is the same guarded form | AR #5 |
| Call result mentions a type parameter | The substitution-aware phase widens the walk's predicate with `maybe_owns` and removes the materializer's in-helper skip; ST-18 pins it | AR #5, #11 |
| Call already stored by a guard `Materialize` | Left alone (no double wrap); the existing temp's release path is unchanged | AR #5 |

## Testing Requirements

- Spec cases ST-8 – ST-9 (`07-testing-strategy.md`), run and ASan-probed (the leak is invisible to
  the exit code); the generic-loop case ST-18 runs with the substitution-aware phase (`03-04`).
- Impl tests for the guarded loop shape (a `Materialize` in a condition selects the flag-first
  form) for `while`, `do while`, and `for`; the guarded fixed-array condition
  (`while (i < 3 && mkArr()[i] != null)`) emits one temp and one release; and a `for` step that
  holds a materialize drops to the flat form (`for_step_is_structured`,
  `compiler/src/own/loops.rs:586-598`).
- A pre-fix ASan probe of the ST-8 program recorded in the execution plan (the red-phase
  evidence).
