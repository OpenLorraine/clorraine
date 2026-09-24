# Appendix B - Standard Library (v1)

> **Status:** draft (Phase C - written against the ledger and the grammar)
> **Depends on:** Ch 08 (arrays and strings), Ch 11 (memory and resources), Ch 12 (FFI and contracts), Ch 14 (diagnostics), Ch 16 (borrow-checker rules), Ch 17 (codegen)
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) sections 3.1 (parameter modes), 4 (types), 6.4 (member and call suffixes), 7 (intrinsics)
> **Ledger:** [decisions.md](decisions.md) section 18 (standard-library scope); T2, T5, T7, T8, F7, F8, X1, X6, L5, P5, P6, P8a

Normative surface for the v1 standard library: the signatures and ownership
rules behind the ledger list in section 18. The library is ordinary Clorraine;
its `ffi` blocks, raw access, and volatile intrinsics stay inside it (X6).
Library APIs are dot-call style (F8): `xs.push(v)` resolves to `push(xs, v)`.

## 1. Reading the signatures

| Convention | Rule |
|---|---|
| parameter modes | mode first: `borrow x: T`, `take x: T`; unmarked and `const` are read-only lends (V5) |
| allocation | an allocator returns `T?`; `null` is the failure sentinel, checked before use (T8, E10010) |
| moves | a `take` argument transfers ownership; the caller's binding is invalid (E10020) |
| views | a view never owns storage; a live view blocks a move (E10022) or a mutation (E10031) of its owner |
| raw and unsafe | `ffi`, `raw` access, and volatile intrinsics live in the library, never in application code (X6) |

## 2. `Buffer` - owned bytes

`Buffer` owns one heap block of bytes (bulk IO, bitmaps, samples). `String` keeps
its own `[len][cap][bytes][NUL]` layout (P6).

```clorraine
module Std.Mem;

export handle Buffer freedBy FreeBuffer;

ffi stdmem {
    func FreeBuffer(take b: Buffer): void;
}
```

| API (dot-call) | Returns | Ownership and rules |
|---|---|---|
| `bufferNew(size: usize)` | `Buffer?` | allocates; the caller owns it; `null` on failure |
| `bufferFrom(src: u8[])` | `Buffer?` | allocates and copies `src`; `null` on failure |
| `bufferBytes(borrow b: Buffer)` | `u8[]` | view of every byte; the lend is exclusive (O12) |
| `bufferLen(b: Buffer)` | `usize` | byte count |
| `slice(view, lo, hi)` | `u8[]` | sub-view into the same storage; no allocation |

`Buffer` is move-only, so it drops through `FreeBuffer(take b)` in the cleanup
ladder, null-safely (P5). A view is a loan: moving or freeing the buffer while it
is live is E10022; a write through a shared loan is E10031 ([Ch 16](16-borrow-rules.md)).

## 3. `Array<T>` - growable sequence

The default dynamic container: a growable sequence owning a `Buffer`. It is
move-only, and its element type decides duplicability (V8).

| API (dot-call) | Returns | Ownership and rules |
|---|---|---|
| `arrayNew<T>()` | `Array<T>` | empty array; constructors need explicit type arguments (G2) |
| `xs.len()` | `usize` | element count; resolves to the intrinsic `len` (grammar section 7) |
| `xs.reserve(n: usize)` | `void` | grow capacity to hold at least `n`; exclusive lend |
| `xs.push(take v: T)` | `void` | append `v`, moving it into the array |
| `xs.get(i: usize)` | `T[]` | one-element view at `i`; bounds-checked (E11211) |
| `xs.set(i: usize, take v: T)` | `void` | replace element `i`, dropping the old one |

Element access is by index, not pointer, so a view carries no lifetime syntax.
While any element view is live, `push`, `reserve`, and `set` are E10031 or E10032:
**no reallocation while a view is live** (O11, O12). On drop, elements drop
first-to-last, then the storage (O2).

```clorraine
import { Array, arrayNew } from std.mem;

func sum(borrow xs: Array<i32>): i32 {
    let total: i32 = 0;
    let i: usize = 0;
    while (i < xs.len()) { total = total + xs.get(i)[0]; i = i + 1; }
    return total;
}
```

## 4. One error/status carrier

Detail-carrying failures use one status struct; `T?` stays detail-free (E1, E6):
`struct Status { ok: bool; code: u32; }`.

