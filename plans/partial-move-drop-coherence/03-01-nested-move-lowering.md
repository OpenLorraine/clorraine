# Owned Literal-Field Consumption: Partial-Move / Drop Coherence

> **Document**: 03-01-nested-move-lowering.md
> **Parent**: [Index](00-index.md)

## Overview

This component makes a value-consuming read of an owned place inside an aggregate
literal a move: the ownership stage records it as a move, and the lowering emits
the copy-plus-zero that realizes the move. It is the whole fix for DEF-11; no
assignment change is needed once the literal field is consumed. It realizes
`spec/17-codegen.md` P3 for a nested owned read.

## Architecture

### Current Architecture

`lower_value` renders a struct-literal field as a plain value
(`compiler/src/own/lower.rs:158-170`); a non-duplicable place field is copied
without zeroing, and the ownership stage's literal check only recurses
(`compiler/src/own/expr.rs:133-137`), so it never records the field as moved.
The assignment emitter then sees a RHS that reads the target and takes the
save/restore path (`compiler/src/codegen/function.rs:503-517`), dropping a copy
that aliases the moved-out handle.

A whole-place move already works: `emit_store` emits
`Statement::Move { target, source }` (`compiler/src/own/statements.rs:179-189`)
and its codegen copies then zeroes the source
(`compiler/src/codegen/function.rs:324-332`). The temporary mechanism
(`new_temp`, `ir::PlaceBase::Temp`, `materialize_value`) already exists
(`compiler/src/own.rs`, `compiler/src/own/lower.rs:524-534`).

### Proposed Changes

Two coordinated changes, both in the ownership stage:

1. **Ownership flow state:** when a struct literal's field is a non-duplicable
   place, mark it moved (`move_target`), so a second read is `E10020` and a
   deeper-than-top-level path is `E10026` (`spec/03-ownership.md:123-134`).
2. **Lowering:** consume that same field through a helper that emits a
   `Statement::Move` into a temporary and uses the temporary in the literal, so
   the source field is zeroed before the literal is built.

Because the literal value then references the temporary rather than the target,
`value_reads_base` is false and the existing drop-then-store fast path releases
the (already-empty) old target correctly. **No assignment-lowering change is
made.**

## Implementation Details

### Ownership mark

`ExprKind::StructLiteral` eval (`compiler/src/own/expr.rs:133-137`) extends its
per-field recursion: after evaluating a field, if the field is consumed — a
non-duplicable place whose type does not mention a type parameter — call
`move_target(place, span)`. `move_target`/`move_place`
(`compiler/src/own/place.rs:170-244`) already enforce the top-level-only rule
(`E10026`), the borrowed-parameter rule (`E10034`), and record the partial move
so a later read reports `E10020`.

The "non-duplicable and does not mention a type parameter" test is the **same
predicate** the lowering uses (below), so the checker and the lowering consume
exactly the same fields. It matches the sibling materializers' guard
(`compiler/src/own/lower.rs:501,554,587`).

### Lowering helper

```rust
/// Consumes a place as a value, moving an owned value so its source is zeroed.
///
/// A duplicable place, or a place whose type still mentions a generic
/// parameter, is returned unchanged (a copy). An owned place is materialized
/// into a temporary through `Statement::Move`, which copies the value and then
/// zeros the source, so a later drop of the source cannot free the value again.
fn consume_place(&mut self, place: Place, span: Span) -> Value
```

`consume_place` uses the shared predicate: when the place is duplicable or its
type mentions a type parameter it returns `Value::place(place)`; otherwise it
allocates a temporary, pushes `Statement::Move { target: temp, source: place }`,
and returns the temporary. Only the `StructLit` arm of `lower_value`
(`compiler/src/own/lower.rs:158-164`) calls it — array elements are out of scope
(AR-15).

### Integration Points

- `emit_store` and the `take`-argument lowering are **unchanged**.
- A guarded or short-circuit field (`cond ? b.value : other`) is an expression,
  not a place, so it is neither marked nor moved; it is out of scope (see
  Error Handling).

## Code Examples

### Example 1: A field moved into a literal

```clorraine
struct Box<T> { value: T; }
func demo(): void {
    let b = Box<String> { value: stringOf("x") };
    b = Box<String> { value: b.value };
}
```

The RHS lowers to a move of `b.value` into a temporary (zeroing `b.value`) and
builds the literal from the temporary; the assignment fast path then drops the
empty old `b` and stores. The handle is freed once.

### Example 2: A temporary-binding form

```clorraine
struct Box<T> { value: T; }
func demo(): void {
    let b = Box<String> { value: stringOf("x") };
    let t = b.value;                     // Move: t = b.value; b.value = NULL;
    b = Box<String> { value: t };        // t moved into the literal
}
```

`t` is moved into the literal and zeroed, so `t` and `b.value` are not the same
owner.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A read of an already-moved field | `move_target` records the partial move; the second read reports `E10020` | AR-14 |
| A deeper-than-top-level path (`o.inner.s`) | `move_place` reports `E10026` (v1) | AR-15 |
| A field of a borrowed parameter | `move_place` reports `E10034` | AR-15 |
| A duplicable place | Copied, not moved; unchanged | AR-2 |
| A type mentioning a generic parameter | Left untouched (`mentions_type_param`); resolved when the instance is emitted | AR-11 |
| A guarded/short-circuit field (`cond ? b.value : other`) | Out of scope; not marked or moved | AR-15 |

> **Traceability:** Every error-handling strategy references the Ambiguity
> Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- `compiler/tests/stage_spec_test.rs`: the temporary form and the reassignment
  form emit the source zero (`b.value = NULL` / `cl_zero(&b.value)`) and no
  `__cl_save` for a moved handle; `p = Pair<String> { first: p.first, second: p.first }`
  reports `E10020`; `Box { value: o.inner.s }` reports `E10026`; a `borrow`
  parameter's field reports `E10034`.
- `compiler/tests/codegen_run_spec_test.rs`: the borrow control and the
  non-reading fast-path control run unchanged.
- `compiler/src/own/tests.rs`: a duplicable literal field is not moved; a field
  whose type mentions a type parameter is left untouched.
- `compiler/tests/partial_move_impl_test.rs`: the temporary and reassignment
  forms run cleanly (no double-free abort).
