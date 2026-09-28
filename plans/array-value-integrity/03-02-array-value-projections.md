# Array Value Projections and Length: Array Value Integrity

> **Document**: 03-02-array-value-projections.md
> **Parent**: [Index](00-index.md)

## Overview

This component makes a projection over an array-returning call evaluate the
call once and read the element, keeps element moves limited to E10026, and
fixes the `len` intrinsic for fixed multi-extent arrays and owned `String`s
(R4, R5; AR-5, AR-6).

## Architecture

### Current Architecture

`ExprKind::Index` lowering (`own/lower.rs:47`) materializes each side-effecting
operand through `materialize_guard_operand` (`own/lower.rs:521`) so the bounds
guard and the access read the same value once. That helper returns fixed-array
values unchanged because a C array cannot be assigned to a temporary
(`own/lower.rs:527`). The guard emission for `ValueKind::Materialize`
(`codegen/guards.rs:100`) writes `__cl_tmpN = value;`, which is invalid for a
hidden out-parameter call. The `len` intrinsic (`codegen/calls.rs:145`) handles
`str`/`static str`, growable `Array<T>`, slices, and single-extent fixed
arrays; a fixed multi-extent array folds every extent, and a `String` handle
falls through to `"0"` (`codegen/calls.rs:166`).

### Proposed Changes

1. `materialize_guard_operand` and `materialize_operand` accept a fixed-array
   `Call` value: the value is wrapped in `ValueKind::Materialize` exactly like
   a scalar call.
2. The two guard-emission arms for `Materialize` special-case a fixed-array
   `Call`: the statement arm emits the hidden out-parameter call into the
   temporary; the conditional-expression arm emits the same call as a comma
   operand, so a short-circuited operand never runs it.
3. `len` gains the `String` arm and returns the outermost extent for a fixed
   multi-extent array. The element-loop bound from `03-01` then needs no
   change.
4. The ownerless-element check reports E10026 at every move decision, not only
   the initializer, `take`, and return sites: the shared store preparation
   (`prepare_store`/`bind_value`), the aggregate-literal consume point
   (`consume_owned_place`), the `take` argument, and the return value. A
   place-rooted element in a projected-target assignment (`h.s = a[0]`, local
   or global target) is consumed through the same `consume_owned_place` after
   the value is evaluated, so the existing move rule reports E10026 there too;
   a legal top-level field move on that path is recorded as a partial move,
   matching the move the store already emits.

## Implementation Details

### New Types/Interfaces

```rust
impl FunctionChecker<'_, '_> {
    /// True when the expression reads an element of an ownerless temporary.
    ///
    /// The root of an index chain decides ownership: a local, parameter, or
    /// out-parameter root is a place the move checker guards at the binding
    /// sites and at projected-target assignments (through `consume_owned_place`),
    /// while a call root owns its storage only until the statement ends. Moving
    /// a non-duplicable value out of a call-rooted element is the v1
    /// element-move limit, reported as E10026.
    fn is_ownerless_element_move(&self, expr: &Expr) -> bool;
}
```

The helper lives with the aggregate consume points in `own/expr.rs` and is
called from the move decisions listed below. It reuses `place_target`
(`own/place.rs:535`), which already resolves an expression to its binding.

### New Functions/Methods

- **Materialize lift** — `own/lower.rs:521` and `:648`: replace the blanket
  fixed-array early return with "return unchanged unless the value is a
  `Call`". A pure fixed-array read never reaches the check (`is_pure_value`
  returns early), so only call results are wrapped.
- **Guard emission** — `codegen/guards.rs:100`: when the materialized inner
  value is a fixed-array `Call`, build the temporary place and push
  `call_with_destination` (`codegen/expr.rs:380`) instead of `tmp = value`.
  `codegen/guards.rs:262` pushes the same call as a comma operand so the
  conditional path stays lazy.
- **`len`** — `codegen/calls.rs:145`:
  - `TyKind::Handle(name) if name == "String"` →
    `(cl_string_view(value)).len`, reusing the widening helper the argument
    coercion already emits (`codegen/function.rs:625`).
  - `TyKind::Array { extents, .. }` (all `Some`) → the first extent instead of
    the product.
