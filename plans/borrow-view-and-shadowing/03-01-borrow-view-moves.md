# Borrow-View Moves: Borrow-View Moves and Shadowed Narrowing

> **Document**: 03-01-borrow-view-moves.md
> **Parent**: [Index](00-index.md)

## Overview

This component rejects an ownership transfer whose source is a
`for (borrow e in xs)` element view, so the v1 element-move limit holds for
views exactly as it holds for direct element expressions (R1; AR-3, AR-4, AR-9).

## Architecture

### Current Architecture

`bind_for_in_element` (`own/loops.rs:302-367`) declares the element binding as
`BindingKind::Local` with `Mode::Borrow` and roots its place at a `DerefTemp`
that aliases the iterable element. Every ownership transfer reaches
`move_place` (`own/place.rs:182`), whose borrowed-parameter rule tests
`BindingKind::Param` only, so the view binding slips through: the projection
path marks the binding moved and the emitter nulls the borrowed element.

### Proposed Changes

Add one guard to `move_place` before any state change: when the binding is a
local borrow view, report the `E10026` registry row through the existing
`report_element_move_limit` helper and return without recording a move or
clearing projections.

```rust
// A borrow element view never owns its element, so an ownership transfer
// through it is the v1 element-move limit even though the view binds as a
// local. (The borrowed-parameter rule below covers parameters.)
if binding.kind == BindingKind::Local && binding.mode == Mode::Borrow {
    self.report_element_move_limit(span);
    return;
}
```

The binding's `base` is a `DerefTemp` for every borrow element view
(`own/loops.rs:335-338`); `Mode::Borrow` on a `BindingKind::Local` occurs only
there, so the guard cannot catch another construct (AR-9, AR-13).

## Implementation Details

### New Types/Interfaces

None.

### New Functions/Methods

None; the guard is an internal branch of the existing `move_place`.

### Integration Points

- `check_argument_place` (`own/call_check.rs`, `Mode::Take`) reaches the guard
  through `move_target` for a take argument.
- `prepare_store`/`bind_value` (`own/statements.rs`) reach it for an owned-local
  binding and for a projected field path.
- `check_return` (`own/statements.rs`) reaches it for a return.
- `consume_owned_place` (`own/expr.rs`) reaches it for an aggregate-literal
  field.
- Parameters keep the existing `E10034` path because their kind is `Param`.

## Code Examples

### Example 1: Rejected take through the view

```clorraine
func eat(take s: String): void { }

func main(): i32 {
    let xs: String[2] = [stringOf("a"), stringOf("bb")];
    for (borrow e in xs) {
        eat(e);              // E10026: element moves are limited in v1
    }
    return 0;
}
```

### Example 2: Guards stay accepted

```clorraine
func vlen(s: str): i32 { return i32(len(s)); }

func main(): i32 {
    let xs: String[2] = [stringOf("a"), stringOf("bb")];
    let total: i32 = 0;
    for (borrow e in xs) {
        total = total + vlen(e);   // a lend is not a move
    }
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    for (borrow row in m) {
        row[0] = row[0] + 10;      // a write through the view stays valid
    }
    for (row in m) {
        let copy: i32[3] = row;    // a copy-form binding owns its copy
    }
    return total;
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Ownership transfer through a borrow element view | `E10026` (registry message and help) at the transfer | AR-3, AR-7, AR-9 |
| Move out of a `borrow` parameter | Unchanged: `E10034` | AR-13 |
| Reads, lends, and writes through the view | Unchanged: accepted | AR-13 |
| Copy-form loop binding | Unchanged: owns its copy | AR-13 |

> **Traceability:** Every error-handling strategy and design choice must
> reference the Ambiguity Register entry (AR #) that resolved it. See
> `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests ST-1 – ST-5 (rejections) and ST-6 – ST-8 and ST-13
  (guards) in `07-testing-strategy.md`.
- Unit coverage in `own::tests` for the view rejection and the unchanged
  copy-form binding.
- Implementation tests pin that the rejection is a single report and that a
  borrow-parameter move still reports `E10034`.
