---
title: Types and modules
description: "Clorraine's scalar types, strings, arrays, slices, structs, enums, handles, and module system."
---

# Types and modules

Clorraine keeps C's machine model and makes the distinctions that C leaves to
convention explicit in the type.

## Scalars

| Type | Meaning |
|---|---|
| `i8`, `i16`, `i32`, `i64` | Signed integers |
| `u8`, `u16`, `u32`, `u64` | Unsigned integers |
| `isize`, `usize` | Pointer-sized signed and unsigned integers |
| `f32`, `f64` | IEEE floating point |
| `bool` | `true` or `false` |
| `void` | No value |

Integers have a fixed width on every profile. There is no plain `int` whose
width depends on the target.

## Strings

| Spelling | Meaning |
|---|---|
| `str` | A borrowed byte view, not NUL-terminated |
| `String` | An owned byte string, released by its deleter |
| `cstring` | A borrowed NUL-terminated view for an `ffi` call |

A `String` does not implicitly become a `cstring`. The NUL-terminated form is a
promise to the C side, so the conversion is explicit.

## Arrays and slices

A fixed array has a compile-time extent: `T[N]`. It lives where its binding
lives and never decays to a pointer. A slice `T[]` is a view: a pointer and a
length, borrowing storage owned elsewhere.

```clorraine
let primes: u32[4] = [2, 3, 5, 7];
let zeros: u8[8] = [; 0];

let values: u32[3] = [10, 20, 30];
let view: u32[] = values;
```

Indexing is bounds-checked on every access. `len(xs)` is the element count:
compile-time for a fixed array, runtime for a slice and `str`.

## Structs and enums

Structs have named fields in C layout order. Fields are semicolon-terminated:

```clorraine
struct Point {
    x: i32;
    y: i32;
}

let p: Point = Point { x: 3, y: 4 };
```

Enums require an integer backing type. Members are scoped and converted with an
explicit cast:

```clorraine
enum Kind : u8 {
    Word,
    Number,
}

let kind: Kind = Kind.Number;
let raw: u8 = u8(kind);
```

There is no tagged `union` and no `match`. The supported model is a struct with
a tag field plus a `switch`.

## Handles, optionals, and raw

| Form | Meaning |
|---|---|
| `T?` | An optional value or handle; `null` is explicit |
| `handle T freedBy F;` | A resource handle with a named deleter |
| `Box<T>` | An owning pointer for recursive or heap values |
| `raw T` | A raw pointer, used at the hardware or C boundary |

A handle is move-only. A `Box<T>` is move-only and cannot be null; use
`Box<T>?` for an optional box.

## A complete example

```clorraine
module demo.Types;

import { String, stringOf } from std.text;
import { println } from std.io;

struct Point {
    x: i32;
    y: i32;
}

enum Kind : u8 {
    Word,
    Number,
}

func main(): i32 {
    let p: Point = Point { x: 3, y: 4 };
    let words: str[3] = ["one", "two", "three"];
    let owned: String = stringOf(words[1]);
    let view: str = owned;
    println(view);
    println($"point={p.x},{p.y}");
    println($"kind={u8(Kind.Number)}");
    return 0;
}
```

Output:

```text
two
point=3,4
kind=1
```

## Modules

A module is a namespace and an ownership boundary. The module name is declared
at the top of the file, and imports use the standard library or another module:

```clorraine
module demo.Greeting;

import { println } from std.io;
import { String, stringOf } from std.text;
```

`export` makes a declaration visible to other modules. The default is
module-private. Application code imports standard-library modules; `ffi` blocks
live inside the library modules that own the C boundary.

## Learn more

- [Chapter 02 — Types](https://github.com/OpenLorraine/clorraine/blob/main/spec/02-types.md)
- [Chapter 07 — Structs](https://github.com/OpenLorraine/clorraine/blob/main/spec/07-structs.md)
- [Chapter 08 — Arrays, Slices, and Strings](https://github.com/OpenLorraine/clorraine/blob/main/spec/08-arrays-strings.md)
- [Chapter 09 — Enums and Tagged Data](https://github.com/OpenLorraine/clorraine/blob/main/spec/09-enums-unions.md)
- [Chapter 10 — Modules](https://github.com/OpenLorraine/clorraine/blob/main/spec/10-modules.md)
