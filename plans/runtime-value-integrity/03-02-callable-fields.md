# Callable Fields: Runtime Value Integrity

> **Document**: 03-02-callable-fields.md
> **Parent**: [Index](00-index.md)

## Overview

This component closes the two callable-field defects: a call whose receiver is a runtime
index must bounds-guard that index before the call (DEF-26), and a `func` value whose
return is a fixed array must carry the hidden out-parameter ABI in its C type and at its
call sites (DEF-27).

## Architecture

### Current Architecture

A callable field lowers to `ValueKind::Member` over the receiver
(`compiler/src/own/call.rs:109-127`) and the call renders as
`(receiver.field)(args)` (`compiler/src/codegen/expr.rs:408-411`). Arguments are guarded
on every path, but the receiver is not: the statement call path
(`compiler/src/codegen/function.rs:461-465`), the fixed-array call assign path
(`:730-732`), and the fixed-array return path
(`compiler/src/codegen/cleanup.rs:339-341`) guard only `args`. The expression-guard path
already guards a call receiver (`compiler/src/codegen/guards.rs:54-59`, `:240-249`).

`func_pointer_type` and `func_pointer_declarator` (`compiler/src/codegen/types.rs:58-73`)
render `{returns} (*)({params})`, while a fixed-array-returning function is emitted with a
hidden trailing out-parameter (`compiler/src/codegen.rs:403-424`). Call destinations are
derived only for named functions (`compiler/src/codegen/function.rs:471-476`,
`:580-586`), so a call through a func value drops the destination.

### Proposed Changes

**1. Receiver guards (DEF-26; AR-8, AR-12).** Emit the receiver's guards wherever the
arguments are guarded, on all three paths:

- `emit_call`'s general arm and its fixed-array `array_result` branch,
- `emit_fixed_array_call_assign`,
- `emit_fixed_array_return`.

The guard is the ordinary guard emission (`codegen/guards.rs`), which descends through
`ValueKind::Member` into the receiver's index and emits the existing `E11211` bounds trap.
The receiver therefore evaluates once and traps before the call, matching every other
runtime-index use.

**2. Func-pointer ABI (DEF-27; AR-9, AR-12).**

- `func_pointer_type` / `func_pointer_declarator` (`codegen/types.rs`) render a
  fixed-array return through the hidden out-param ABI: the return type becomes `void` and
  a trailing destination parameter is appended with the same declarator spelling the callee
  declaration uses (`codegen.rs:403-424`; a multi-extent fixed array uses a
  pointer-to-row declarator, so `declarator` is the shared helper). This covers struct
  fields, locals, and parameters, because they share the helpers.
- `emit_call` resolves the callee's return type for a fixed-array result when the callee
  is not a named function: for a func-typed field the callee value carries the type, and
  for a local func-typed value the emitter resolves the local's declared type by name
  (the emitter already maps locals). A call through a func value then appends the
  destination exactly like a named call: the result place when one exists, a discard
  temporary plus its drop for a discarded fixed-array result.
- Assignment compatibility follows from the shared declarator, so
  `let g: func(i32): i32[2] = make;` and a field store of `make` compile.
- Out of scope: a closure struct's `fn` field and a lifted lambda's own signature are
  rendered inline with `c_type` (`codegen/program.rs:267-283`,
  `codegen/function.rs:198-220`), not by these helpers; the accepted-but-invalid fixed-
  array-returning form is filed as `DEF-30` (AR-9).

The field-call shape (`(h.run)(3, __cl_call_1);`) is already correct; only the declared
pointer type and the func-value destination detection change.

## Implementation Details

### New Types/Interfaces

None.

### New Functions/Methods

- A shared helper that appends the hidden destination parameter to a signature's C
  parameter list, used by both the plain and the declarator rendering.
- A codegen helper that resolves a callee value's return type (named function, func field,
  or func-typed local) for destination/discard decisions.

### Integration Points

- `codegen/guards.rs` Call arm is the reference guard behavior; the three call sites align
  with it.
- `codegen/expr.rs` `call_with_destination`/`render_call` already append the destination
  argument; only the decision to use it widens.
- `codegen/cleanup.rs` `emit_fixed_array_return` gains the receiver guard for returns.
- Existing behavior to preserve: direct named calls, scalar-returning func values, the
  field-over-function precedence rule, and the FFI mapping (`abi.rs` maps arrays to `APTR`
  and is not a `func(...)` language type — unchanged).
- Out of scope: closure/lifted-lambda fixed-array returns (`DEF-30`).

## Code Examples

### Example 1: Runtime-index receiver traps out of range

```clorraine
struct Ops { run: func(i32): i32; }

func bump(x: i32): i32 { return x + 1; }

func main(): i32 {
    let ops: Ops[2] = [Ops { run: bump }, Ops { run: bump }];
    let i: i32 = 5;
    ops[i].run(1);          // E11211 bounds trap before the call
    return 0;
}
```

### Example 2: A func field returning a fixed array compiles

```clorraine
struct Maker { make: func(i32): i32[2]; }

func pair(x: i32): i32[2] { return [x, x + 1]; }

func main(): i32 {
    let m: Maker = Maker { make: pair };
    let a: i32[2] = m.make(3);   // C type: void (*)(i32, i32 *)
    return a[1];                 // 4
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Runtime index out of range on a receiver | Existing `E11211` trap before the call, on every call path | AR-8 |
| Func value with a fixed-array return | Hidden destination in the C pointer type; destination appended at the call | AR-9 |
| Discarded fixed-array result through a func value | Discard temporary plus its release (existing named-call shape) | AR-9 |
| Discarded result through a local func-typed callee | The callee's declared type is resolved from the function's locals | AR-9 |
| Closure struct field or lifted lambda with a fixed-array return | Out of scope; filed as `DEF-30` | AR-9 |
| Scalar-returning func values and named calls | Unchanged | AR-12 |

> **Traceability:** Every error-handling strategy and design choice references the
> Ambiguity Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests ST-12 – ST-17 in `07-testing-strategy.md`.
- Implementation tests pin the emitted C: the receiver bounds trap appears in the
  statement, fixed-array assign, and return shapes; the emitted pointer declarator carries
  the destination parameter; a discarded func-value call passes a destination, including
  the local func-typed callee form.
- C-acceptance is part of every run case (the suite builds the emitted C under
  `-Wall -Wextra -Werror`).
