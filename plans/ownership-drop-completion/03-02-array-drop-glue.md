# Array Element Drop Glue: Ownership and Drop Model Completion

> **Document**: 03-02-array-drop-glue.md
> **Parent**: [Index](00-index.md)

## Overview

This component gives the drop ladder an array arm: a fixed array drops its
elements first-to-last, and the growable `Array<T>` drops its elements
first-to-last and then frees its buffer. It also removes the `E90000` trigger
that blocked `Array<move-only>`, so the growable container accepts a
move-only element. It realizes R5 and R6.

## Architecture

### Current Architecture

`Codegen::emit_drop_expr` (`compiler/src/codegen/drop.rs:32-63`) matches a
handle (release through its deleter, then `NULL`), a struct (recurse into the
owned fields), and everything else (no-op). A `TyKind::Array` therefore emits
nothing, and a `Struct("Array", …)` recurses only into its `data: Buffer` field,
freeing the buffer but leaking every element. `Codegen::emits_drop`
(`drop.rs:70-95`) returns true for a handle and a struct and false otherwise, and
`Codegen::needs_drop` (`drop.rs:98-104`) is `!duplicable && !is_view && …`.

`Sema::check_sequence_element` (`compiler/src/sema/mod.rs:266-289`), called at
`sema/mod.rs:240`, reports `E90000` for an `Array` whose element is not
duplicable.

### Proposed Changes

Fixed arrays become owned after `03-01`, so `needs_drop` already returns true for
a fixed array of move-only elements. This component adds the emission:

1. an array arm in `emit_drop_expr` for a fixed array,
2. a special case for the growable `Array` before the generic struct recursion,
3. an array arm in `emits_drop`,
4. the removal of the `E90000` trigger.

## Implementation Details

### Fixed Array

For `TyKind::Array { element, extents }` with every extent present, the elements
drop first-to-last (AR-7). Because the C declaration is a multi-dimensional array
(`ClString grid[2][2]`), the drop must index **one dimension at a time**; a flat
index would treat a row as an element and index out of bounds (PF-002). A small
recursive helper emits one loop per extent:

```rust
/// Emits the drop of a fixed array by recursing one extent per level.
///
/// The C declaration keeps every extent (`ClString grid[2][2]`), so each level
/// indexes one dimension; a flattened index would name a row, not an element.
fn emit_fixed_array_drop(&mut self, c_name: &str, element: &Ty, extents: &[usize]) {
    match extents.split_first() {
        None => self.emit_drop_expr(c_name, element),
        Some((head, tail)) => {
            for i in 0..*head {
                self.emit_fixed_array_drop(&format!("{c_name}[{i}]"), element, tail);
            }
        }
    }
}
```

The array arm calls it only when an element owns storage:

```rust
TyKind::Array { element, extents } if extents.iter().all(Option::is_some) => {
    if self.needs_drop(element) {
        let dims: Vec<usize> = extents.iter().filter_map(|e| *e).collect();
        self.emit_fixed_array_drop(c_name, element, &dims);
    }
}
```

Emitted form for `ClString arr[2]`:

```c
if (arr[0]) { FreeString(arr[0]); }
arr[0] = NULL;
if (arr[1]) { FreeString(arr[1]); }
arr[1] = NULL;
```

For `ClString grid[2][2]` the same shape is emitted for all four elements in
row-major, first-to-last order: `grid[0][0]`, `grid[0][1]`, `grid[1][0]`,
`grid[1][1]`.

### Growable `Array<T>`

Before the generic struct recursion, match `TyKind::Struct(name, args)` with
`name == "Array"` and one argument (the element type). The buffer pointer is read
once, then each element drops first-to-last, then the buffer is freed and the
array is emptied so a second drop is a no-op (O7). The `Array` case handles its
own `data` field, so the generic recursion must be skipped for it:

```rust
TyKind::Struct(name, args) if name == "Array" && args.len() == 1 => {
    let element = &args[0];
    // guard on data so a dropped array is empty and a second drop is a no-op
    // read the pointer once, loop 0..count dropping each element,
    // then FreeBuffer(data), then data = NULL and count = 0
}
```

Emitted form for `Array<String>`:

```c
if (xs.data) {
    ClString *__cl_elems = (ClString *)cl_buffer_ptr(xs.data);
    usize __cl_i = 0;
    while (__cl_i < xs.count) {
        if (__cl_elems[__cl_i]) { FreeString(__cl_elems[__cl_i]); }
        __cl_elems[__cl_i] = NULL;
        __cl_i = __cl_i + 1;
    }
    FreeBuffer(xs.data);
    xs.data = NULL;
    xs.count = 0;
}
```

