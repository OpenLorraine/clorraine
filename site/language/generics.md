---
title: Generics
description: "Type parameters, callable bounds, inference, and monomorphized instances in Clorraine."
---

# Generics

Clorraine generics are compile-time type parameters. Each instantiation is
monomorphized into concrete code with a hash-deduped symbol. There is no runtime
type dictionary and no boxing requirement.

## Type parameters

Write parameters after the function name:

```clorraine
func identity<T>(x: T): T {
    return x;
}
```

Call with an explicit type or let the arguments infer it:

```clorraine
let a: i32 = identity<i32>(5);
let b: i32 = identity(7);
```

Inference is arguments-only. A constructor with no value argument needs explicit
type arguments, for example `arrayNew<i32>()`.

## Callable bounds

The only bound is callable: `F: func(T): R`. A value bound this way can be
called or passed, but not stored or returned.

```clorraine
func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);
}

func addOne(x: i32): i32 {
    return x + 1;
}
```

```clorraine
let c: i32 = apply(5, addOne);
```

## A complete example

```clorraine
module demo.Generics;

import { println } from std.io;

func identity<T>(x: T): T {
    return x;
}

func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);
}

func addOne(x: i32): i32 {
    return x + 1;
}

func main(): i32 {
    let a: i32 = identity<i32>(5);
    let b: i32 = identity(7);
    let c: i32 = apply(a, addOne);
    println($"a={a} b={b} c={c}");
    return 0;
}
```

Output:

```text
a=5 b=7 c=6
```

## Rules to remember

| Rule | Consequence |
|---|---|
| Inference is arguments-only | Supply explicit type arguments when there is no value to infer from |
| The only bound is callable | A generic body records the copy obligations it needs and checks them at each instantiation |
| A generic function cannot be `export`ed | Write a concrete wrapper for the public API |
| Instantiation recursion is capped | A recursive generic that never reaches a concrete type is rejected |
| Generic aggregates reject ephemeral type arguments | A closure type that cannot be stored cannot parameterize a stored aggregate |

## Why monomorphization

A generic call becomes a concrete C function. The generated C has no dispatch
overhead and no hidden allocation. The trade-off is code size: each distinct
type argument set produces an instance, deduplicated by the compiler.

## Learn more

- [Chapter 06 — Functions, Generics, and Lambdas](https://github.com/OpenLorraine/clorraine/blob/main/spec/06-functions.md)
- [Chapter 17 — Code Generation](https://github.com/OpenLorraine/clorraine/blob/main/spec/17-codegen.md)
