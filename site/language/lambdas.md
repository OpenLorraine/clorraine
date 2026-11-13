---
title: Lambdas and closures
description: "Capture-less function pointers, inline capturing closures, capture lists, and synchronous C hooks in Clorraine."
---

# Lambdas and closures

Clorraine has two lambda forms with different powers:

| Form | Shape | Can be stored? |
|---|---|---|
| **Capture-less lambda** | `(x: i32) => x + 1` | Yes; it is a plain function pointer |
| **Capturing closure** | `[borrow total] (n: i32) => { ... }` | No; it is ephemeral and inline-only |

A capture-less lambda matches `func(...)`. A capturing closure used where a
plain function pointer is required is `E10400`.

## Capture-less lambdas

```clorraine
let addOne: func(i32): i32 = (x: i32) => x + 1;
let result: i32 = addOne(4);
```

The value is an ordinary function pointer with no environment.

## Capturing closures

A capture list makes the environment explicit:

```clorraine
let total: i32 = 10;
([borrow total] (n: i32) => { total = total + n; })(5);
```

- Writing a captured binding requires `[borrow name]`.
- A duplicable capture is copied; a non-duplicable capture is borrowed
  read-only.
- A `take` capture is rejected (`E10403`).
- A capturing closure cannot be returned, stored, or captured again
  (`E10404`).

A complete example:

```clorraine
module demo.Lambdas;

import { println } from std.io;

func main(): i32 {
    let addOne: func(i32): i32 = (x: i32) => x + 1;
    let base: i32 = 10;
    let total: i32 = addOne(base);
    ([borrow total] (n: i32) => { total = total + n; })(5);
    println($"total={total}");
    return 0;
}
```

Output:

```text
total=16
```

## C hooks

A capturing closure may cross a synchronous `borrow`-mode callback on an `ffi`
declaration. The compiler generates a trampoline and a caller-stack
environment. The callback signature must be ABI-identical to the lifted lambda.

The environment must still be live when the C hook calls back. Invoking a
capturing callback after its hook returned faults (`E11215`). A capture-less
callback has no environment and no such check.

## Why the limits

A capturing closure owns or borrows part of its caller's stack. Storing it would
allow it to outlive that stack, so the language rejects the escape instead of
leaving a dangling environment. The restrictions are the price of a closure that
needs no allocation and no reference counting.

## Learn more

- [Chapter 06 — Functions, Generics, and Lambdas](https://github.com/OpenLorraine/clorraine/blob/main/spec/06-functions.md)
- [Chapter 12 — FFI and Contracts](https://github.com/OpenLorraine/clorraine/blob/main/spec/12-ffi-contracts.md)
- [Chapter 16 — Borrow Rules](https://github.com/OpenLorraine/clorraine/blob/main/spec/16-borrow-rules.md)
