# Module-Initializer Integrity: Evaluation and Emission Integrity

> **Document**: 03-01-module-initializer-integrity.md
> **Parent**: [Index](00-index.md)

## Overview

Phase 1 makes module initializers correct: fixed-array-returning calls bind through the hidden
destination in every accepted position (DEF-117), impure guard operands are materialized once
(DEF-121), and a slice-typed binding from an array literal is rejected instead of emitting an
array assignment (DEF-122).

## Architecture

### Current Architecture

- `emit_global_init` (`codegen/program.rs:640-684`) renders the lowered initializer and stores it;
  fixed-array calls are excluded from the element fill (`fills_element_wise:1513`) and reach the
  store without a destination; only the function path appends one (`call_with_destination`,
  `codegen/expr.rs:503-517`; `materialize_init_local:799-842` stores through a destination-less
  `self.value` at `:826`).
- The same store re-renders the expression after `emit_guards` (`program.rs:682-683`); the
  guarded-sub-operand rewrite (`materialize_guarded_sub_operands:844-926`) is reached only from
  the array-fill and intrinsic-operand paths, not the scalar single-assignment path.
- `check_array_literal` (`sema/expr.rs:748-860`) adopts a `None`-bearing expected extent
  (`:755-761`, returns `[None]` at `:851-855`), so a slice-typed target silently accepts a fixed
  literal; emission then spells `x = { 1, 2 };`.

### Proposed Changes

1. **Hidden destinations (DEF-117).** `emit_global_init` applies a module-init materialization
   pass before rendering: every fixed-array `Call`/`ClosureCall` in the lowered value — all six
   positions — binds once through the destination-aware renderers, and
   `materialize_init_local`'s fixed-array arm stores through the destination instead of
   `self.value`. The function path is the contract mirror (`own/lower.rs:999-1035`,
   `codegen/guards.rs:164-212`).
2. **Single evaluation (DEF-121).** The scalar single-assignment path runs
   `materialize_guarded_sub_operands` before `emit_guards`, so an impure index, divisor, or cast
   operand is stored once into an init local shared by the guard and the read; evaluation order
   and per-branch laziness are preserved (`program.rs:844-853`).
3. **Slice literals (DEF-122).** `check_array_literal` stops adopting `None`-bearing expected
   extents, so the literal keeps its fixed type and `check_conversion` reports `E10200`; module
   and function-body spellings reject alike. `slice(array, lo, hi)` initialization is untouched.

## Implementation Details

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `codegen/program.rs` | init emission and the materialization helpers |
| `codegen/expr.rs` | destination-aware renderers (reused) |
| `codegen/guards.rs` | the operand contract (reused) |
| `sema/expr.rs` | the literal adoption rule |
| `own/lower.rs` | the function-path contract (reused, unchanged) |

## Code Examples

```clorraine
// DEF-117: each position builds under -Werror
let a: i32[2] = mk();                 // whole value
let b: i32 = mk()[1];                 // element projection
let c: Box = mkBox();                 // member projection
eat(mk());                            // user-fn argument
use(slice(mk(), 0, 2));               // intrinsic argument
let d: i32[2] = pick() ? mk() : y;    // conditional arm

// DEF-121: idx() runs once
let v: u8 = s[idx()];

// DEF-122: rejected with E10200; the sanctioned spelling stays accepted
let x: i32[] = [1, 2];                // E10200: a literal has no backing to view
let arr: i32[2] = [1, 2];
let y: i32[] = slice(arr, 0, 2);      // accepted
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Slice-typed target from an array literal | one `E10200` at the initializer | AR-B5 |
| Fixed-array call in any init position | accepted; destination-bound emission | AR-B13 |
| Impure guard operand | materialized once; guard and read share it | AR-B13 |

> **Traceability:** every error-handling strategy back-references its register entry. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification cases ST-1 – ST-7 ([07-testing-strategy.md](07-testing-strategy.md)), including
  all six DEF-117 positions with strict builds and run cases, the DEF-121 counting-index run, and
  the DEF-122 rejection with the `slice()` control.
- Host ASan probe for the DEF-121 shape (the double evaluation is a global-buffer-overflow at
  baseline).
- Implementation pins: the destination-carrying emission per position, the shared init local for
  the guard operand, byte-identical re-emission of unaffected initializers.
