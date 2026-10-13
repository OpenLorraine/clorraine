# Update Order Completion: Conditional, Projection, and Update Residue

> **Document**: 03-06-update-order.md
> **Parent**: [Index](00-index.md)

## Overview

Four rows where an update shares an expression with another access, and the emitted C leaves the
order unspecified or undefined: the assignment target's location and index parts, a literal field
or element, the dot receiver and a loop-condition operand, and an earlier owning argument. The
frozen rule is spec/04 §3: evaluation is strictly left-to-right; the batch-H pin (`contains_update`
+ `pin_update_value`) already implements the rule for written call arguments and non-short-circuit
binary operands.

## Design

### 1. Assignment targets (`DEF-273`, AR-9)

`arr[i] = i++;`, `i = i++;`, and `i += i++;` leave the target, the update, and the store
unsequenced — the pinned container compilers reject the shapes under the suite's own strict flags
(`-Werror=sequence-point` / `-Werror,-Wunsequenced`, selected by `-Wall -Wextra -Werror`; AR-24):
the target's location read (`arr[i]`'s index; the
compound assignment's current-value read), the right-hand side's update, and the store's write,
while the language orders the target location before the right-hand side and completes the update
before the store.

Extend the pin to the assignment's parts: an index or location part that reads a place the
right-hand side updates pins at its own (target-first) position; the right-hand side itself, when
it contains an update, pins into a statement temporary at its own source position (so the update
completes before the store); the compound assignment's current-value read pins before the
right-hand side when the right-hand side updates the same place. The store itself keeps its shape;
a plain `i = v;` re-emits identically.

### 2. Literal fields and elements (`DEF-274`, AR-10)

`let p: P = P { a: i++, b: i };` and `let xs: i32[2] = [i++, i];` leave the C initializer order
unspecified while the language evaluates fields and elements in source order.

Pin an update-containing literal field or element at its source position (the existing pin, applied
to the literal's evaluated operands in order); the literal's own shape is unchanged, and a literal
without an update re-emits identically.

### 3. The dot receiver and loop conditions (`DEF-277`, AR-9)

`(i++).get(i)` and `while (i + id(i++) < 3)` leave the receiver and the loop-condition operand
unsequenced (the pinned container compilers reject the shapes under the suite's own strict flags —
AR-24).
The pin covers the written call arguments and non-short-circuit binaries; the dot receiver is
argument position zero (pushed before the argument loop) and a loop condition is excluded by the
`in_loop_condition` guard.

- Dot receiver: pin the receiver when the call's receiver or arguments carry an update, aligning
  with the call's argument modes as the argument loop does.
- Loop condition: a loop condition is re-evaluated each iteration, so its pin must live inside the
  condition, not before the loop. A plain statement pin is exactly what the `in_loop_condition`
  guard excludes; route the update pin through the condition's per-iteration `Materialize` fill
  instead (the guard machinery the loop lowering already emits before every test), so the update
  materializes inside each iteration.

### 4. The owning argument's order (`DEF-278`, AR-9)

`f(g(), i++)` builds and evaluates `g()` after the update (the host compilers order right-to-left),
while the language orders the arguments left-to-right. The pin skips a non-duplicable,
non-update argument because a plain copy would carry ownership.

Bind the owning argument's result at its own position through the existing ownership-safe binding
(a scope owner or a move carrier), then the call reads the bound value; the argument's ownership
transfers exactly once and the left-to-right order holds. A duplicable argument keeps the plain
temporary.

## Integration Points

- `own/lower.rs`: `pin_update_value` (:1234), `contains_update` (:3506), the assignment lowering,
  the literal lowering, and `materialize_owned_argument` (:3223).
- `own/call.rs`: the call receiver/argument loop and its pin positions.
- `own/expr.rs`: the update triggers on the receiver and the assignment lowering.
- `own/loops.rs`: the loop-condition lowering and the per-iteration `Materialize` fill.
- `codegen/control.rs`: the loop header's per-iteration guard emission.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| An update-free assignment/literal/call | Re-emits identically (the pin is a no-op) | AR-9 |
| An update-containing assignment right-hand side | Pins into a statement temporary at its source position | AR-9 |
| A guarded `&&`/`\|\|` operand or a `?:` arm | Keeps the existing deferred spelling | AR-9 |
| A loop condition | The pin lives in the per-iteration `Materialize` fill | AR-9, AR-24 |
| An owning argument | A scope owner or move carrier; one transfer | AR-9 |

## Testing Requirements

- Spec: the three assignment-target spellings, the literal field and element orders, the dot
  receiver and loop-condition cases (single-shot and loop), and the owning argument's order; the
  run assertions carry the spec/04 §3 values and the emission order is pinned by the impl pins
  (the pinned container rejects the sequence shapes under the strict flags and orders initializer
  lists left-to-right — AR-24); the batch-H update pins stay green.
- Impl: the pin positions and statements, the owning-argument binding shape, and the re-emission
  determinism pins for the untouched spellings.
