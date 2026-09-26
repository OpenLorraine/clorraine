# Fixed-Array Return Lowering: Ownership and Drop Model Completion

> **Document**: 03-04-fixed-array-return.md
> **Parent**: [Index](00-index.md)

## Overview

A function may return a fixed array by value (`func pair(): String[2]`,
`spec/08-arrays-strings.md` section 6; F2), but C cannot return an array. This
component lowers a by-value fixed-array return through a hidden out-parameter.
It realizes R3 and resolves PF-001. It is the largest single item in the plan.

## Architecture

### Current Architecture

Classification alone is not enough. The emitter spells a function's return type
with `c_type` (`codegen.rs:401-407`, `codegen/function.rs:197-201`,
`codegen/program.rs:267-270`), and `c_type` of a fixed array is its **element**
type (`codegen/types.rs:36-42`). A fixed-array return would emit
`static ClString cl_pair(void)` and `return <array literal>`, which is not C.
`emit_closing_return` handles a slice but a fixed array falls to `return 0;`
(`codegen/cleanup.rs:322-333`). Today the checker rejects the program earlier
with `E10036`, so the emitter is never reached.

### Proposed Changes

Use a hidden out-parameter, the same shape the emitter already uses for a
pointer-lowered `borrow`/`take` parameter:

1. A function whose return type is a fixed array (`Ty::is_fixed_array`) is emitted
   as `void`, with a trailing parameter `T __cl_ret[N]`.
2. The return terminator copies the result into `__cl_ret` and returns `void`.
3. A call whose result is a fixed array passes the caller's destination array as
   the trailing argument; the destination is the call's result local.

C array parameters decay to pointers, so the callee writes through the caller's
storage, and the array is never copied by the C ABI. This is an internal C
signature change only; the Clorraine-level ABI is unchanged (revises AR-15).

## Implementation Details

### Signature

In `signature_with_symbol` (`codegen.rs:394-423`) and the lambda/closure
signature builders, when `self.substitute(&ret).is_fixed_array()`:

```rust
// The declared C return type becomes void; the result is written through a
// trailing out-parameter declared with the array's own declarator suffix.
let out = format!("{} __cl_ret", super::types::declarator(&ret, "").trim_end());
format!("{storage}void {symbol}({params}, {out})")
```

(The exact spelling is `declarator(&ret, "__cl_ret")`, which already yields
`ClString __cl_ret[2]`.) A prototype builder must use the same signature.

### Return

`emit_return` (`codegen/cleanup.rs:253-287`) and `emit_closing_return` gain a
fixed-array arm:

- assign the returned value into `__cl_ret` with the existing array-assignment
  path (`emit_array_assign` for a literal, the element-by-element move for a
  place), then `return;`;
- `emit_local_declarations` must not declare a second local result variable for a
  fixed-array return; `result_var` is the out-parameter name.

### Call Sites

`Statement::Call` already binds a result to a local. For a fixed-array result the
local is the destination; the call passes it as the trailing argument. A call
whose result is consumed directly (for example `let v = head(pair());`) is
materialized into a compiler temporary first, exactly as for a struct result.
C arrays decay, so passing the temporary onward needs no cast.

### Scope Boundary

Whole fixed-array **assignment** (`b = a;`) is a separate pre-existing gap
(`codegen/function.rs:483-518`; PF-009). This component does not introduce it and
must not route a fixed array through the assignment scratch local. If a returned
fixed array is assigned to a binding, the assignment goes through the array
element path, not `emit_assign`'s scratch path.

## Code Examples

```clorraine
func pair(): String[2] {
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    return [a, b];
}

func demo(): void {
    let p: String[2] = pair();   // the result is written into `p`
    let c: String = pair()[0];   // a temporary array, then index it
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Return of a slice | unchanged view-return path | AR-5 |
| Fixed-array return with a move-only element | element moves into the out-parameter; caller drops it first-to-last | AR-7 |
| A generic fixed-array return | monomorphization resolves the element before the signature is emitted | AR-6 |

> **Traceability:** the design choices above cite the Ambiguity Register entries
> that resolved them — see `00-ambiguity-register.md`.

## Testing Requirements

- A checker test that `func pair(): String[2]` compiles (no `E10036`).
- A build-and-run oracle that `let p: String[2] = pair();` releases each element
  exactly once and does not free the literals.
- A call-site test that `pair()[0]` reads the returned element.
- A no-regression test that a slice return and a struct return are unchanged.
