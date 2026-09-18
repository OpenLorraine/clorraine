# Standard Library: Clorraine Compiler MVP

> **Document**: 03-07-stdlib.md
> **Parent**: [Index](00-index.md)

## Overview

The standard-library slice is ordinary Clorraine (X6,
`appendix-stdlib.md:9-11`) under top-level `stdlib/`, embedded in `clc` and
compiled with every program (AR-19, AR-25). It binds the per-profile C shim
through `ffi` blocks; application code never writes `ffi` and never touches
`unsafe` (X6, M9). This document owns the MVP module list, the plan-local
signatures that the normative appendix leaves open, the compiler-known
intrinsic helpers (AR-46, AR-50, AR-51), and the shim C API (AR-19).

## Modules

| Module | File | Provides |
|---|---|---|
| `std.mem` | `stdlib/std.mem.clor` | `Buffer`, `Array<T>`, `Span`, `Status`, allocation and slice helpers |
| `std.text` | `stdlib/std.text.clor` | `String`, `stringOf`, `clone`, `bytes`, `toCString` |
| `std.fmt` | `stdlib/std.fmt.clor` | `text`, `toHex`, `toDecimal`, `padLeft`, `padRight` |
| `std.io` | `stdlib/std.io.clor` | `File`, `openFile`, `createFile`, `readFile`, `writeBytes`, `print`, `println`, mode constants |
| `std.amiga.dos` | `stdlib/std.amiga.dos.clor` | `delayTicks` (hosted-amiga only) |
| `std.amiga.interrupt` | `stdlib/std.amiga.interrupt.clor` | `HandlerToken`, `addHandler`, `removeHandler` (hosted-amiga only) |

Declarations use lowercase dotted names (`module std.mem;`), matching every
import example in the specification (AR-52). The appendix's `module Std.Mem;`
line is a known editorial defect; correcting it in the spec requires a ruling
because the spec is frozen (AR-31).

## Compiler-Known Intrinsic Helpers (AR-46, AR-51)

Declared as `intrinsic func` (GF23) inside the stdlib; the compiler implements
them and rejects user declarations of these names:

| Helper | Signature | Semantics |
|---|---|---|
| `rawSlice<T>` | `(p: raw T, n: usize): T[]` | unchecked view over raw storage; used inside `unsafe` by the stdlib only |
| `rawStr` | `(p: raw u8, n: usize): str` | unchecked `str` view over raw bytes; stdlib only |
| `slice<T>` | `(v: T[], lo: usize, hi: usize): T[]` | sub-view into the same storage, no allocation, bounds guard E11211 (IX-1) |
| `len` | `(v) : usize` | fixed array extent, view length, or `Array<T>.count` (AR-50) |

`sizeof`/`offsetof`/volatile/`panic`/`assert` are the language intrinsics of
grammar §7. `embed` is out of scope (E90000, AR-10).

## `std.mem`

Normative pieces: `Buffer` and its `FreeBuffer` deleter, `Span`, `Status`
(`appendix-stdlib.md` §2, §4, §8). The MVP signatures:

```clorraine
module std.mem;

export handle Buffer freedBy FreeBuffer;
export struct Status { ok: bool; code: u32; }
export struct Span   { start: usize; len: usize; }

export func bufferNew(size: usize): Buffer?;          // allocator: nullable
export func bufferFrom(src: u8[]): Buffer?;           // allocator: nullable
export func bufferBytes(borrow b: Buffer): u8[];      // exclusive view
export func bufferLen(b: Buffer): usize;

export func spanOf(src: u8[], at: usize, n: usize): Span;   // E11211 via slice
export func spanBytes(src: u8[], s: Span): u8[];            // slice(src, start, start+len)

export struct Array<T> { data: Buffer; count: usize; }      // move-only
export func arrayNew<T>(): Array<T>;                        // empty, no allocation
export func push<T>(borrow xs: Array<T>, take v: T): void;
export func get<T>(borrow xs: Array<T>, i: usize): T[];     // one-element view, E11211
export func set<T>(borrow xs: Array<T>, i: usize, take v: T): void;
export func reserve<T>(borrow xs: Array<T>, n: usize): void;
```

