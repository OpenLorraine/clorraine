# Guards and Cleanup: Call Value Integrity

> **Document**: 03-04-guards-and-cleanup.md
> **Parent**: [Index](00-index.md)

## Overview

Fix DEF-36 (moving out of a fixed-array parameter zeroes the parameter pointer), DEF-37 (a
`do`/`while` guarded condition never re-evaluates its value), and DEF-39 (a re-materialized
growable-array operand leaks in the embedded guard spelling).

**Decisions per:** AR #8 (goto-free do-while), AR #9 (limited growable drop expression), AR #10
(parameter zeroing).

## Architecture

### Current Architecture

- A move out of an array source emits `memcpy` plus `cl_zero(&source)`
  (`emit_move_zero`, `compiler/src/codegen/function.rs:1101-1109`); for a C-adjusted array
  parameter (`ClString *a`) the copy over-reads the pointer slot and `cl_zero` nulls the pointer, so
  the later cleanup dereferences NULL. The element-walk precedent exists for drops
  (`emit_fixed_array_drop`, `compiler/src/codegen/drop.rs:95`, `fixed_array_drop_expr`, `:163`).
- A `do`/`while` body's fall-through is printed as `continue;` when the condition block is the loop
  branch (`classify_jump`, `compiler/src/codegen/control.rs:243-262`; `LoopKind::DoWhile`,
  `:362-375`), so the condition block's guard and store are emitted after the `continue` and never
  run per iteration.
- The embedded guard spelling drops a previously materialized value only when `drop_expr_text` has a
  form for the type (`compiler/src/codegen/guards.rs:316-352`); growable `Array<T>` has no form
  (`compiler/src/codegen/drop.rs:139-160`), unlike the statement spelling which drops through
  `emit_growable_array_drop` (`:112-129`).

### Proposed Changes

1. **Parameter move and zeroing (DEF-36).** A by-reference fixed-array parameter is C-adjusted to
   a pointer (`ClString *a`), so a move must go through the array value on **both** the source and
   the target of the move. When either side of a `Statement::Move` is such a parameter, the copy
   targets the elements (`memcpy(&dst[0], src, sizeof(dst))` or an element-wise copy for
   non-trivially-copyable elements) and the zero uses the element walk (recursive `cl_zero` on
   each element handle), never `cl_zero(&param)` and never `sizeof(param)`. The by-reference
   parameter is detected from the move place's base plus its array type; `is_pointer_param` keeps
   its current Borrow-only meaning, so the parameter ABI is unchanged. True array locals keep the
   existing `cl_zero` byte-zero (pinned by `array_value_integrity_impl_test.rs:241`).
2. **Do-while condition evaluation (DEF-37).** A guarded `do`/`while` is emitted **goto-free** with
   a first-iteration flag:

   ```c
   {
       i32 __cl_first = 0;
       for (;;) {
           if (__cl_first) { <guards>; if (!(<cond>)) break; }
           __cl_first = 1;
           <body>
       }
   }
   ```

   The first iteration skips the guards and the test and runs the body once; every later
   iteration — a body fall-through and every explicit `continue` alike — reaches the guard
   statements and the test at the top of the loop, so the condition re-evaluates after every body
   iteration. `break` exits the `for`. An unguarded do-while keeps today's
   `do { ... } while (cond);` emission, because with no statements before the test a `continue`
   already reaches the condition. The form changes no IR shape and emits no loop-control `goto`,
   so `spec/17-codegen.md:24` (goto for cleanup ladders only) holds and the structured-loop tests
   keep their guarantee.
3. **Growable drop expression (DEF-39).** `drop_expr_text` gains a growable-array form **only when
   the element type needs no drop**: the buffer guard plus `FreeBuffer` and the count reset, e.g.
   `(__cl_tmp0.data ? (FreeBuffer(__cl_tmp0.data), __cl_tmp0.data = NULL, __cl_tmp0.count = 0) : 0)`.
   The embedded guard spelling then drops the previous value before re-materializing, exactly like
   the fixed-array case pinned by `runtime_value_integrity_impl_test.rs:364`. The statement path's
   element loop has no C99 expression form, so an `Array<T>` whose `T` needs a drop keeps today's
   skip in embedded guards; that limitation is recorded in `01-requirements.md` (Won't Have) and is
   not a regression. ST-29 (`Array<i32>`) is fully covered.

## Implementation Details

### Changed Functions

| Function | File | Change |
| -------- | ---- | ------ |
| `emit_move_zero` / move copy | `compiler/src/codegen/function.rs` | Both-sides parameter copy and element-walk zero; no signature change |
| Parameter-array detection | `compiler/src/codegen/function.rs` | Detect a by-reference array parameter from the move place's base and array type |
| `is_pointer_param` usage | `compiler/src/codegen.rs` | Semantics unchanged (Borrow-only); the parameter declaration and rendering are not touched |
| Do-while emission | `compiler/src/codegen/control.rs` | Goto-free flag-first loop when the condition carries guards |
| `drop_expr_text` | `compiler/src/codegen/drop.rs` | Growable-array expression form for element types that need no drop |
| `guard_expressions` | `compiler/src/codegen/guards.rs` | Use the new form where it was skipped |

### Integration Points

- The zero walk reuses the drop walk's structure; it adds no runtime function and leaves the
  parameter ABI untouched.
- The do-while fix is emission-local; the loop's IR shape (`BranchShape::Loop`, `LoopKind`) is
  unchanged, and the flag-first form emits no loop-control `goto`, so the frozen emitted-C
  contract (`spec/17-codegen.md:24`) and the structured-loop tests hold.
- The growable form frees the buffer through `FreeBuffer` and resets the count; it matches the
  statement spelling's buffer release and is limited to element types that need no element drop.

## Code Examples

```clorraine
func f(take a: String[2]): i32 {
    let b: String[2] = a;   // parameter source: copy then element-walk zero
    return i32(len(b[0]));
}

func main(): i32 {
    let i: i32 = 0;
    do {
        if (i == 1) { continue; }                        // reaches the guards/test
        i = i + 1;
    } while (vlen(mk()[0]) == 2 && i < 3);               // DEF-37: flag-first loop
    while (i < 3 && growable()[0].count > 0) { i = i + 1; }  // DEF-39
    return i;
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Array parameter on either side of a move | Pointer-aware copy; element-walk zero; parameter ABI unchanged | AR #10 |
| Unguarded do-while | Today's emission stays (a `continue` already reaches the test) | AR #8 |
| Guarded do-while fall-through, `continue`, embedded-only guards | The flag-first loop routes every edge through the guards and the test | AR #8 |
| Growable `Array<T>` with a drop-needing `T` in an embedded guard | Keeps today's skip; recorded in Won't Have | AR #9 |

> **Traceability:** Every error-handling strategy references the Ambiguity Register entry that
> resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: ST-26 – ST-31 and ST-38 in `07-testing-strategy.md` (host runs; ASan in
  close-out for the memory-safety cases per AR #16).
- Implementation tests: the both-sides parameter copy/zero shape (locals keep `cl_zero`), the
  goto-free flag-first do-while shape, the explicit-`continue` path, and the limited growable drop
  expression in the embedded spelling.