| Convention | Rule |
|---|---|
| shape | `ok` is the success flag; `code` is meaningful only when `ok` is `false` |
| empty state | `Status { ok: false, code: 0 }` is the safe zero value (O7); a detail-free failure uses `T?` instead |
| partial IO | return the count moved plus a `Status`; `ok` means the request completed |
| libraries | wrappers return `Status`; they never hide a failure behind `panic` (E4) |

## 5. Format helpers

Templates desugar to these helpers (P8a); a non-formattable type is E10500.
Format specs are not in v1 (S3), so call a helper instead (E10501).

| Helper | Returns | Direction |
|---|---|---|
| `text(v)` | `String` | formattable value or `str` to an owned `String`; floats use six significant digits (S2a) |
| `toHex(v: u32)` | `String` | unsigned value to `0x`-prefixed hex |
| `toDecimal(v: i32)` | `String` | signed value to decimal |
| `padLeft(s: str, width: usize, fill: u8)` | `String` | left-pad to `width` |
| `padRight(s: str, width: usize, fill: u8)` | `String` | right-pad to `width` |

## 6. Per-chip MMIO wrappers

The library owns the register addresses and the only `unsafe` volatile access
(T6, T7); application code never writes a chip address or a volatile intrinsic.

| Item | Rule |
|---|---|
| register bases | `const raw Regs = raw Regs(0x00DFF000);` at module level (T7) |
| overlay types | a `struct` with `volatile` fields, used only through a `raw` pointer (T11, E10207) |
| wrappers | one exported safe function per register operation; the volatile access stays inside |
| application code | outside `unsafe`; raw volatile access is E10604 |

```clorraine
module Std.Amiga.Custom;

struct CustomRegs { dmacon: volatile u16; }

const CUSTOM_BASE: raw CustomRegs = raw CustomRegs(0x00DFF000);

export func setDma(enabled: bool): void {
    unsafe {
        if (enabled) { CUSTOM_BASE.dmacon = CUSTOM_BASE.dmacon | 0x0080; }
    }
}
```

## 7. Interrupt and hook dispatcher

There is no `interrupt` declaration; interrupts and hooks go through one
dispatcher (F7). The library holds the C-side qualified handler, and Clorraine
code registers a capture-less `func(): void` and gets a token back.

| API | Returns | Rule |
|---|---|---|
| `addHandler(hook: func(): void)` | `HandlerToken` | installs the C-side handler; the token identifies the registration |
| `removeHandler(t: HandlerToken)` | `void` | unregister; removing an unknown token is safe |
| handler rules | - | allocation-free, drop-free, panic-free (IH-2); an audited convention, not a compiler check |

A token is `struct HandlerToken { id: u32; }`. The handler rules are enforced by
audit in v1; effect-marker enforcement is a Phase 2 follow-up. State
shared with a handler is a `volatile` module-level `let` (T7).

## 8. Screen lifecycle (`std.amiga.intuition`)

The module owns a custom screen. `openScreen` returns an owned `Screen?`; the
caller null-checks it (T8, E10010), and the generated drop closes the screen once
through the null-safe deleter (P5). The `RastPort` is never exposed; the drawing
calls live in the companion module (§9).

```clorraine
module std.amiga.intuition;

export handle Screen freedBy cl_close_screen;

ffi stdintuition {
    func cl_open_screen(width: i32, height: i32, depth: u8, title: cstring): Screen?;
    func cl_close_screen(take screen: Screen): void;
}

export func openScreen(width: i32, height: i32, depth: u8, title: cstring): Screen? {
    return cl_open_screen(width, height, depth, title);
}
```

| API | Returns | Ownership and rules |
|---|---|---|
| `openScreen(width: i32, height: i32, depth: u8, title: cstring)` | `Screen?` | opens a custom screen; `null` on failure. `width`/`height` are pixels; `depth` is the bitplane count (1-5 on the base chipset) |
| drop (generated) | `void` | `cl_close_screen(take screen)` runs exactly once; it is null-safe |

`title` names the screen and must stay alive for as long as the screen is open:
Intuition keeps the pointer and does not copy the string. `Screen` is move-only;
windows, text, bitmaps, sprites, and palettes beyond four bits are outside v1.

## 9. Drawing (`std.amiga.graphics`)

The module draws into a screen borrowed from `std.amiga.intuition`. Coordinates
are pixels and `pen` indexes the screen's color map; the `RastPort` calls stay
inside the shim (X6).

