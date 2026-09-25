# Lambdas: Clorraine v1 language completion

> **Document**: 03-02-lambdas.md
> **Parent**: [Index](00-index.md)

## Overview

This component implements lambda semantics for stages 1-4 (L1-L5). The syntax
and AST already exist (`compiler/src/ast/expr.rs:122`); only the checker and the
emitter are missing. A capture-less lambda is a plain C function pointer; a
capturing closure is a two-word callable (function pointer plus stack
environment) that may appear only inline (AR-10, AR-11, AR-23).

## Architecture

### Current Architecture

- The parser builds `ExprKind::Lambda { captures, params, body }` with typed
  parameters and a capture list (`Capture { mode, name, span }`).
- The checker reports `E90000` for a capture-less lambda and `E90002` for any
  capture list (`compiler/src/sema/expr.rs:81-98`).
- Codegen has no lambda case; the expression would fall through.
- `TyKind::Func(FuncTy)` already models an exact-signature C function pointer,
  and function-pointer types are duplicable (`compiler/src/types/mod.rs:236`).

### Proposed Changes

1. Type a lambda: capture-less → `TyKind::Func`; capturing →
   `TyKind::Closure { id, params, returns }`.
2. Check the capture rules and the ephemerality rules.
3. Lift, emit, and call the two-word form (Ch 17 §8, P7).

## Implementation Details

### New Types/Interfaces

```rust
/// A capturing closure's ephemeral type.
///
/// `id` is the stable index of the lambda site, used to name the lifted
/// function, the environment struct, and the monomorphization key.
pub struct ClosureTy {
    /// Stable lambda-site index.
    pub id: usize,
    /// Parameter types.
    pub params: Vec<Ty>,
    /// Return type.
    pub returns: Box<Ty>,
}
```

`TyKind` gains `Closure(ClosureTy)`. A closure type is not duplicable, is never
a value of an aggregate field, and is not droppable; it exists only as a
temporary (AR-23).

The semantic stage assigns `id` as a deterministic per-program counter over
lambda sites in source order, stores it on the `ClosureTy`, and threads it
through the IR, so `type_key` and the emitter name the same instance every run.
A lambda defined **inside** a generic body is not required by this plan and is
left out of scope, so a site id need not include the enclosing instance.

IR gains one value form, `ValueKind::Closure { site }`, and one call form,
`ValueKind::ClosureCall { callable, args }`, where the callable is a two-word
value; a `func(...)` call keeps the existing `ValueKind::Call`.

### New Functions/Methods

- `sema/expr.rs::check_lambda` — types the parameters and body in a child scope
  that binds the captures, then returns `Func` (capture-less) or `Closure`
  (capturing). It enforces `E10403` (a `take` capture), `E10401` (the body moves
  a capture), `E10402` (the body writes a capture without `[borrow x]`), and
  records the captures for emission.
- `sema` storage checks report `E10404` when a `Closure`-typed value is used as
  a `let`/`const` initializer, a struct field or array element value, a return
  value, or another lambda's capture initializer (AR-11, AR-14).
- `sema::check_conversion` reports `E10400` when a `Closure` value meets an
  expected `TyKind::Func` (a concrete `func(...)` type).
- `codegen` lifts each lambda to a file-scope `static` function
  `cl_lambda_<N>` (`N` the site id). A capture-less lambda's lifted function has
  the lambda's own signature; a capturing lambda's lifted function takes a
  leading environment pointer, and its site also emits an environment struct
  `ClEnv_<N>` with one field per capture.
- `codegen` emits a capture-less lambda value as the function name and a
  capturing lambda value as the two-word struct `{ cl_lambda_<N>, &env }`, where
  `env` is a synthesized local on the enclosing frame holding the captured
  values (by value for a duplicable copy capture, by pointer for a read-only or
  `[borrow]` capture). Because the closure is inline-only, `env` never outlives
  the enclosing frame (AR-11, AR-23).
- A call of a closure value emits `callable.fn(callable.env, args...)`; a call
  of a `func(...)` value is unchanged.
- In a monomorphized generic body, the call form follows the concrete type bound
  to the callable parameter: `Func` → a direct call; `Closure` → the two-word
  call (AR-9, AR-10). `type_key` for a closure is `closure_<N>`, so distinct
  closure sites dedupe to distinct instances (R9).

### Capture Rules

| Capture form | Type of capture | Value stored in the environment |
| ------------ | --------------- | ------------------------------- |
| `[x]`, `x` duplicable | a copy | the value |
| `[x]`, `x` non-duplicable | a read-only borrow | a pointer |
| `[borrow x]` | a writable borrow | a pointer |
| `[take x]` | rejected | — (`E10403`) |

Writing a captured binding without `[borrow x]` is `E10402`; moving a capture
is `E10401` (L2, L-3..L-5; AR-13).

### Integration Points

- The checker's scope handling already tracks locals and modes, so captures bind
  like ordinary locals.
- Monomorphization already keys instances by a rendered type; a closure instance
  adds the site key (AR-29).
- The emitter's function-pointer spelling (`codegen/types.rs`) covers the
  capture-less case unchanged.

## Code Examples

### Example 1: Capture-less lambda is a function pointer

```clorraine
func apply2(x: i32, f: func(i32): i32): i32 { return f(f(x)); }

func demo(): i32 {
    return apply2(3, (x: i32) => x + 1);   // a plain function pointer
}
```

### Example 2: Capturing closure passed inline to a callable bound

```clorraine
func apply<T, F: func(T): T>(x: T, f: F): T { return f(x); }

func demo(): i32 {
    let base: i32 = 10;
    // F is left to inference: a capturing closure is accepted only at the
    // callable-bound parameter. Writing func(i32): i32 for F makes the
    // argument a concrete func value, which is E10400 (see Example 3).
    return apply(1, [base] (x: i32) => x + base);
}
```

### Example 3: A capturing closure with an explicit `func(...)` type argument is E10400

```clorraine
func apply<T, F: func(T): T>(x: T, f: F): T { return f(x); }

func demo(): i32 {
    let base: i32 = 10;
    // The explicit type argument binds F to a concrete func value type, so a
    // capturing closure there does not match and is E10400.
    return apply<i32, func(i32): i32>(1, [base] (x: i32) => x + base);
}
```

### Example 4: A named capturing closure is E10404

```clorraine
func demo(): i32 {
    let base: i32 = 10;
    let f = [base] (x: i32) => x + base;   // E10404: closures are ephemeral
    return f(1);
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Capturing closure where a concrete `func(...)` is required | `E10400` at the conversion | AR-10 |
| Lambda moves a captured binding | `E10401` | AR-13 |
| Lambda writes a capture without `[borrow]` | `E10402` | AR-13 |
| `take` capture | `E10403` | AR-22 |
| Capturing closure bound to a name, stored, returned, or captured | `E10404` | AR-11, AR-14 |

> **Traceability:** every strategy cites the register entry that resolved it;
> see `00-ambiguity-register.md`.

## Testing Requirements

- Type unit tests for capture classification and the four diagnostics.
- Codegen unit tests for the lifted function, the environment struct, and the
  two-word call.
- Specification tests per the lambda ST cases in `07-testing-strategy.md`,
  including the five un-skipped compile-fail goldens.
