# Multi-Extent Element Loops and Fixed-Array Values: Array Value Integrity

> **Document**: 03-01-multi-extent-element-loops.md
> **Parent**: [Index](00-index.md)

## Overview

This component makes the element form of `for` walk rows of a multi-extent
array and makes fixed-array values assignable in C. It owns the element type
derivation, the loop binding, and the whole-array store path shared by
declarations, reassignments, moves, fields, and rows (R1, R2, R3; AR-1, AR-2,
AR-4, AR-6, AR-7). The `len` intrinsic's outer-extent rule it relies on is
specified in `03-02`.

## Architecture

### Current Architecture

`check_for_in` reads the array's base `element` and ignores the remaining
extents, so `i32[2][3]` binds `i32` (`own/loops.rs:379`, `sema/stmt.rs:198`).
The loop bound is `len(iterable)` (`own/loops.rs:489`), which for a fixed
multi-extent array currently multiplies the extents. The element selection
already indexes the C array, so `m[index]` is the row once the type is right.
Statement assignment stores with `target = value;`
(`codegen/function.rs:577`, `:322`), which C accepts for scalars and structs
but not for arrays.

### Proposed Changes

1. One shared derivation, `Ty::iterated_element`, strips the first extent:
   `T[N][M]` and `T[][M]` iterate `T[M]` rows; `T[N]` and `T[]` iterate `T`.
2. Both `check_for_in` passes and the `E10040` duplicability check use the
   derived element; the loop bound stays `len(iterable)` and becomes correct
   with the `03-02` `len` rule. A `borrow` row's address temporary keeps its
   shape, but `codegen/types.rs` must spell `raw T[M]` as a C pointer-to-array
   (`T (*tmp)[M]`) so the address-of-row assignment is well-typed under
   `-Werror`.
3. A fixed-array store path emits a whole-array byte copy through the C
   runtime `memcpy`; a move zeroes the source after the copy; a reassignment
   drops the old target first; a self-referencing non-call assignment saves the
   old target into a zeroed local first; a call right-hand side is written into
   a compiler temporary (the discarded-call pattern) and then lowered as a move
   of that temporary, so the callee never reads and writes the target storage
   at once; a right-hand side that renders to the same place as the target
   emits no copy (a no-op).

## Implementation Details

### New Types/Interfaces

```rust
impl Ty {
    /// The element one step of an element loop binds.
    ///
    /// A fixed or slice array with more than one extent iterates its rows:
    /// the result keeps every extent after the first (`T[N][M]` and `T[][M]`
    /// both yield `T[M]`). A single-extent array or slice iterates its
    /// element type unchanged. Any other type yields `Ty::unknown()`; the
    /// caller reports or ignores it as before.
    pub fn iterated_element(&self) -> Ty;
}
```

`iterated_element` lives in `compiler/src/types/mod.rs` next to
`is_fixed_array` (`types/mod.rs:302`) and `is_slice` (`types/mod.rs:291`), so
the checker and the lowering share one rule.

### New Functions/Methods

No new codegen entry points are introduced. The store path is an internal
branch of the existing emitter:

- `emit_assign` (`codegen/function.rs:577`) gains a fixed-array branch:

  ```c
  memcpy(&target, &value, sizeof target);          /* copy */
  ```

  A same-place right-hand side (the value renders to the target itself) emits
  no copy. A value that reads the target through a non-call expression takes
  the existing save path with the array form of the save: `cl_zero(&save);
  memcpy(&save, &target, sizeof save);` before the store, then the save is
  dropped with the existing `emit_drop_expr` (`codegen/drop.rs:33`).

- A call right-hand side never reaches `emit_assign`: the `Assign` arm's
  fixed-array call case (`codegen/function.rs:336-353`) writes the call into a
  compiler temporary first (the discarded-call pattern at `:466-489`) and then
  lowers the assignment as the fixed-array `Statement::Move` from that
  temporary, so the callee cannot read and write the target storage at once
  and the old target is dropped by the move's reassignment drop.

- The `Statement::Move` arm (`codegen/function.rs:357`) copies the array bytes
  and then calls the existing `emit_move_zero` (`codegen/function.rs:702`),
  which already zeroes an array through `cl_zero(&source)`.

- `emit_reassigned_target_drop` (`codegen/function.rs:562`) already drops an
  assigned-over fixed array first to last through `emit_fixed_array_drop`
  (`codegen/drop.rs:40`); a byte copy replaces only the store step.

The byte copy uses the C runtime `memcpy`, available on all three profiles
(`spec/17` section 12); no inline loop and no new shim helper is added (AR-4).

### Integration Points

- `sema/stmt.rs::check_for_in` and `own/loops.rs::check_for_in` both call
  `iterated_element`, so the checker and the ownership pass agree.
- `own/loops.rs::bind_for_in_element` (`own/loops.rs:302`) keeps its shape;
  the passed element type changes and its address temporary needs the
  pointer-to-array spelling in `codegen/types.rs` (`declarator`/`c_type`).
- `emit_statement` (`codegen/function.rs:322`) already routes array literals
  and array-returning calls to their specialized emitters before reaching the
  new branch.
- The element-move limit (E10026) for ownerless temporaries is owned by
  `03-02`; this component's store path is reached only by moves the checker
  accepts.

## Code Examples

### Example 1: Row iteration and row copy

```clorraine
func main(): i32 {
    let m: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
    let s: i32 = 0;
    for (row in m) {                 // row: i32[3], two iterations
        s = s + row[0] + row[1] + row[2];
    }
    let r: i32[3] = m[1];            // whole-row byte copy
    return s + r[0];                 // 21 + 4
}
```

Emitted C (shape):

```c
memcpy(&r, &m[1], sizeof r);
for (index = 0; index < len(m); index++) {
    memcpy(&row, &m[index], sizeof row);
    s = (s + row[0]) + row[1] + row[2];
}
```

### Example 2: Move with a zeroed source

```clorraine
let a: String[2] = [stringOf("ab"), stringOf("cde")];
let b: String[2] = a;                // move; a is emptied
```

Emitted C (shape):

```c
memcpy(&b, &a, sizeof b);
cl_zero(&a);
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Non-duplicable rows without `borrow` | Keep E10040, now naming the row type | AR-1 |
| Move out of an ownerless temporary element (`mk()[0]`) | E10026, owned by `03-02` | AR-5 |
| `len` of a fixed multi-extent array | Outermost extent, owned by `03-02` | AR-6 |
| Self-referencing assignment | Save path, extended to arrays | AR-7 |

> **Traceability:** Every error-handling strategy and design choice must
> reference the Ambiguity Register entry (AR #) that resolved it. See
> `00-ambiguity-register.md`.

## Testing Requirements

- Unit tests for `Ty::iterated_element` over fixed, slice, nested, and
  single-extent shapes.
- Unit test for the raw pointer-to-array declaration (`raw T[M]`).
- Specification tests ST-1 – ST-15, ST-25, ST-26 in `07-testing-strategy.md`.
- Implementation tests for the emitted copy/move shape, the same-place no-op,
  the call-into-temporary assignment shape, and the counting-deleter release
  sites. The save path stays in the emitter for a non-call right-hand side that
  reads an owned target. An exact same-place assignment is a no-op, an owned
  self-assignment reports `E10020`, and a self-referencing call takes the
  temporary path; a conditional right-hand side (`ok ? a : b`) is accepted
  today and does reach the branch, but the C it emits is invalid — that form is
  filed as `DEF-25`, and the branch stays defensive until it is handled.
