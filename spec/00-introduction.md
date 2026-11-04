# Chapter 00 - Introduction and Axioms

> **Status:** stable (v1.0)
> **Depends on:** nothing; this chapter is the front door
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md)
> **Ledger:** [decisions.md](decisions.md)

## 1. What Clorraine is

Clorraine is a statically typed systems language for the Motorola 68000 family.
It looks like C, is checked like Rust, and compiles to plain C:

* **C-like surface syntax** - braces, semicolons, `name: type`, `func`,
  `module`, `export`/`import`, and the C operator table.
* **Ownership and borrow checking** - memory and OS resources are released
  deterministically. Use after release, double release, and dangling borrows
  are compile-time errors. There is no garbage collector and no reference
  counting.
* **Transpiles to C** - the output is a strict C99 common subset accepted by
  both GCC (`m68k-amigaos-gcc`) and vbcc, so the language inherits their
  backends, debuggers, and the whole Amiga ecosystem. The emitted C is the
  codegen contract in [Ch 17](17-codegen.md).
* **Built for the Amiga first** - the standard library is the AmigaOS API
  behind reviewed ownership contracts. LorraineOS is binary compatible with
  AmigaOS, so the same programs run on both.

The specification is written for a developer who knows C and wants to learn
Clorraine without unlearning habits. Where a rule is not stated, the pinned
Rust semantics apply ([lineage.md](lineage.md) section 9).

## 2. Design axioms

These are givens; every feature must respect them.

### A1 - C-like syntax

A developer fluent in C should read Clorraine without a manual. No new sigil
vocabulary where keywords or existing C notation can carry the meaning. The
one deliberate addition is `$"..."` for string templates.

### A2 - Deterministic release, statically checked

Every owned value is released exactly once, at a statically known point, with
a statically known release function. There is no garbage collector and no
reference counting in the core language. [Ch 03](03-ownership.md) defines the
model.

### A3 - No undefined behavior

Every input either compiles to a defined result or is a compile-time error.
Arithmetic wraps, shifts are defined and masked, division by zero is a
deterministic fault, and the code generator never relies on C undefined
behavior (no signed overflow, no strict-aliasing violations, no unaligned
access). [Ch 17](17-codegen.md) section 14 lists the commitments.

### A4 - Explicit over implicit

Declarations carry types; ownership modes are visible in signatures; there are
no hidden conversions or hidden allocations. Where the compiler can infer
something safely (a local's type from its initializer, a view's lifetime from
one lend parameter), it does, but the contract stays readable.

### A5 - Target-neutral core, profiles for facts

The core language never mentions a chip, an address, or a library. Platform
facts (toolchain flags, runtime, API contracts, package format) live in
profiles ([Ch 15](15-profiles.md)). v1 ships the `hosted-amiga` profile;
`host` is the test profile; `freestanding` is implemented provisionally (m68k,
no libc) for LorraineOS to adopt.

### A6 - The C common subset is the only backend

The compiler emits C99 that both GCC and vbcc accept. No GCC extensions, no
`__attribute__((cleanup))`, no VLAs, no compiler-specific tricks. The codegen
contract is normative.

### A7 - Safety checks are always on

Memory safety is strong above the hardware boundary: ownership, borrows, and
nullability are proved statically. The remaining checks - array bounds,
integer division, and float-to-integer range - are always emitted and fault
deterministically on the profile's runtime (T12, B7). There is no release mode
that removes them, and there is no unwinding: `panic` and the fault checks
abort. Below the boundary (MMIO, interrupts, raw memory), access is audited
`unsafe`, and the standard-library wrappers shrink that surface over time.

### A8 - Co-designed with LorraineOS

Clorraine exists to write LorraineOS and Amiga programs. Binary compatibility
with AmigaOS is the v1 integration path. Target facts live in profiles, so
supporting another machine would be a profile addition, not a redesign; no
such target (for example ARM64) is planned for v1.

## 3. What Clorraine deliberately is not

