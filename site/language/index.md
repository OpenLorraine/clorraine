---
title: The language
description: "A tour of Clorraine: C-like syntax, Rust-style ownership, generics, closures, hardware access, and four target profiles."
---

# The language

Clorraine is C-like on the surface and strict underneath. It keeps the machine
model Amiga programmers already know — fixed-width integers, explicit layout,
pointers at the hardware boundary, one C translation unit — and changes who owns
a value and when it is released.

## Design principles

| Principle | Consequence |
|---|---|
| **One owner per value** | Moves, borrows, and drops are checked; there is no garbage collector |
| **Plain C99 output** | The generated C is inspectable and builds with the toolchains you already use |
| **Amiga-first standard library** | Graphics, audio, Intuition, Exec, DOS, layers, text, ILBM, and RTG ship with the compiler |
| **Profiles, not guesses** | Target facts live in a profile; a construct the target cannot lower is rejected |
| **A frozen specification** | The language, grammar, and diagnostics are versioned and tested |
| **Explicit boundaries** | `ffi`, `raw`, and `volatile` mark the places where safety is handed to C or hardware |

## A small complete program

```clorraine
module demo.Hello;

import { println } from std.io;

func main(): i32 {
    let who: str = "Amiga";
    println($"Hello, {who}, from Clorraine!");
    return 0;
}
```

That program is real: it compiles to C99, runs on the host profile, and runs on
AmigaOS through the same source.

## The tour

| Page | What it covers |
|---|---|
| [Ownership](/language/ownership) | Moves, borrows, drops, parameter modes, and the move-then-use diagnostic |
| [Types and modules](/language/types) | Scalars, strings, arrays, slices, structs, enums, handles, and imports |
| [Generics](/language/generics) | Type parameters, inference, callable bounds, and monomorphization |
| [Lambdas and closures](/language/lambdas) | Function pointers, capturing closures, capture lists, and C hooks |
| [Volatile and MMIO](/language/mmio) | Overlay types, volatile fields, raw access, and per-chip wrappers |
| [Profiles](/language/profiles) | `host`, `hosted-amiga`, `hosted-amiga-13`, and `freestanding` |
| [From C to Clorraine](/language/from-c) | The mental model for an Amiga C programmer, with side-by-side patterns |

## What the compiler does not do

- It does not hide the C boundary. `ffi`, `raw`, and `unsafe` are explicit.
- It does not use a tracing collector or an ownership runtime. The generated C
  contains a small platform shim, not a garbage collector.
- It does not infer a target from the host machine. You select a profile.
- It does not promise that every AmigaOS API is covered. The SDK tracks 81
  capabilities, each with an acceptance gate.

The frozen specification is the authority for every rule. Start with
[the specification overview](https://github.com/OpenLorraine/clorraine/blob/main/spec/README.md),
or follow the tour and open a chapter when you need the exact wording.
