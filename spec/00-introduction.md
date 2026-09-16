# Chapter 00 - Introduction and Axioms

> **Status:** draft
> **Depends on:** nothing

## 1. What Clorraine is

Clorraine is a statically typed systems language for the Motorola 68000 family
(and, by design, later ARM64). It looks like C, is checked like Rust, and
compiles to plain C:

* **C-like surface syntax** - braces, semicolons, `name: type`, `function`,
  `module`, `export`/`import`.
* **Ownership and borrow checking** - memory and OS resources are released
  deterministically; use-after-release, double-release, and dangling borrows
  are compile-time errors, with no garbage collector and no reference counts.
* **Transpiles to C** - output is a strict C99 common subset accepted by both
  GCC (`m68k-amigaos-gcc`) and vbcc, so the language inherits their backends,
  debuggers, and the entire Amiga ecosystem.
* **Built for the Amiga first** - the standard library is the AmigaOS API
  behind ownership contracts; LorraineOS is binary compatible with AmigaOS, so
  the same programs run on both.

## 2. Design axioms

These are givens; every feature must respect them.

### A1 - C-like syntax

A developer fluent in C should read Clorraine without a manual. No new sigil
vocabulary where keywords or existing C notation can carry the meaning.

### A2 - Deterministic release, statically checked

Every owned value is released exactly once, at a statically known point, with a
statically known release function. There is no garbage collector and no
reference counting in the core language.

### A3 - No undefined behavior

Every input either compiles to a defined result or is a compile-time error.
Arithmetic wraps, shifts are defined, and the code generator must not rely on
C undefined behavior (no signed overflow, no strict-aliasing violations, no
unaligned access).

### A4 - Explicit over implicit

Declarations carry types; ownership modes are visible in signatures; no hidden
conversions. Where the compiler can infer something safely (a local lifetime,
a temporary move), it does - but the contract stays readable.

### A5 - Target-neutral core, profiles for facts

The core language never mentions a chip, address, or library. Platform facts
(toolchain flags, runtime, API contracts, package format) live in profiles
(Ch 15). v1 ships the `hosted-amiga` profile; `host` is the test profile;
`freestanding` is designed for LorraineOS and implemented later.

### A6 - The C common subset is the only backend

The compiler emits C99 that both GCC and vbcc accept. No GCC extensions, no
`__attribute__((cleanup))`, no VLAs, no compiler-specific tricks. The codegen
contract (Ch 17) is normative.

### A7 - Safety without a runtime framework

The only runtime is a small shim (acquire/drop helpers, allocator hooks,
panic). The borrow checker proves the properties; nothing is checked at run
time in release builds.

### A8 - Co-designed with LorraineOS

Clorraine exists to write LorraineOS and Amiga programs. Binary compatibility
with AmigaOS is the v1 integration path; source portability (pointer-size,
endianness) keeps a future ARM64 port a profile change, not a redesign.

## 3. What Clorraine deliberately is not

| Not adopted | Why |
|---|---|
| Traits, `impl` blocks | Module functions plus exact-signature function values cover v1 needs |
| Closures | Callbacks use function values; no captured environments |
| Generics | Deferred; containers come later, concrete or monomorphized |
| `Result`/`?` error plumbing | C-like error returns and enums; no exceptions |
| Macros (source-level) | The language has none; `comptime` functions generate code instead (later phase) |
| Garbage collection / reference counting | A2 |
| Lifetime annotations everywhere | v1 elides; explicit lifetime parameters only if needed later |
| Package manager | Modules plus the SDK image; no registry in v1 |

## 4. Profiles at a glance

| Profile | Purpose | Runtime | Status |
|---|---|---|---|
| `hosted-amiga` | AmigaOS 2.0+/3.x programs, v1 flagship | libnix `-mcrt=nix20`, `-m68000` default | v1 |
| `host` | Fast tests of emitted C on the development machine | small shim | v1 |
| `freestanding` | LorraineOS kernel and native programs | defined by LorraineOS | designed, later |

## 5. Vocabulary

| Term | Meaning |
|---|---|
| value | A piece of data: scalar, aggregate, or handle |
| owned | The binding is responsible for releasing the value |
| copy | A value that may be duplicated freely (scalars, plain data) |
| shared borrow | Read-only access without ownership |
| exclusive borrow | Unique access without ownership; no other live borrow |
| move | Transfer of ownership; the source becomes invalid |
| drop | Inserted release of an owned value (null-safe, exactly once) |
| resource | Anything with a release function: memory, libraries, files, ports |
| contract | Declared ownership behavior of an external (FFI) function |
| profile | Target-specific facts: toolchain, runtime, API surface, packaging |