- `Buffer` is a descriptor `{ data, len, cap }` with a stable handle; growth
  goes through the shim so the handle never changes and `borrow` stays sound.
- `Array<T>` accepts **duplicable element types only**; a non-duplicable
  instantiation is E90000 (AR-49 — v1 has no drop glue, so element drops
  cannot be generated). With duplicable elements, structural drop releases the
  `Buffer` and no element owns a resource.
- `get(i)` returns a one-element view via the `slice` intrinsic, so it traps
  E11211 out of range and creates a loan of the array's storage (E10022/E10031
  apply while it lives). Dot-call resolves `xs.push(v)` to `push(xs, v)` (F8).
- Helper signatures not printed here (for example `reserve` growth) follow
  `appendix-stdlib.md` §2-§3; growth on OOM panics (E1).

## `std.text`

```clorraine
module std.text;

export handle String freedBy FreeString;   // T2a, P6 layout

export func stringOf(v: str): String;      // allocates; panics on OOM (AR-53)
export func clone(s: String): String;      // independent copy
export func bytes(s: String): u8[];        // shared loan of s (appendix §9)
export func toCString(s: String): cstring; // NUL guaranteed by the layout
```

`String` layout is the P6 block `[len: u32][cap: u32][bytes][NUL]`; the handle
points at the block start. `str` is the value struct `{ptr, len}`. `static str`
and `String` widen to `str` implicitly; every other conversion is a named call
(T5). In-place mutation of `String` growing through `borrow` is deferred
(corpus needs none); construction, reading, cloning, and dropping are in scope.
The template builder is shim code (P8a) and panics on OOM (AR-53).

## `std.fmt`

```clorraine
module std.fmt;

export func text(v: i32): String;   export func text(v: u32): String;
export func text(v: f64): String;   export func text(v: bool): String;
export func text(v: str): String;
export func toHex(v: u32): String;      // "0x" + uppercase hex, no padding
export func toDecimal(v: i32): String;  // signed decimal
export func padLeft(s: str, width: usize, fill: u8): String;
export func padRight(s: str, width: usize, fill: u8): String;
```

Float `text` uses six significant digits (S2a). All builders return plain
`String` and panic on OOM (AR-53). They forward to the `cl_text_*` and
`cl_string_*` shim functions (the same ones templates desugar to), so
formatting behavior is identical between templates and explicit calls.

## `std.io`

```clorraine
module std.io;

export handle File freedBy Close;              // Ch 12 §6; BPTR on Amiga
export const MODE_OLDFILE: i32 = 1005;
export const MODE_NEWFILE: i32 = 1006;

export func openFile(path: cstring, mode: i32): File?;
export func createFile(path: cstring): File?;  // MODE_NEWFILE
export func readFile(borrow f: File, borrow buf: u8[]): i32;   // bytes read, <= 0 EOF/error
export func writeBytes(borrow f: File, const buf: u8[]): i32;  // bytes written
export func print(s: str): void;
export func println(s: str): void;
```

`readFile`/`writeBytes` return the transfer count (partial IO is the caller's
loop; `appendix-stdlib.md` §4). `print`/`println` write the view bytes through
`cl_write`; no allocation, no NUL requirement. `File` is declared once here
(CN-5) with the validated Ch 12 §6 contract (`Open`, `Close`, `Read`, `Write`).

## `std.amiga.dos` and `std.amiga.interrupt`

```clorraine
module std.amiga.dos;
export func delayTicks(ticks: u32): void;      // dos Delay

module std.amiga.interrupt;
export struct HandlerToken { id: u32; }
export func addHandler(hook: func(): void): HandlerToken;
export func removeHandler(t: HandlerToken): void;
```