| Not adopted | Why |
|---|---|
| Traits, `impl` blocks, inheritance | Module functions plus exact-signature function values cover v1 needs |
| Closures with heap environments | Lambdas are implemented (L1-L4); a capturing lambda uses a stack environment, never a heap allocation. C-hook trampolines remain staged (L5). |
| `Result`/`Maybe` and `?` propagation | Three error tiers instead: `panic`, `T?`, and status structs (E1-E6) |
| Exceptions and unwinding | Deterministic release on m68k; `panic` aborts (D4, E2) |
| Macros and a preprocessor | Modules replace includes; `comptime` is the compile-time path and is staged |
| Tagged unions and `match` | C-style tagged structs plus C-exact `switch` are the data model (T10) |
| Garbage collection, reference counting | A2 |
| Lifetime annotation syntax | Elision only; ambiguous cases are errors (D2) |
| Interior mutability, `Rc`/`Arc`, threads | Single-threaded language; tasks are the OS model behind FFI (C1, D6, D7) |
| Package manager | Modules plus the SDK image; no registry in v1 |
| `goto` and labels | The compiler emits `goto` for cleanup ladders; source code does not write it |

Generics **are** in v1 (G1-G8): `<T>` on functions and structs, monomorphized,
with callable-only bounds. Floats (`f32`/`f64`) are in v1. Both are additions
to the original plan, recorded in the ledger.

## 4. Error handling in one paragraph

Bugs, broken invariants, and out-of-memory conditions abort with `panic`.
Recoverable failures without detail use `T?` and a `!= null` check.
Recoverable failures with detail use a status struct or a `borrow` out
parameter. There is no propagation sugar, and there never will be: explicit
checks are the design. The full rules are in [Ch 03](03-ownership.md) and
[Ch 09](09-enums-unions.md).

## 5. Profiles at a glance

| Profile | Purpose | Runtime | Status |
|---|---|---|---|
| `hosted-amiga` | AmigaOS 2.0+/3.x programs, v1 flagship | libnix `-mcrt=nix20`, `-m68000` default | v1 |
| `hosted-amiga-13` | A500/Kickstart 1.3 baseline | libnix `-mcrt=nix13`, 68000, OCS, 512 KB | v1.1 |
| `host` | Fast tests of emitted C on the development machine | small shim | v1 |
| `freestanding` | LorraineOS kernel and native programs | self-contained no-libc shim with `_start` | implemented (provisional, m68k) |

## 6. How to read this specification

| File | What it is |
|---|---|
| [decisions.md](decisions.md) | The decision ledger: every locked decision, authoritative when a chapter disagrees |
| [grammar.ebnf.md](grammar.ebnf.md) | The frozen surface grammar (ISO 14977 EBNF) |
| [grammar-deltas.md](grammar-deltas.md) | Every delta from Blend65 v4, with reasons |
| [lineage.md](lineage.md) | The pinned Rust semantic reference and the deliberate deviations |
| Chapters 00-17 | The language itself, in grammar order |
| [appendix-amiga.md](appendix-amiga.md) | Amiga platform notes and the v1 API practice |

Each chapter opens with its dependencies, the grammar sections it maps to, and
the ledger decisions it implements. Diagnostics are referenced by code; the
registry and message templates live in [Ch 14](14-diagnostics.md).

## 7. Vocabulary

| Term | Meaning |
|---|---|
| value | A piece of data: scalar, aggregate, view, or handle |
| owned | The binding is responsible for releasing the value |
| duplicable | A value that copies freely: scalars, enums, views, and structs whose fields are all duplicable |
| lend | Access without ownership: read-only (unmarked or `const`) or writable (`borrow`) |
| move | Transfer of ownership through a `take` parameter, a return, or a move-only binding; the source becomes invalid |
| drop | The inserted release of an owned value: null-safe, exactly once |
| handle | An opaque owned type whose release function is named by `freedBy` |
| deleter | The `take T`-to-`void` function that releases a handle |
| view | A non-owning window: `T[]`, `str`, or `cstring` |
| overlay | A struct with a `volatile` field, accessed only through `raw` pointers |
| profile | Target-specific facts: toolchain, runtime, API surface, packaging |
