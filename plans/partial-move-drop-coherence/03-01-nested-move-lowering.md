# Nested Move Lowering: Partial-Move / Drop Coherence

> **Document**: 03-01-nested-move-lowering.md
> **Parent**: [Index](00-index.md)

## Overview

This component makes a value-consuming read of an owned place a move that zeroes
the source, wherever the read occurs. It fixes the temporary-binding form
(DEF-11) and the `take`-argument projection gap, and it provides the source
zeroing that the assignment component (03-02) depends on. It realizes
`spec/17-codegen.md` P3 for nested reads.

## Architecture

### Current Architecture

`lower_value` renders a struct or array literal field as a plain value
(`compiler/src/own/lower.rs:158-170`); a non-duplicable place field is copied
without zeroing. `take_move_out_places` matches only a whole
`ValueKind::Place`, not a `ValueKind::Member`/`Index`
(`compiler/src/own/call.rs:317`). A whole-place move already zeroes the source
through `Statement::Move` (`compiler/src/own/statements.rs:179-189`).

### Proposed Changes

Add one consume helper that moves a non-duplicable place through the existing
`Statement::Move` and use it at every value-consuming site, so the "copy plus
zero the source" rule has a single owner.

## Implementation Details

### New/Changed Functions

```rust
/// Consumes a place as a value, moving an owned value so its source is zeroed.
///
/// A duplicable place is copied (returned as a place value). An owned
/// (non-duplicable) place is materialized into a temporary through
/// `Statement::Move`, which copies the value and then zeros the source, so a
/// later drop of the source cannot free the value again. This is the one owner
/// of the "a move lowers to a copy plus a zero of the source" rule
/// (`spec/17-codegen.md` P3).
fn consume_place(&mut self, place: Place, span: Span) -> Value
```

`consume_place` returns `Value::place(place)` when `TypeTable::duplicable(place.ty)`
is true; otherwise it allocates a temporary of the place's type, pushes
`Statement::Move { target: temp, source: place }`, and returns a place value for
the temporary.

### Integration Points

- `lower_value`'s `StructLit` and `ArrayLit` arms call `consume_place` for each
  field/element whose value is a place (`compiler/src/own/lower.rs:158-170`).
- `take_move_out_places` calls `consume_place` for a `take` argument that is a
  place, including a field projection (`compiler/src/own/call.rs:302-322`).
- `emit_store` keeps its existing behavior, now expressed through the same rule
  (`compiler/src/own/statements.rs:179-189`).

## Code Examples

### Example 1: A field moved into a literal

```clorraine
struct Box<T> { value: T; }
func demo(): void {
    let b = Box<String> { value: stringOf("x") };
    let t = b.value;                     // Move: t = b.value; b.value = NULL;
    b = Box<String> { value: t };        // t moved into the literal
}
```

The emitted C moves `t` into a temporary and zeroes `t`, so `t` and `b.value`
are not the same owner.

### Example 2: A `take` argument that is a field

```clorraine
struct Box<T> { value: T; }
func consume(take s: String): void {}
func demo(): void {
    let b = Box<String> { value: stringOf("x") };
    consume(b.value);                    // b.value is moved and zeroed
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A read of an already-moved place | The ownership stage reports `E10020` before lowering; the lowering does not move it twice | AR-14 |
| A borrow/lend read | Not a move; `consume_place` is not called for a lend argument | AR-12 |
| A deeper-than-top-level partial move | Remains `E10026` (v1) | AR-15 |
| A duplicable place | Copied, not moved; unchanged | AR-2 |

> **Traceability:** Every error-handling strategy references the Ambiguity
> Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- `compiler/tests/stage_spec_test.rs`: the emitted C for the temporary form
  contains the source zero (`cl_zero(&b.value)` or `b.value = NULL`) and builds.
- `compiler/tests/codegen_run_spec_test.rs`: the temporary form runs and frees
  once (ASan clean).
- `compiler/src/own/tests.rs`: an owned field passed to a `take` parameter is
  moved (lowered IR contains a `Move`/`MoveOut` for the field).