Both modules are outside the host profile surface: importing either under
`--profile host` is E11200 (PR-2, AR-30). `addHandler` installs a VBlank
server through the shim, dispatches to the capture-less Clorraine function
pointer (L3/L5), and returns the token; removal is idempotent. Handlers remain
allocation-free, drop-free, panic-free by contract (IH-2, audited).

## Shim C API (AR-19)

The shim is profile-specific C with **no ownership logic** (Ch 17 §12); it is
prepended to the emitted unit and implements the symbols the stdlib binds.
The host and hosted-amiga shims share the API and differ only in implementation:

| Group | Symbols |
|---|---|
| Types | fixed-width typedefs, `bool`, `usize`, `ClStr`, `ClSlice_*`, `ClString`, `ClBuffer`, `ClFile` |
| Memory | `cl_zero`, `cl_memcpy`, `cl_memcmp`, `cl_alloc`, `cl_free` |
| Strings | `cl_string_new`, `cl_string_append`, `cl_text_i32/u32/f64/bool/str`, `FreeString`, `cl_string_ptr`, `cl_string_len` |
| Buffers | `cl_buffer_new`, `cl_buffer_ptr`, `cl_buffer_len`, `cl_buffer_set_len`, `cl_buffer_grow`, `FreeBuffer` |
| IO | `Open`, `Close`, `Read`, `Write`, `cl_write`, `print`/`println` support |
| Faults | `cl_trap(code, message)` (first line `error CODE: message`, then abort; AR-36), `cl_panic(message)` (`panic: <message>`) |
| Amiga | `Delay`, `cl_add_handler`, `cl_remove_handler` (VBlank server) |

Per-profile differences: host implements `Open`/`Close`/`Read`/`Write` over
`fopen`/`fread`/`fwrite`/`fclose` with `LONG`-style returns and maps the two
mode constants; hosted-amiga calls exec/dos directly (Phase 0-validated
prototypes, `docs/findings-phase0.md`); `ClFile` is `FILE*` on host and `BPTR`
on Amiga; fault output is stderr+`abort` on host and stdout+exit on Amiga
(AR-36); the Amiga shim additionally owns the C-side `__interrupt` VBlank
server. The spike S2 fixture fixes the ABI expectations.

## Layout Contracts

| Type | Layout | Empty state |
|---|---|---|
| `String` | one block `[len: u32][cap: u32][bytes][NUL]`; handle = block start (P6) | `NULL` |
| `Buffer` | descriptor `{ data: raw u8, len: u32, cap: u32 }`; handle = descriptor | `NULL` |
| `Array<T>` | `{ data: Buffer, count: usize }`; `len` lowers to `.count` (AR-50) | `data = NULL`, `count = 0` |
| `Span` | `{ start: usize, len: usize }` (appendix §8) | zero |
| `Status` | `{ ok: bool, code: u32 }` (appendix §4) | `{false, 0}` |
| `HandlerToken` | `{ id: u32 }` | zero |

## Build and Embedding

- `stdlib.rs` holds a fixed table `module name → include_str!` for the six
  files; `clc` embeds them and loads a module when an import names it
  (AR-25). No filesystem lookup and no install path (deterministic output).
- The stdlib is compiled with every program, so the compiler must accept its
  own library: a spec test compiles the stdlib alone with zero diagnostics
  (AR-42), and every corpus compile exercises it end to end.
- The shims are embedded from `compiler/runtime/` the same way.

## Testing Requirements

- ST cases in [07-testing-strategy.md](07-testing-strategy.md): warning-free
  stdlib compile under both profiles; per-module behavior programs (buffer
  bytes, array push/sum, format output, file copy round-trip, span bounds).
- The corpus is the primary stdlib integration test; there are no shim mocks
  (real objects only, per the standards).