```clorraine
module std.amiga.graphics;

import { Screen } from std.amiga.intuition;

ffi stdgraphics {
    func cl_set_color(borrow screen: Screen, pen: u8, r: u8, g: u8, b: u8): void;
    func cl_fill(borrow screen: Screen, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8): void;
    func cl_draw(borrow screen: Screen, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8): void;
    func cl_clear(borrow screen: Screen, pen: u8): void;
}

export func setColor(borrow screen: Screen, pen: u8, r: u8, g: u8, b: u8): void {
    cl_set_color(screen, pen, r, g, b);
}

export func fill(borrow screen: Screen, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8): void {
    cl_fill(screen, x1, y1, x2, y2, pen);
}

export func draw(borrow screen: Screen, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8): void {
    cl_draw(screen, x1, y1, x2, y2, pen);
}

export func clear(borrow screen: Screen, pen: u8): void {
    cl_clear(screen, pen);
}
```

| API | Returns | Rule |
|---|---|---|
| `setColor(borrow screen, pen: u8, r: u8, g: u8, b: u8)` | `void` | maps 8-bit components to the Amiga 4-bit palette (`SetRGB4`); `pen` is valid in `0..(1 << depth) - 1` |
| `fill(borrow screen, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8)` | `void` | fills the inclusive rectangle (`RectFill`) |
| `draw(borrow screen, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8)` | `void` | draws the arbitrary line between the two points (`Draw`) |
| `clear(borrow screen, pen: u8)` | `void` | fills the whole screen with `pen` |

Every call takes the screen by `borrow`, so the caller keeps ownership; the
screen's depth bounds `pen`, and an out-of-range pen is a caller contract.
`graphics.library` clips coordinates outside the screen.

## 10. `Span` - offset-based parsing

`Span` is an offset pair into bytes the caller still owns. View fields are banned
(T4, E10205), so it stores no pointer; the source view is passed on each use:
`struct Span { start: usize; len: usize; }`.

| API | Returns | Rule |
|---|---|---|
| `spanOf(src: u8[], at: usize, n: usize)` | `Span` | bounds-checked (E11211); no storage moves |
| `spanBytes(src: u8[], s: Span)` | `u8[]` | the view is derived from `src`; the caller supplies the owner |

Because a `Span` holds no view, it may outlive one expression while the caller
holds `src`; a different `src` is a caller contract, not a compiler check.

## 11. `String` and `str` helpers

`String` is the owned handle and `str` the immutable view (T2); direction and
allocation are fixed (T5).

| Helper | From -> To | Allocation |
|---|---|---|
| `stringOf(v)` | value or `str` -> `String` | allocates |
| `clone(s)` | `String` -> independent `String` | allocates |
| `text(v)` | formattable -> `String` | allocates |
| `bytes(s)` | `String` -> `u8[]` | none |
| `toCString(s)` | `String` or `str` -> `cstring` | none |
| `slice(view, lo, hi)` | view -> same view type | none |
| `bufferBytes(b)` | `Buffer` -> `u8[]` | none |

`static str` widens to `str` and `String` widens to `str` implicitly; every other
conversion is one of the named calls (T5). Widening a `String` to `str` creates a
shared loan: mutating, moving, or reallocating the `String` while that view is
live is E10031 or E10022 (V9a). The same loan applies to any helper that
returns a view or `cstring` over owned storage — `bytes`, `toCString`, `slice`,
and `bufferBytes` — for the result's origin
([Ch 16](16-borrow-rules.md) section 2).

## 12. Staged for v1.1 and v2

| Item | Target | Reason |
|---|---|---|
| Pools and arenas | v2 | bulk release invalidates views; needs a region model (E90004, [Ch 11](11-memory.md) section 5) |
| `Box<T>` with generated drop glue | v1.1 | needs the X1 amendment allowing ordinary Clorraine `freedBy` functions (X1) |
| C-hook trampolines for capturing lambdas | v1.1 | capturing closures cross synchronous C hooks only, via generated trampolines (L5) |

## 13. Diagnostics

| Code | Trigger |
|---|---|
| E10010 | a nullable allocation result used before a null check |
| E10022 | moving or freeing an owner while a view of it is live |
| E10031 | mutating an owner while a shared view of it is live |
| E10604 | volatile access outside `unsafe` (inside a wrapper, not application code) |
| E11211 | runtime fault: an index or span offset out of range |

All message text lives in [Ch 14](14-diagnostics.md).
