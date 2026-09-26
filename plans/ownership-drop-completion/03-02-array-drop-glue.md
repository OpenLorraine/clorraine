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

For `TyKind::Array { element, extents }` with every extent present, the count is
the product of the extents, and each element drops first-to-last (AR-7):

```rust
TyKind::Array { element, extents } if extents.iter().all(Option::is_some) => {
    if self.needs_drop(element) {
        let count: usize = extents.iter().filter_map(|e| *e).product();
        for i in 0..count {
            self.emit_drop_expr(&format!("{c_name}[{i}]"), element);
        }
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
`emit_drop_expr`, so a nested struct or handle element drops correctly. The
indentation of the loop body is cosmetic and does not change any existing output.

### `emits_drop`

Add a fixed-array arm so the assignment scratch local is used when an element
needs a drop:

```rust
TyKind::Array { element, extents } if extents.iter().all(Option::is_some) => {
    self.emits_drop(element)
}
```

A slice keeps the `false` result (a view owns nothing). A growable `Array<T>`
already returns true through its `data: Buffer` field and needs no change.

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

- `emits_drop` gates the assignment scratch local (`codegen/cleanup.rs`), so the
  fixed-array arm is required for a correct assignment drop.
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
- A build-and-run test that `Array<String>` with pushed elements drops each
  element exactly once and frees the buffer.
- A checker test that `Array<Buffer>` no longer reports `E90000`.
- An emitter test for the fixed-array drop shape and the growable loop shape.
