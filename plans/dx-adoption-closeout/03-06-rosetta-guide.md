# Rosetta Guide: DX and Adoption Closeout

> **Document**: 03-06-rosetta-guide.md
> **Parent**: [Index](00-index.md)

## Overview

A translation guide for a programmer who knows Amiga C and wants to write the
same program in Clorraine. The guide explains the ownership shift and the
diagnostics that follow from it. It is documentation only: no code, no build
target, no specification change. Requirements: R13, AR-33.

## Architecture

### Current Architecture

The frozen specification is the normative reference. `docs/` holds findings and
review records but has no index and no translation guide. `README.md` links
only `docs/known-limitations.md`.

### Proposed Changes

- `docs/README.md`: a short documentation index.
- `docs/c-to-clorraine.md`: the guide.
- `README.md`: an "Editor and tooling" section plus a guide link.

## Content Specification

The guide uses this order. Every section has a C example, the Clorraine
equivalent, and a short "what changed" paragraph. Snippets are excerpts, not
compilable programs, and say so once at the top.

### 1. Mental model

| C habit | Clorraine rule |
| ------- | -------------- |
| A pointer may be null, borrowed, or owned; the type does not say | `T`, `T?`, `borrow T`, `take T`, and views make the mode explicit |
| `free` is called by convention | The owner drops once at scope end; a `take` parameter transfers the duty |
| A struct copy is a byte copy | A value copy is allowed only when the type is duplicable; owning types move |
| A cast reinterprets | `raw T` is explicit and `unsafe` at the hardware boundary |

Link: `spec/03-ownership.md`, `spec/11-memory.md`.

### 2. Allocation

C:

```c
struct Buffer *b = AllocMem(sizeof *b, MEMF_CLEAR);
if (b == NULL) { return ENOMEM; }
...
FreeMem(b, sizeof *b);
```

Clorraine:

```clorraine
let b: Box<Buffer> = boxNew(Buffer { ... });
// b drops here
```

Also show the raw pattern for a caller-owned handle, a `freedBy` deleter, and
a `take` parameter. Explain `Box<T>` non-nullability and `Box<T>?` for an
optional allocation.

### 3. Files

C `Open`/`Read`/`Write`/`Close` versus the `std.amiga.dos` handle functions.
Show the `Buffer` ownership, the `borrow` read, and the error path. Include
the no-exceptions status style and the `E1` trap distinction.

### 4. Strings

C `char *` and `strlen` versus `str` (borrowed bytes), `String` (owned), and
`cstring` (NUL-terminated FFI view). Show:

- a `str` parameter as `borrow str`;
- building an owned `String`;
- passing a `cstring` to an `ffi` call;
- why a `String` does not implicitly become a `cstring` at every boundary.

### 5. MMIO

C volatile pointer arithmetic versus the per-chip wrappers. Show the
`const raw` base, a volatile register store/read, and why the wrapper is
preferred over a hand-written address. Link `spec/15-profiles.md` and the
`std.amiga.*` module docs.

### 6. Interrupts

C server/client chain versus the `std.amiga.interrupt` surface. Show the
handler signature, the allocation-free rule, and the ownership duty of the
registered argument. State that a capturing callback is rejected at a storing
registration API.

### 7. Hooks

C `Hook` and `CallHookA` versus a Clorraine `ffi` hook. Show a synchronous
capturing callback (the L5 trampoline) and a stored callback (capture-less
only). Explain `E10400` and `E11215`.

### 8. Ownership modes

A decision table:

| Mode | Use when | Caller after the call |
| ---- | -------- | --------------------- |
| `T` (by value) | The callee needs its own copy; `T` is duplicable | Still owns its copy |
| `borrow T` | Read-only use for the call | Still owns |
| `take T` | The callee becomes the owner | Must not use |
| `borrow T` on a view | The callee may write through a view where allowed | Still owns |
| `handle` | An opaque resource with a `freedBy` deleter | Moves when passed by value |
| `raw T` | Hardware or an uncontracted C boundary | Unchanged; `unsafe` |

### 9. Error handling

C `LONG`/`BPTR` sentinel returns versus:

- a nullable return and `if (x != null)` narrowing;
- a status struct with a `bool`/enum code;
- `panic`/trap for a broken invariant;
- partial I/O: return the count and the status separately;
- why there are no exceptions and no hidden error global.

### 10. Diagnostics a C programmer will meet

| Code | C habit | Fix |
| ---- | ------- | --- |
| `E10010` | Using a nullable pointer without a check | Narrow with `if (p != null)` |
| `E10012` | Initializing a non-null type from `null` | Use `T?` or fix the initializer |
| `E10020` | Copying a non-duplicable value | Move it or pass `borrow` |
| `E10026` | Moving out of a borrowed view | Borrow, copy the element, or take ownership |
| `E10034` | Releasing or storing a borrowed parameter | Declare `take` or by value |
| `E10105` | Passing a generic/Box value across FFI | Use the box/handle API at the boundary |
| `E10201` | A C-style cast between unrelated types | Use the declared conversion or `raw` |
| `E10207` | Storing an overlay outside its scope | Keep the overlay local or copy |
| `E10306` | Storing an ephemeral closure | Use a plain function or a synchronous hook |
| `E10400` | A capturing lambda at a storing C API | Make it capture-less or use a synchronous hook |
| `E11200` | An Amiga module on the wrong profile | Select `hosted-amiga` |
| `E11215` | A stored hook callback invoked after return | Do not store the callback; keep the call synchronous |

Each row links the registry entry with
`clorraine --explain <code>` in prose (not as a build step).

## Constraints

- No new specification chapter, no copied normative text.
- Every code block is an excerpt; the guide says so once.
- No build target and no test that compiles the examples.
- Links resolve from `docs/`; `make check` only validates `spec/`, so a new
  lightweight link check in the documentation test suite covers the guide.
- British/international English, short sentences, no idioms.

## Testing Requirements

- A documentation test reads the guide and asserts every required heading and
  every listed diagnostic code is present.
- A link test resolves every relative link from `docs/`.
- `README.md` and `docs/README.md` link the guide.
