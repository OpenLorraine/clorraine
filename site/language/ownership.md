---
title: Ownership
description: "How Clorraine moves, borrows, and drops values, and what the compiler rejects before the C compiler sees it."
---

# Ownership

Clorraine gives every owned value one owner. The owner releases it once, at a
well-defined point. Borrows grant temporary access without taking
responsibility. The compiler proves the rules before it emits C.

There is no garbage collector and no tracing runtime.

## Kinds of value

| Kind | Examples | Behaviour |
|---|---|---|
| Scalar | `i32`, `u32`, `f64`, `bool` | Duplicable; a copy is a new value |
| Aggregate | struct, fixed array | Duplicable when every field or element is duplicable |
| View | `str`, `T[]`, `cstring` | Duplicable; a copy is a pointer and length |
| Handle | `File`, `Buffer`, `String` | Move-only; released by its `freedBy` deleter |
| Box | `Box<T>` | Move-only; released by generated recursive drop glue |

A **move** transfers ownership. After a move, the source binding is invalid: a
read, borrow, second move, or drop is rejected. Re-initializing the binding makes
it live again.

## Parameter modes

| Mode | Meaning | May write | Ownership |
|---|---|---|---|
| `x: T` | Read-only lend | No | Caller keeps it |
| `const x: T` | Read-only lend | No | Caller keeps it |
| `borrow x: T` | Writable lend | Yes | Caller keeps it |
| `take x: T` | Ownership transfer | Yes | Callee owns it |

Call-site markers are optional assertions. If written, they must agree with the
declaration.

## The move-then-use diagnostic

This complete program is rejected:

```clorraine
module demo.OwnershipFail;

func observe(s: String): void {
}

func main(): i32 {
    let n: u32 = 1;
    let s: String = $"n={n}";
    let t: String = s;      // move: s is invalid from here
    observe(s);             // use after move
    observe(t);
    return 0;
}
```

The compiler reports:

```text
error E10020: 's' was moved and cannot be used
  --> examples/ownership-fail.clo:12:13
  help: re-initialize the binding or use a view
```

The C compiler never sees this program. The source file is
[`examples/ownership-fail.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/ownership-fail.clo),
and the case is pinned in the compile-fail suite.

## Drops

A local owner is released at the end of its scope. The compiler inserts the
release once, including on early returns:

```clorraine
module demo.Drop;

import { String, stringOf } from std.text;
import { println } from std.io;

func main(): i32 {
    let message: String = stringOf("released at scope end");
    let view: str = message;
    println(view);
    return 0;
}
```

A moved value is not dropped by the source scope. The destination owns it. The
compiler tracks move state through branches and loops, so a value moved on one
path is not treated as live on another.

## Borrows

A `borrow` parameter receives writable access without ownership:

```clorraine
func fill(borrow buffer: Buffer, value: u8): void {
    let bytes: u8[] = bufferBytes(buffer);
    let i: usize = 0;
    while (i < len(bytes)) {
        bytes[i] = value;
        i = i + 1;
    }
}
```

The caller keeps the buffer and releases it. A writable lend is exclusive: the
compiler rejects overlapping writable access that could invalidate the borrow.

## Safety at the boundary

The ownership rules stop at an explicit boundary:

- `ffi` declares the C ABI and its parameter modes.
- `raw` names a raw pointer type.
- `unsafe` marks the block where raw access happens.

Application code calls library wrappers rather than writing `ffi` blocks. That
keeps the unsafe surface inside modules that can be reviewed and tested.

## Learn more

- [Chapter 03 — Ownership, Moves, and Destructors](https://github.com/OpenLorraine/clorraine/blob/main/spec/03-ownership.md)
- [Chapter 16 — Borrow Rules](https://github.com/OpenLorraine/clorraine/blob/main/spec/16-borrow-rules.md)
- [Chapter 12 — FFI and Contracts](https://github.com/OpenLorraine/clorraine/blob/main/spec/12-ffi-contracts.md)