- **Element moves** — report E10026 for a non-duplicable element
  projection: `is_ownerless_element_move` at the move decisions below
  (call-rooted projections), and the shared `consume_owned_place` at
  projected-target assignments (place-rooted projections):

  | Site | Condition |
  | ---- | --------- |
  | `own/statements.rs::prepare_store` (`:234`; serves `let`, `const`, and plain assignment through `bind_value`) | target not duplicable, no place source, ownerless element projection |
  | `own/expr.rs::consume_owned_place` (`:48`; struct- and array-literal fields, `:151-160`) | value not duplicable, no place target, ownerless element projection |
  | `own/expr.rs::check_assign` projected targets (local and global), calling `consume_owned_place` after the value is evaluated | value not duplicable, place-rooted element projection (E10026 through `move_place`); a top-level field records a partial move |
  | `own/call_check.rs::check_argument_place` (`:260`) | `Mode::Take`, argument not duplicable, no place target, ownerless element projection |
  | `own/statements.rs::check_return` (`:673`) | return type not duplicable, ownerless element projection |

  The message and help reuse the registry rows for E10026
  (`spec/14-diagnostics.md:57`).

### Integration Points

- The store path from `03-01` makes the materialize assignment valid C for
  arrays; this component adds only the call-shaped special case.
- The `len` change makes the `for ... in` bound from `03-01` the outermost
  count for fixed arrays; slices already return their runtime outer length.
- No new diagnostic code is introduced; the existing E10026, E10040, and
  E10209 registrations stand.

## Code Examples

### Example 1: Call result indexed once

```clorraine
func mk(): i32[2] { return [7, 9]; }

func main(): i32 {
    let x: i32 = mk()[1];          // call runs once into a temporary
    return x;                      // 9
}
```

Emitted C (shape):

```c
i32 __cl_tmp0[2];
__cl_mk(__cl_tmp0);
x = __cl_tmp0[1];
```

### Example 2: Deferred evaluation inside a short-circuit operand

```clorraine
func boom(): i32[2] { panic("boom"); }

func main(): i32 {
    let ok: bool = false;
    if (ok && boom()[0] == 1) { return 1; }
    return 0;                      // boom never runs
}
```

### Example 3: Length

```clorraine
let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
let s: String = stringOf("hello");
let a: usize = len(m);             // 2, the row count
let b: usize = len(s);             // 5, the byte count
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Move of a non-duplicable element out of an ownerless temporary | E10026 at every move decision (store preparation, aggregate consume point, `take`, return) | AR-5 |
| Move of a non-duplicable place element into a projected target (`h.s = a[0]`) | E10026 through the shared consume point at the assignment | AR-5 |
| Element move through a borrow view or parameter | Unchanged; DEF-18 owns the view case | AR-10 |
| Non-duplicable rows without `borrow` | E10040, unchanged | AR-1 |
| Indexing a non-indexable base | E10209, unchanged | AR-11 |

> **Traceability:** Every error-handling strategy and design choice must
> reference the Ambiguity Register entry (AR #) that resolved it. See
> `00-ambiguity-register.md`.

## Known Limitations

- A materialized array-returning call can own elements; this plan does not
  release the materialized statement temporary (the existing scalar
  `Materialize` temporaries have the same property, and `spec/17`'s
  statement-temp drop is not yet implemented). A follow-up defect tracks the
  temporary lifetime (`DEF-23`); the ASan probes run with leak detection
  disabled. Views bound from such a temporary (`let v: str = mk()[0]`) are
  included in that follow-up.

## Testing Requirements

- Specification tests ST-16 – ST-21, ST-23, ST-24, ST-27 – ST-30 (projections and element moves) and ST-22 (`len`, implemented in Phase 1 with the outer-extent arm) in `07-testing-strategy.md`.
- Implementation tests for the emitted materialize shape, the short-circuit
  comma operand, the `len` renderings, and the E10026 sites (assignment,
  field-target assignment, aggregate-literal field).
- Unit tests for `is_ownerless_element_move` (place root, call root, nested
  index chain).