The element C type is `super::types::c_type(element)`; `cl_buffer_ptr` is the
existing FFI (`compiler/runtime/shim_host.c:119`), already called by the emitted
`get`/`set` for `Array<T>` (`stdlib/std.mem.clo:91-102`). The element drop reuses
`emit_drop_expr`, so a nested struct or handle element drops correctly. The loop
locals are allocated with `self.unique_local("__cl_elems")` /
`self.unique_local("__cl_i")` so a user local of the same name is not shadowed
(PF-010). The indentation of the loop body is cosmetic and does not change any
existing output.

### The replaced element in `set<T>`

Un-staging `Array<move-only>` (AR-2) also makes `xs.set(i, take v)` legal for a
move-only element. The standard-library contract is to replace element `i`,
**dropping the old one** (`spec/appendix-stdlib.md:62`). The preflight read
`set`'s body (`stdlib/std.mem.clo`) and inferred a leak (PF-007), but the
emitter already releases an owned assignment target: `one[0] = v` lowers through
`emit_assign`, which drops the old element before the store. The emitted
`cl_set__String` body contains `FreeString((one).ptr[0])` before the store, and
an AddressSanitizer run of `push("a"); set(0, "b")` reports no leak and no
double free. No `stdlib/std.mem.clo` change is needed (AR-31); this component
only adds the verification test.

### `emits_drop` (no fixed-array arm)

`emits_drop` (`codegen/drop.rs:70-95`) only gates the assignment scratch local in
`emit_assign` (`codegen/function.rs:486`). That scratch local is declared with
`declarator`, which for an array is a C array (`ClString __cl_save[2]`) that
cannot be assigned: `__cl_save[2] = target;` is not C. Adding a fixed-array arm
would therefore route a fixed-array assignment through invalid C (PF-009), so
`emits_drop` stays `false` for a fixed array, exactly as for a slice. A
fixed-array literal assignment is emitted by the array path in
`emit_statement` (`codegen/function.rs:311-323`); whole fixed-array assignment
(`b = a;`, `b = id(a)`) is a pre-existing gap outside this plan and must not be
introduced or relied on. A growable `Array<T>` still returns true through its
`data: Buffer` field and needs no change.

### Removing the `E90000` Trigger

Delete `Sema::check_sequence_element` (`sema/mod.rs:266-289`) and its call at
`sema/mod.rs:240`, so `Array<move-only>` type-checks. `E90000` stays defined in
`spec/14-diagnostics.md` as the generic staged fallback and stays in `check.sh`'s
exclusion list; only the stale staged test data changes (AR-3):

- delete `spec/tests/staged/E90000_generic_staged.clo`;
- remove its row from `spec/tests/staged/README.md`;
- update the `E90000` comment in `spec/check.sh:92-94` and `:163-165` to say it is
  a dormant fallback with no live trigger.

### Integration Points

- `emits_drop` lives in `codegen/drop.rs:70-95` and gates the assignment scratch
  local in `emit_assign` (`codegen/function.rs:486`); it gains **no** fixed-array
  arm (PF-009).
- `needs_drop` (`codegen/drop.rs:98-104`) consults `is_view`, so a fixed array of
  move-only elements starts to need a drop without a direct edit.
- The growable loop calls an FFI function from the drop ladder; the emission
  order is elements then buffer, so a freed buffer is never read.

## Code Examples

```clorraine
import { String, stringOf } from std.text;

func pair(): String[2] {
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    return [a, b];
}   // caller scope exit: element 0 drops first, then element 1 (O2)

func grow(): void {
    let xs: Array<String> = arrayNew<String>();
    xs.push(stringOf("x"));     // accepted; no E90000
}   // elements drop first-to-last, then the buffer
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A second drop of the same array | the `data` guard and the `NULL` reset make it a no-op (O7) | AR-7 |
| A growable array with zero elements | the loop body never runs; the buffer is still freed | AR-11 |
| A fixed array of duplicable elements | `needs_drop` is false, so no drop is emitted | AR-11 |
| `Array<move-only>` | accepted after the trigger is removed; the element drop releases each element once | AR-2 |

> **Traceability:** the design choices above cite the Ambiguity Register entries
> that resolved them — see `00-ambiguity-register.md`.

## Testing Requirements

- A build-and-run test that a `String[2]` local and a returned `String[2]` drop
  each element exactly once (the C library aborts on a double free).
- A build-and-run test that a `String[2][2]` drops all four elements
  first-to-last, which also exercises the nested-literal fix in
  [03-05](03-05-nested-array-literals.md).
- A build-and-run test that `Array<String>` with pushed elements drops each
  element exactly once and frees the buffer.
- A build-and-run test that `xs.set(0, v)` on an `Array<String>` drops the old
  element (PF-007).
- A checker test that `Array<Buffer>` no longer reports `E90000`, and that the
  updated `sema/tests.rs` unit test agrees (PF-005).
- An emitter test for the fixed-array drop shape (one dimension per level) and
  the growable loop shape.
