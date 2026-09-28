# Array Value Integrity Implementation Plan

> **Feature**: Fix the remaining fixed-array value defects: multi-extent element loops bind the row (DEF-15), fixed-array value assignment emits invalid C (DEF-22), a projection over an array-returning call emits invalid C (DEF-21), and `len` on an owned `String` yields 0 (DEF-19)
> **Status**: Planning Complete
> **Created**: 2026-09-28
> **Implements**: DEF-15, DEF-19, DEF-21, DEF-22
> **CodeOps Artifact Schema**: 1

## Overview

Four filed defects share one surface: the compiler mishandles fixed-array
*values*. The element form of `for` over a multi-extent array binds the base
element instead of the row, so `for (row in m) { row[0] }` is rejected for a
row of type `i32[3]` and the untyped `for (borrow row in m)` form silently
walks every scalar. Fixed-array value assignment (`let b = a;`, `let r = m[0];`,
`let c = s.v;`, whole-array moves) is accepted by the checker and then emitted
as C array assignment, which no C compiler accepts. A projection over an
array-returning call (`mk()[i]`) renders the hidden out-parameter call inline,
which the C compiler rejects. And `len` of an owned `String` falls through to
the literal `0` instead of its byte count.

All four violate the frozen specification: `spec/08` AR-3 and section 5 define
rows as the elements of a multi-extent array, `spec/03` section 3 requires
copies for duplicable aggregates and moves for move-only aggregates, `spec/04`
section 9 defines indexing and evaluation order, and `spec/02` section 9 plus
`spec/04` section 11 define `String`'s implicit widening and `len`.

The fix reuses the existing machinery: the element type is derived from the
array's remaining extents, fixed-array assignment lowers to the shim's
whole-array byte copy, the existing `Materialize` value defers the call result
into a compiler temporary before the projection, and the `len` intrinsic gains
the missing `String` and outer-extent cases.

## Minimum-Sufficient Baseline

**Original goal:** accepted programs that copy, move, iterate, index, or take
the length of fixed-array values must compile to valid C and match the
specification.

**Smallest viable design:** derive the element type and count from the array's
extents in the checker and loop lowering, and spell a borrow row's address
temporary as a C pointer-to-array; emit a whole-array byte copy with the C
runtime `memcpy` in `codegen/function.rs` (source zeroed on move; a call result
is first written into a temporary and then moved) and reuse the drop and save
paths already present; lift the fixed-array exclusion in the existing
materialize helpers so a call result is written once into a compiler temporary;
add the `String` and outer-extent arms to the `len` intrinsic.

**Excluded machinery:** None — no new layer, dependency, harness, framework,
or refactor; every change is a direct edit in existing functions and the C
runtime `memcpy`.

**Approved complexity:** None.

## Document Index

| #   | Document                                                       | Description                                 |
| --- | -------------------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)                 | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                           | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                             | Feature requirements and scope              |
| 02  | [Current State](02-current-state.md)                           | Analysis of current implementation          |
| 03-01 | [Multi-Extent Element Loops](03-01-multi-extent-element-loops.md) | Element typing, count, and fixed-array value assignment |
| 03-02 | [Array Value Projections](03-02-array-value-projections.md)    | Call-result materialization and `len`       |
| 07  | [Testing Strategy](07-testing-strategy.md)                     | Test cases and verification                 |
| 99  | [Execution Plan](99-execution-plan.md)                         | Phases, sessions, and task checklist        |

## Quick Reference

### Usage Examples

```clorraine
func rows(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let s: i32 = 0;
    for (row in m) {              // row is i32[3]; two iterations
        s = s + row[0] + row[1] + row[2];
    }
    return s;                     // 21
}

func values(): i32 {
    let a: i32[2] = [1, 2];
    let b: i32[2] = a;            // copy of a duplicable array
    let r: i32[3] = [[1, 2, 3], [4, 5, 6]][0];   // row copy
    let x: i32 = mk()[1];         // call result indexed once
    return b[0] + r[0] + x;
}
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Plan scope | DEF-15, DEF-22 (new), DEF-21, DEF-19; DEF-18/DEF-20 in a second plan (AR-1, AR-10) |
| Array copy/move | Whole-array byte copy through the existing shim memcpy; move zeroes the source (AR-4, AR-7) |
| Call-result projection | Materialize the call once, then index; element moves stay E10026 (AR-5) |
| `len` | Outermost element count for fixed multi-extent arrays; byte count for `String` (AR-6) |
| Verification | `make check` per task, `make verify` at close (AR-8) |

## Related Files

- `compiler/src/own/loops.rs` — element-loop typing, count, and element binding
- `compiler/src/sema/stmt.rs` — checker element type for `for ... in`
- `compiler/src/types/mod.rs` — shared type rules (`Ty::iterated_element`)
- `compiler/src/own/statements.rs` — initializer/store preparation and move lowering
- `compiler/src/own/expr.rs` — aggregate consume points, assignment checking, and the ownerless-element predicate
- `compiler/src/own/call_check.rs` — `take` argument ownership effect
- `compiler/src/own/place.rs` — place resolution (`place_target`)
- `compiler/src/codegen/function.rs` — assignment, move, save path, and call emission
- `compiler/src/codegen/expr.rs`, `compiler/src/codegen/guards.rs` — value rendering and materialization
- `compiler/src/codegen/calls.rs` — the `len` intrinsic
- `compiler/src/codegen/types.rs` — C type and declarator spelling for borrow rows
- `compiler/src/own/lower.rs` — index lowering and operand materialization
- `compiler/tests/array_value_integrity_spec_test.rs`, `compiler/tests/array_value_integrity_impl_test.rs` — new suites
