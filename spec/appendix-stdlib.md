# Appendix B - Standard Library (v1)

> **Status:** stable (v1.0; v1.1 adds the A500 classic Intuition surface)
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
| `addHandler(hook: func(): void)` | `HandlerToken` | installs the vertical-blank (`INTB_VERTB`) handler; the token identifies the registration |
| `addPortHandler(hook: func(): void)` | `HandlerToken` | installs the ports (`INTB_PORTS`) handler; independent of the VBlank slot |
| `removeHandler(t: HandlerToken)` | `void` | removes whichever slot owns the token; removing an unknown token is safe |
| handler rules | - | allocation-free, drop-free, panic-free (IH-2); an audited convention, not a compiler check |

A token is `struct HandlerToken { id: u32; }`. There is one slot per kind, and
both kinds share one token counter, so a token names exactly one registration.
The handler rules are enforced by audit in v1; effect-marker enforcement is a
Phase 2 follow-up. State shared with a handler is a `volatile` module-level
`let` (T7).

## 8. Intuition (`std.amiga.intuition`)

The module owns two Intuition handles and the event value. `Screen` owns a
custom screen; `Window` owns a window on that screen and every gadget, menu,
string, and pointer image it builds. Both are move-only and close through the
generated drop. A window borrows its screen, so keep the screen alive until the
window has closed: locals drop in reverse declaration order (declare the screen
before the window), while struct fields drop in declaration order (declare the
window field before the screen field). Do not return a window from a function
whose locals own the screen.

The surface is the classic Kickstart 1.3 Intuition API. The same source
compiles on `hosted-amiga-13` and `hosted-amiga`; the 2.0+ additions (gadtools,
ASL requesters, and BOOPSI) are deferred and are not exposed. The host profile
rejects the module with `E11200`. The KS1.3 A500 gate is the runtime proof; the
`hosted-amiga` profile is compile-verified.

### 8.1 Screen and window lifecycle

| API | Returns | Ownership and rules |
|---|---|---|
| `openScreen(width: i32, height: i32, depth: u8, title: cstring)` | `Screen?` | opens a custom screen; `null` on failure. `width`/`height` are pixels; `depth` is the bitplane count (1-5 on the base chipset) |
| `saveScreen(borrow screen: Screen, path: cstring)` | `bool` | writes the screen's raw bitmap in `CSCR` form: the header carries the size and depth, followed by the palette and bitplanes. The path is an AmigaDOS path and the volume must be writable |
| drop (generated) | `void` | `cl_close_screen(take screen)` runs exactly once; it is null-safe |
| `openWindow(borrow screen: Screen, x: i32, y: i32, width: i32, height: i32, title: cstring)` | `Window?` | opens a window at screen pixel (`x`,`y`). `width`/`height` are the outer size; the inner drawable area is `windowWidth`/`windowHeight`. The window has a drag bar, depth gadget, close gadget, and `WFLG_GIMMEZEROZERO`. `null` for a null screen, a null or over-long title, a non-positive size, a size larger than the screen, or a rectangle that leaves the screen |
| `closeWindow(take window: Window)` | `void` | closes now instead of at scope end; clears the menu strip and pointer, then releases the window, gadgets, menus, strings, and pointer image |
| `windowWidth(borrow window: Window)` / `windowHeight(borrow window: Window)` | `i32` | inner drawable size in pixels; 0 on a null window |
| drop (generated) | `void` | `cl_close_window(take window)` runs exactly once; it is null-safe |

`title` names the screen and must stay alive for as long as the screen is open:
Intuition keeps the pointer and does not copy the screen title. Window titles,
gadget labels, menu titles, and menu item labels are copied, so the caller's
`cstring` only has to live for the call.

```clorraine
import { Screen, Window, openScreen, openWindow, waitEvent,
         EVENT_CLOSE, EVENT_NONE } from std.amiga.intuition;
import { String, stringOf, toCString } from std.text;

func demo(): i32 {
    let title: String = stringOf("demo");
    let screen: Screen? = openScreen(320, 200, 4, toCString(title));
    if (screen == null) { return 1; }
    let window: Window? = openWindow(screen, 24, 24, 272, 136,
                                     toCString(stringOf("window")));
    if (window == null) { return 1; }
    let running: bool = true;
    while (running) {
        let event = waitEvent(window);
        if (event.kind == EVENT_CLOSE || event.kind == EVENT_NONE) {
            running = false;
        }
    }
    return 0;   // the window drops first, then the screen
}
```

### 8.2 IDCMP events

`WindowEvent` is a copy-only value: it owns nothing and can be stored or
compared freely. `kind` selects one of the `EVENT_*` constants; `code` and
`qualifier` carry the raw Intuition message values. `mouse_x` and `mouse_y` are
the current inner-window pointer coordinates (`GZZMouseX`/`GZZMouseY`), not
necessarily the event message's own coordinates. `gadget` and `menu` carry the
caller's id when the event names a gadget or menu item, and 0 otherwise.

| Constant | Meaning |
|---|---|
| `EVENT_NONE` | no event; returned by `pollEvent` and for an invalid or closed window |
| `EVENT_CLOSE` | the window's close gadget was selected |
| `EVENT_GADGET` | a gadget was released; `gadget` names it |
| `EVENT_MENU` | a menu item was selected; `menu` names it |
| `EVENT_MOUSE` | a mouse button event; `code`, `qualifier`, and the mouse fields are set |
| `EVENT_KEY` | a raw or vanilla key event; `code` and `qualifier` are set |
| `EVENT_ACTIVE` / `EVENT_INACTIVE` | the window became active or inactive |

| API | Returns | Rules |
|---|---|---|
| `waitEvent(borrow window: Window)` | `WindowEvent` | blocks for the next event; returns `EVENT_NONE` for a null or closed window |
| `pollEvent(borrow window: Window)` | `WindowEvent` | returns the next queued event, or `EVENT_NONE` when the queue is empty |

Both calls reply the Intuition message before returning, so the queue cannot
grow without bound. Refresh events are acknowledged inside the shim and are not
delivered; the window is not resizable.

### 8.3 Gadgets and menus

Gadget and menu-item ids must be at least 1 and unique within the window; the
event fields use 0 for "none". Buttons use pens 0, 1, and 2 (label, fill, and
select), so the caller sets those screen pens before adding a button. Strings
and labels are copied. A label wider than its gadget is clipped by Intuition.

| API | Returns | Rules |
|---|---|---|
| `addButton(borrow window: Window, x: i32, y: i32, width: i32, height: i32, label: cstring, id: i32)` | `bool` | appends a classic boolean button; the hit box is the inner-window rectangle. `false` for invalid geometry, an over-long label, a duplicate id, or allocation failure |
| `addString(borrow window: Window, x: i32, y: i32, width: i32, height: i32, label: cstring, initial: cstring, maxChars: i32, id: i32)` | `bool` | appends a classic string gadget; `maxChars` is 1..64. Enter delivers `EVENT_GADGET` with `id` |
| `windowString(borrow window: Window, id: i32, borrow buf: u8[])` | `i32` | copies the current text into `buf` and returns the byte count; 0 for an unknown or non-string id, or an empty buffer. The copy is bounded by the gadget's own limit and NUL-terminated when there is room |
| `activateGadget(borrow window: Window, id: i32)` | `bool` | sends keyboard focus to the gadget; intended for a string field |
| `addMenu(borrow window: Window, title: cstring)` | `i32` | appends a menu header and returns its zero-based index, or -1 on failure |
| `addMenuItem(borrow window: Window, menu: i32, label: cstring, id: i32)` | `bool` | appends an enabled textual item; a selection delivers `EVENT_MENU` with `id` |
| `addMenuItemKey(borrow window: Window, menu: i32, label: cstring, id: i32, key: u8)` | `bool` | the same item with an ASCII command key, invoked with Right Amiga plus that character |

Menu titles, item labels, and window and gadget labels are limited to 64 bytes.
Submenus, checkmarks, toggles, and menu enable/disable are outside this surface.

### 8.4 Requesters, alerts, and pointer

| API | Returns | Rules |
|---|---|---|
| `askYesNo(borrow window: Window, body: cstring)` | `bool` | shows a blocking 240x40 `AutoRequest` with fixed Yes/No labels; `true` for Yes. The body is copied and limited to 28 bytes; a null window, a null body, an over-long body, or allocation failure returns `false` without showing a requester |
| `showAlert(body: cstring)` | `void` | shows a blocking recovery alert with a Continue button on the default screen; the body is copied and limited to 28 bytes. A null or over-long body is a no-op |
| `setPointer(borrow window: Window, show: bool)` | `void` | installs or clears the built-in 16x16 arrow. The first `true` allocates it in chip memory and later calls reuse it; `false` clears it without freeing. The arrow uses sprite registers 17 (white) and 18 (black); set them with `setColor`. The image is freed when the window closes |

`DisplayAlert` uses a private modal screen, so the alert temporarily covers the
caller's custom screen; the window layer restores its content when the alert
returns. The interactive A500 acceptance uses `askYesNo` because the private
alert screen cannot be focused reliably by the headless harness; `showAlert`
stays part of the surface and is covered by the specification test.

The native-video path in §15 is the bitmap/blitter/copper alternative to an
Intuition screen.

## 9. Drawing (`std.amiga.graphics`)

The module draws into a screen or window borrowed from `std.amiga.intuition`.
Screen coordinates are screen pixels; window coordinates are relative to the
inner drawable area. `pen` indexes the screen's color map; the `RastPort` calls
stay inside the shim (X6).

```clorraine
module std.amiga.graphics;

import { Screen, Window } from std.amiga.intuition;

ffi stdgraphics {
    func cl_set_color(borrow screen: Screen, pen: u8, r: u8, g: u8, b: u8): void;
    func cl_fill(borrow screen: Screen, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8): void;
    func cl_draw(borrow screen: Screen, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8): void;
    func cl_clear(borrow screen: Screen, pen: u8): void;
    func cl_window_fill(borrow window: Window, x1: i32, y1: i32, x2: i32,
                        y2: i32, pen: u8): void;
    func cl_window_draw(borrow window: Window, x1: i32, y1: i32, x2: i32,
                        y2: i32, pen: u8): void;
    func cl_window_clear(borrow window: Window, pen: u8): void;
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

export func windowFill(borrow window: Window, x1: i32, y1: i32, x2: i32,
                       y2: i32, pen: u8): void {
    cl_window_fill(window, x1, y1, x2, y2, pen);
}

export func windowDraw(borrow window: Window, x1: i32, y1: i32, x2: i32,
                       y2: i32, pen: u8): void {
    cl_window_draw(window, x1, y1, x2, y2, pen);
}

export func windowClear(borrow window: Window, pen: u8): void {
    cl_window_clear(window, pen);
}
```

| API | Returns | Rule |
|---|---|---|
| `setColor(borrow screen: Screen, pen: u8, r: u8, g: u8, b: u8)` | `void` | maps 8-bit components to the Amiga 4-bit palette (`SetRGB4`); `pen` indexes the 32-entry color map. On a depth-4 screen, 0-15 are the playfield colors and 16-31 are the sprite/pointer registers; a depth-5 screen uses 0-31 for the playfield. `pen` at or above 32 is a no-op |
| `fill(borrow screen: Screen, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8)` | `void` | fills the inclusive rectangle (`RectFill`) |
| `draw(borrow screen: Screen, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8)` | `void` | draws the arbitrary line between the two points (`Draw`) |
| `clear(borrow screen: Screen, pen: u8)` | `void` | fills the whole screen with `pen` |
| `windowFill(borrow window: Window, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8)` | `void` | fills the inner-window rectangle (`RectFill` on the window RastPort) |
| `windowDraw(borrow window: Window, x1: i32, y1: i32, x2: i32, y2: i32, pen: u8)` | `void` | draws a line between inner-window points (`Draw`) |
| `windowClear(borrow window: Window, pen: u8)` | `void` | fills the whole inner window |

Every call takes the screen or window by `borrow`, so the caller keeps
ownership. `setColor` indexes the 32-entry color map and ignores a pen at or
above 32. For drawing, only pens `0..(1 << depth) - 1` are playfield colors; a
higher pen is a caller contract because the sprite/pointer registers are not
drawing colors. `graphics.library` clips coordinates outside the target.

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

## 13. Exec (`std.amiga.exec`)

The module owns private `MsgPort` handles, two move-only message handles, a
worker `Task` handle, a `Library` handle, and a generic `Device` handle. It is
available on `hosted-amiga-13` and `hosted-amiga`; the host and freestanding
profiles reject it with `E11200`. The event-loop shape is in
[appendix-amiga.md](appendix-amiga.md) §6.

### 13.1 Worker tasks

`createTask` starts a capture-less worker at priority -1 with a
4096..65536-byte stack and returns an owned `Task?`. The worker's entry must
use only arithmetic, module-level scalars, and `reportTaskResult`; it must not
call DOS, Intuition, `std.io`, allocate, drop an owning value, or block. The
worker reports its result, returns, and the shim signals the parent. `joinTask`
waits for the worker, removes it with the Kickstart 1.3-safe
`Forbid`/`RemTask`/`Permit` sequence, and returns the result. The generated
drop joins an unjoined task. The restrictions are an audited convention; the
compiler cannot enforce them.

| API | Returns | Ownership and rules |
|---|---|---|
| `createTask(entry: func(): void, stackBytes: u32)` | `Task?` | starts a priority -1 worker; `stackBytes` 4096..65536; `null` on allocation failure |
| `reportTaskResult(value: i32)` | `void` | stores the current worker's result; no-op outside a created task |
| `joinTask(take task: Task)` | `i32` | waits, removes the worker, frees the join signal, returns the result |
| drop (generated) | `void` | joins and removes an unjoined task; it is null-safe |

```clorraine
import { Task, createTask, reportTaskResult, joinTask } from std.amiga.exec;

func worker(): void {
    reportTaskResult(42);
}

export func run(): i32 {
    let task: Task? = createTask(worker, 8192);
    if (task == null) { return 1; }
    return joinTask(task);   // waits for the worker and removes it
}
```

### 13.2 Message ports

The message surface owns private and public ports and two move-only message
handles.

| API | Returns | Ownership and rules |
|---|---|---|
| `createMsgPort()` | `MsgPort?` | private port; `null` on failure; delete it after draining |
| `createPublicPort(name: cstring)` | `MsgPort?` | named port for `FindPort`; `null` for an empty name or allocation failure; the name must stay alive until the port is deleted |
| `deleteMsgPort(take port)` | `void` | deletes a drained port |
| `createMessage(borrow reply: MsgPort, payload: i32)` | `NewMessage?` | allocates an unsent message with a reply port and an integer payload |
| `messagePayload(borrow msg: Message)` | `i32` | reads the payload of a fetched message |
| `putMsg(borrow port: MsgPort, take msg: NewMessage)` | `void` | moves the message into the port |
| `signals(borrow port: MsgPort)` | `u32` | the port's signal mask, `1 << mp_SigBit` |
| `waitSignals(mask: u32)` | `u32` | waits for any bit in `mask` and returns the bits received |
| `getMsg(borrow port: MsgPort)` | `Message?` | the next message, or `null` |
| `replyMsg(take msg: Message)` | `void` | replies the message and moves it to the reply path |
| `freeMessage(take msg: NewMessage)` | `void` | releases an unsent message explicitly |
| `releaseMsg(take msg: Message)` | `void` | releases a fetched message this program created that will not be replied |

`NewMessage` is a message this module created and has not sent; its generated drop releases it.
`Message` is a message fetched from a port; its generated drop replies it. `putMsg` consumes the
sent message; `getMsg` returns an owned `Message`; `replyMsg` consumes it. A message that has
returned through its reply port is released with `releaseMsg`, not replied a second time. The
caller must drain a port before deleting it.

```clorraine
import { MsgPort, signals, waitSignals, getMsg, replyMsg } from std.amiga.exec;

export func drain(borrow port: MsgPort): void {
    let sig = waitSignals(signals(port));
    if ((sig & signals(port)) != 0) {
        let msg = getMsg(port);
        while (msg != null) {
            replyMsg(msg);
            msg = getMsg(port);
        }
    }
}
```

### 13.3 Libraries

`Library` owns an open `Library *` base. `openLibrary(name, version)` opens a
library at or above `version`; version 0 accepts any installed version, and a
version above the installed library returns `null`. `closeLibrary` calls
`CloseLibrary` exactly once; the generated drop is null-safe.
`libraryVersion`/`libraryRevision` read `lib_Version`/`lib_Revision` and return
0 for a null handle.

| API | Returns | Ownership and rules |
|---|---|---|
| `openLibrary(name: cstring, version: u16)` | `Library?` | `OpenLibrary`; `null` on a null name or a missing/too-old library |
| `closeLibrary(take library: Library)` | `void` | `CloseLibrary` exactly once |
| `libraryVersion(borrow library: Library)` | `u16` | `lib_Version`, or 0 for null |
| `libraryRevision(borrow library: Library)` | `u16` | `lib_Revision`, or 0 for null |
| drop (generated) | `void` | closes an unclosed library; null-safe |

### 13.4 Devices

`Device` owns one `struct IOStdReq`, one private reply port, and an optional
device-owned asynchronous buffer. The reply port is installed in
`io_Message.mn_ReplyPort` before `OpenDevice`, so the request can be replied.
The handle owns the request, the port, and the asynchronous buffer; the caller
never keeps a raw data pointer inside the request.

Synchronous I/O borrows the caller's buffer for the duration of the call:
`deviceDoIO(device, buffer)` sets `io_Data`/`io_Length`, runs `DoIO`, and
returns `io_Error`. A synchronous read writes directly into the caller's
buffer, and the borrow checker keeps that buffer alive for the call.

Asynchronous I/O uses a device-owned buffer so the caller cannot leave a
dangling pointer behind a returned handle. `deviceSetBuffer(device, length)`
reserves the buffer (length 0 releases it), `deviceWriteBuffer(device, data)`
copies a write payload into it, and `deviceSendIO(device)` starts the request.
`deviceWaitIO(device, dest)` and `deviceCheckIO(device, dest)` copy up to
`len(dest)` bytes of the result out and return the request status. The buffer
is freed when the device closes. Only the standard `IOStdReq`-sized devices
are accepted; `audio.device`, `timer.device`, `serial.device`, and
`parallel.device` need larger requests and keep their typed modules.

`console.device` is bound to an Intuition `Window`, so it has a typed
`openConsole(window, unit)` helper: it fills `io_Data` with the window and
`io_Length` with `sizeof(struct Window)`. The window is refcounted: closing the
`Window` handle while the console is open defers the actual close until the
console device closes, so the console cannot use a freed window.

The generic command constants are `CMD_INVALID` (0), `CMD_RESET` (1),
`CMD_READ` (2), `CMD_WRITE` (3), `CMD_UPDATE` (4), `CMD_CLEAR` (5),
`CMD_STOP` (6), `CMD_START` (7), and `CMD_FLUSH` (8).

| API | Returns | Ownership and rules |
|---|---|---|
| `openDevice(name: cstring, unit: i32, flags: u32)` | `Device?` | opens a standard `IOStdReq` device; `unit` 0..255; `null` on failure or an extended-request device |
| `openConsole(borrow window: Window, unit: u8)` | `Device?` | opens `console.device` on an Intuition window; the window is refcounted and closes after the device |
| `closeDevice(take device: Device)` | `void` | aborts a pending request, waits, closes the device, deletes the reply port, frees the asynchronous buffer, and releases the console window; null-safe |
| `deviceCommand(borrow device, command: u16)` | `bool` | sets `io_Command`; false for a null handle or a pending request |
| `deviceSetBuffer(borrow device, length: u32)` | `bool` | reserves a device-owned asynchronous buffer; length 0 releases it; false for a null handle, a pending request, or allocation failure |
| `deviceWriteBuffer(borrow device, borrow data: u8[])` | `bool` | copies a write payload into the device-owned buffer; false without a buffer, while pending, or when `data` is longer |
| `deviceSetOffset(borrow device, offset: u32)` | `bool` | sets `io_Offset`; the unit is device-specific; false for a null handle or a pending request |
| `deviceDoIO(borrow device, borrow data: u8[])` | `i32` | synchronous `DoIO` over the caller's buffer; returns `io_Error`; a pending request returns -1 |
| `deviceSendIO(borrow device)` | `bool` | starts `SendIO` over the device-owned buffer; false for a null handle or a pending request |
| `deviceWaitIO(borrow device, borrow data: u8[])` | `i32` | waits for a pending request, copies the result out, and returns `io_Error`; idle returns the last error, null returns -1 |
| `deviceCheckIO(borrow device, borrow data: u8[])` | `bool` | true when a pending request completed; reaps the reply, copies the result out, and clears the pending state |
| `deviceAbortIO(borrow device)` | `void` | requests an abort; the caller reaps it with `deviceWaitIO` |
| `deviceActual(borrow device)` | `u32` | the last transfer length |
| `deviceError(borrow device)` | `i32` | the last request error, or -1 for a null handle |
| drop (generated) | `void` | `closeDevice` semantics; null-safe |

```clorraine
import { Device, CMD_READ, openDevice, closeDevice, deviceCommand,
         deviceSetOffset, deviceDoIO, deviceActual } from std.amiga.exec;

export func readBlock(borrow buffer: u8[]): u32 {
    let device: Device? = openDevice(toCString(stringOf("trackdisk.device")), 0, 0);
    if (device == null) { return 0; }
    if (!deviceCommand(device, CMD_READ) || !deviceSetOffset(device, 0)) {
        closeDevice(device);
        return 0;
    }
    let error: i32 = deviceDoIO(device, borrow buffer);
    let actual: u32 = deviceActual(device);
    closeDevice(device);
    return error == 0 ? actual : 0;
}
```

For an asynchronous read, reserve the device buffer first, start the request,
and copy the result out at the wait:

```clorraine
if (!deviceSetBuffer(device, 512) || !deviceSendIO(device)) {
    closeDevice(device);
    return 0;
}
let error: i32 = deviceWaitIO(device, borrow buffer);
```

## 14. Icon, Workbench, and expansion

This section covers the Kickstart 2.0+ services. The three modules are
available only on `hosted-amiga`; the host and `hosted-amiga-13` profiles
reject them with `E11200`. The Workbench-disk services (commodities, locale,
ARexx, clipboard, serial/parallel/printer) and `nonvolatile.library` are not
part of this surface because the acceptance baseline has no Workbench media.

### 14.1 Icon library (`std.amiga.icon`)

`DiskObject` owns one in-memory `.info` icon. `getDiskObject(name)` reads
`<name>.info`; `putDiskObject(name, icon)` writes it; the caller frees every
handle with `freeDiskObject` or the generated drop. `getDefDiskObject(kind)`
returns a fresh default icon for one of `ICON_DISK`, `ICON_DRAWER`,
`ICON_TOOL`, `ICON_PROJECT`, `ICON_GARBAGE`, `ICON_DEVICE`, `ICON_KICK`, and
`ICON_APPICON`. `diskObjectType`/`setDiskObjectType` read or validate the
`do_Type` field.

| API | Returns | Ownership and rules |
|---|---|---|
| `getDiskObject(name: cstring)` | `DiskObject?` | reads `<name>.info`; `null` on a missing file or failure |
| `putDiskObject(name: cstring, borrow icon: DiskObject)` | `bool` | writes `<name>.info`; false on a null name/icon or a write failure |
| `freeDiskObject(take icon: DiskObject)` | `void` | frees exactly once; null-safe |
| `getDefDiskObject(kind: i32)` | `DiskObject?` | fresh default icon; the caller owns it |
| `diskObjectType(borrow icon: DiskObject)` | `i32` | `do_Type`, or 0 for null |
| `setDiskObjectType(borrow icon: DiskObject, kind: i32)` | `bool` | accepts `ICON_DISK`..`ICON_APPICON`; false otherwise |
| drop (generated) | `void` | frees an unfreed icon; null-safe |

```clorraine
import { DiskObject, ICON_TOOL, getDefDiskObject, putDiskObject,
         getDiskObject, diskObjectType, freeDiskObject } from std.amiga.icon;

func roundTrip(name: cstring): i32 {
    let icon: DiskObject? = getDefDiskObject(ICON_TOOL);
    if (icon == null) { return 0; }
    let ok: bool = putDiskObject(name, icon);
    freeDiskObject(icon);
    if (!ok) { return 0; }
    let loaded: DiskObject? = getDiskObject(name);
    if (loaded == null) { return 0; }
    let kind: i32 = diskObjectType(loaded);
    freeDiskObject(loaded);
    return kind;
}
```

### 14.2 Workbench application objects (`std.amiga.workbench`)

`AppIcon`, `AppWindow`, and `AppMenuItem` register application objects with
Workbench and return owned handles. `AppWindow` borrows the `Window`: remove it
before the window closes. `waitAppMessage` blocks on the port and returns an
owned `AppMessage`; `replyAppMessage` or the generated drop replies it exactly
once. `am_Type` is one of `APPMSG_APPWINDOW` (7), `APPMSG_APPICON` (8), or
`APPMSG_APPMENUITEM` (9); `am_Class` 0 is `APPMSG_OPEN`. The module does not
start Workbench; the acceptance harness starts the ROM-resident task with a
test-only fixture.

| API | Returns | Ownership and rules |
|---|---|---|
| `addAppIcon(borrow port: MsgPort, title: cstring, id: u32, userData: u32)` | `AppIcon?` | default image, pinned position, copied title; `null` on failure |
| `removeAppIcon(take icon: AppIcon)` | `void` | removes the icon and frees its image/title |
| `addAppWindow(borrow window: Window, borrow port: MsgPort, id: u32, userData: u32)` | `AppWindow?` | the window must outlive the AppWindow |
| `removeAppWindow(take window: AppWindow)` | `void` | removes the AppWindow |
| `addAppMenuItem(borrow port: MsgPort, title: cstring, id: u32, userData: u32)` | `AppMenuItem?` | copied title; appears in the Tools menu |
| `removeAppMenuItem(take item: AppMenuItem)` | `void` | removes the menu item and its title |
| `waitAppMessage(borrow port: MsgPort)` | `AppMessage?` | blocks; `null` only for a null port |
| `appMessageType(borrow message)` | `i32` | `am_Type`, or 0 for null |
| `appMessageId(borrow message)` | `u32` | `am_ID`, or 0 for null |
| `appMessageUserData(borrow message)` | `u32` | `am_UserData`, or 0 for null |
| `appMessageClass(borrow message)` | `i32` | `am_Class`, or 0 for null |
| `appMessageArgCount(borrow message)` | `i32` | `am_NumArgs`, or 0 for null |
| `replyAppMessage(take message: AppMessage)` | `void` | replies exactly once |
| `savePublicScreen(name: cstring, path: cstring)` | `bool` | native `CSCR` capture of a public screen |
| drop (generated) | `void` | removes an unremoved object or replies an unhandled message |

```clorraine
import { MsgPort, createPublicPort } from std.amiga.exec;
import { AppIcon, AppMessage, addAppIcon, removeAppIcon,
         waitAppMessage, appMessageType, appMessageId, appMessageUserData,
         appMessageClass, appMessageArgCount, replyAppMessage }
    from std.amiga.workbench;

func announce(name: cstring): i32 {
    let port: MsgPort? = createPublicPort(name);
    if (port == null) { return 1; }
    let icon: AppIcon? = addAppIcon(port, name, 7, 42);
    if (icon == null) { return 2; }
    let message: AppMessage? = waitAppMessage(port);
    if (message == null) { return 3; }
    let kind: i32 = appMessageType(message);
    let id: u32 = appMessageId(message);
    let user: u32 = appMessageUserData(message);
    let class: i32 = appMessageClass(message);
    let args: i32 = appMessageArgCount(message);
    replyAppMessage(message);
    removeAppIcon(icon);
    return kind + i32(id) + i32(user) + class + args;
}
```

### 14.3 Expansion, card, and keymap (`std.amiga.expansion`)

`expansionBoardCount` walks the read-only `ConfigDev` list. The per-board
accessors return the `ExpansionRom` fields; an out-of-range index returns 0.
`cardPresent`/`cardStatus` treat the optional `card.resource` as absent without
failing. `mapRawKey` maps a raw key and qualifier to text, or `null` when the
keymap library is absent or the key is unmapped. The bare ROM has a default
keymap object but no loaded `DEVS:keymaps` layout, so an acceptance machine
without Workbench media reports no mapped text.

| API | Returns | Rules |
|---|---|---|
| `expansionBoardCount()` | `i32` | number of `ConfigDev` boards; 0 when the library is absent |
| `expansionBoardManufacturer(index)` | `u16` | `cd_Rom.er_Manufacturer`; 0 out of range |
| `expansionBoardProduct(index)` | `u8` | `cd_Rom.er_Product`; 0 out of range |
| `expansionBoardSerial(index)` | `u32` | `cd_Rom.er_SerialNumber`; 0 out of range |
| `expansionBoardKind(index)` | `u8` | `cd_Rom.er_Type`; 0 out of range |
| `cardPresent()` | `bool` | true when `card.resource` opens |
| `cardStatus()` | `u32` | `ReadCardStatus`, or 0 when the resource is absent |
| `mapRawKey(code: u16, qualifier: u16)` | `String?` | owned text, or `null` when absent/unmapped |

```clorraine
import { expansionBoardCount, expansionBoardManufacturer,
         expansionBoardProduct, cardPresent, cardStatus, mapRawKey }
    from std.amiga.expansion;
import { String } from std.text;

func probe(): i32 {
    let boards: i32 = expansionBoardCount();
    let manufacturer: u16 = expansionBoardManufacturer(0);
    let product: u8 = expansionBoardProduct(0);
    let present: bool = cardPresent();
    let status: u32 = cardStatus();
    let text: String? = mapRawKey(0x20, 0);
    let mapped: i32 = text == null ? 0 : 1;
    return boards + i32(manufacturer) + i32(product) + i32(status) + mapped +
           (present ? 1 : 0);
}
```

## 15. Native video (`std.amiga.video`)

The module owns chip memory, displayable bitmaps, a copper builder, and a
double-buffered custom display. It is available on `hosted-amiga` and
`hosted-amiga-13`; the host and freestanding profiles reject it with `E11200`.
The display uses the 1.3 `LoadView` pipeline from
[appendix-amiga.md](appendix-amiga.md) §1.1, not the Intuition screen path.

```clorraine
module std.amiga.video;

export handle ChipBuffer freedBy cl_video_chip_free;
export handle Bitmap     freedBy cl_video_bitmap_free;
export handle Copper     freedBy cl_video_copper_free;
export handle Display    freedBy cl_video_display_free;

export func newBitmap(width: i32, height: i32, depth: u8): Bitmap? {
    return cl_video_bitmap_new(width, height, depth);
}

export func bitmapFill(borrow bitmap: Bitmap, x1: i32, y1: i32,
                       x2: i32, y2: i32, pen: u8): void {
    cl_video_bitmap_fill(bitmap, x1, y1, x2, y2, pen);
}

export func bitmapBlit(borrow dest: Bitmap, const src: Bitmap,
                       sx: i32, sy: i32, dx: i32, dy: i32,
                       w: i32, h: i32): void {
    cl_video_bitmap_blit(dest, src, sx, sy, dx, dy, w, h);
}

export func openDisplay(borrow front: Bitmap, borrow back: Bitmap): Display? {
    return cl_video_display_open(front, back);
}

export func displayFlip(borrow display: Display): void {
    cl_video_display_flip(display);
}
```

| API | Returns | Ownership and rules |
|---|---|---|
| `allocChip(size: u32)` | `ChipBuffer?` | zeroed `MEMF_CHIP` memory; `null` on failure |
| `freeChip(take buffer)` | `void` | releases the buffer; the generated drop is null-safe |
| `chipSize`, `chipRead8/16/32`, `chipWrite8/16/32` | `u32`/`u8`/`u16`/`u32`/`void` | bounds-checked access; an out-of-range offset is a no-op or zero |
| `newBitmap(width: i32, height: i32, depth: u8)` | `Bitmap?` | `AllocRaster` planes in chip memory; `width`/`height` 1..=1024, `depth` 1..=8 |
| `freeBitmap(take bitmap)` | `void` | frees every plane |
| `bitmapWidth/Height/Depth/BytesPerRow` | `i32`/`u8`/`u32` | geometry accessors |
| `bitmapClear`, `bitmapPlot`, `bitmapLine`, `bitmapFill` | `void` | software drawing; coordinates are clipped, pen bits above the depth are ignored |
| `bitmapPlaneBit`, `bitmapSetPlaneBit` | `bool`/`void` | direct plane-bit access |
| `bitmapBlit(borrow dest, const src, ...)` | `void` | `BltBitMap`, minterm `0xC0` |
| `bitmapBlitMasked(borrow dest, const src, const mask, ...)` | `void` | `BltMaskBitMapRastPort`, minterm `0xE0`; `mask` is depth 1 and matches the source dimensions |
| `bitmapChecksum(const bitmap)` | `u32` | FNV-1a over the geometry and every plane byte |
| `newCopper()` | `Copper?` | two `UCopList` streams, one per view |
| `copperWait`, `copperMove`, `copperColor`, `copperEnd` | `void` | append to both streams; 256-instruction limit |
| `displaySetCopper(borrow display, take copper)` | `bool` | attaches before the first flip; `false` after the display is built |
| `openDisplay(borrow front, borrow back)` | `Display?` | both bitmaps must match; the display borrows them and must drop first |
| `displaySetColor(borrow display, index, r, g, b)` | `void` | `SetRGB4CM`; call before the first flip |
| `displayFlip(borrow display)` | `void` | first call builds both views; later calls alternate and wait for VBlank |
| `displayFailed(const display)` | `bool` | true when the views could not be built; later flips are no-ops |
| `displayIndex(const display)` | `u8` | 0 for the front bitmap, 1 for the back |
| `waitVBlank()` | `void` | `WaitTOF` |

The `Display` borrows its two bitmaps; declare them before the display and do
not free them until the display is closed. A copper passed to
`displaySetCopper` moves into the display, which releases it through
`FreeVPortCopLists`.

## 16. Fixed-point math (`std.math`)

`std.math` is pure Clorraine and is available on `host`, `hosted-amiga`, and
`hosted-amiga-13`. Angles use 256 steps per turn; sine and cosine results are
scaled by 256.

| API | Returns | Rule |
|---|---|---|
| `sin(angle: i32)` | `i32` | `-256..=256`; angles wrap every 256 |
| `cos(angle: i32)` | `i32` | `sin(angle + 64)` |
| `sqrt(value: i32)` | `i32` | integer floor; negative input returns 0 |

## 17. Hardware sprites (`std.amiga.sprite`)

The module owns the eight OCS hardware sprite channels through the 1.3
`graphics.library` simple-sprite machine. It is available on `hosted-amiga` and
`hosted-amiga-13`; the host and freestanding profiles reject it with `E11200`.
Sprites are global DMA channels; the demo attaches them to an Intuition
`Screen` viewport, which already has a compiled display list.

| API | Returns | Rule |
|---|---|---|
| `newSprite(channel: u8, height: u8)` | `Sprite?` | channel 0..7, height 1..255; `null` if the channel or chip memory is unavailable |
| `freeSprite(take sprite)` | `void` | hides the channel, waits a frame, and releases the image and channel |
| `spriteSetPixels(borrow sprite, const pixels: u8[])` | `void` | `16 * height` values 0..3; packs the two sprite bitplanes |
| `spriteMove(borrow sprite, x, y)` | `void` | x 0..511, y 0..255; the sprite is 16 pixels wide |
| `spriteShow` / `spriteHide` | `void` | show restores the image; hide uses a zero image |
| `spriteAttach(borrow even, borrow odd)` | `bool` | adjacent channels; sets the odd ATTACH bit for 16 colors |
| `spriteAttachScreen(borrow sprite, borrow screen)` | `bool` | adds the sprite to the screen viewport and rebuilds its display list |
| `spriteDetach(borrow sprite)` | `void` | clears the screen/display pointer; call before releasing the screen |
| `enableCollisions()` | `void` | `CLXCON = 0xF000` for all sprite-pair groups |
| `collisions()` | `u32` | reads and clears `CLXDAT`; sprite-sprite bits are 9..14 |
| `newAnimation(channel, height, frames)` | `SpriteAnimation?` | 1..8 frames in chip memory |
| `animationSetFrame` / `animationShowFrame` / `animationNext` | `void` | pack, show, or advance one frame |
| `animationMove` / `animationShow` / `animationHide` | `void` | delegate to the animation's sprite |
| `animationAttachScreen` / `animationDetach` | `bool` / `void` | attach/detach the animation's sprite |
| `animationFrame` | `u8` | current frame index |

A sprite image is the documented `posctl[2] + data[height][2] + reserved[2]`
block in chip memory. The attached pair uses sprite palette registers 16..31;
a single channel `n` uses `16 + ((n & 6) << 1) + 1 .. +3`. The supported
lifetime is hide, detach, close the screen, then free the sprite. The shim also
clears a channel when its screen closes, so freeing after a close is safe even
if the explicit detach was omitted.

## 18. Audio (`std.amiga.audio`)

The module wraps `audio.device` with a private reply port, one `IOAudio`
block per allocated channel, and a chip-memory `Sample`.

| API | Returns | Rule |
|---|---|---|
| `openAudio(channels: u8)` | `Audio?` | 1..4; requests a channel mask and returns `null` on device/allocation failure |
| `freeAudio(take audio)` | `void` | stops pending writes, sends `ADCMD_FREE`, closes the device |
| `audioChannels(const audio)` | `u8` | allocated mask (bits 0..3) |
| `audioPlay(borrow audio, channel, borrow sample)` | `bool` | `CMD_WRITE` + `ADIOF_PERVOL`; false when the channel is busy |
| `audioStop(borrow audio, channel)` | `void` | aborts the playing write |
| `audioControl(borrow audio, channel, period, volume)` | `bool` | `ADCMD_PERVOL` |
| `newSample(const data: u8[], period, volume, cycles)` | `Sample?` | copies signed 8-bit samples to chip memory; even length; `cycles=0` loops |
| `freeSample(take sample)` | `void` | frees the chip buffer |
| `sampleLength/Period/Volume/Cycles` | values | read-only |

The sample must stay alive while playing. The period is
`3579545 / sample_rate`; volume is 0..64.

## 19. Joystick (`std.amiga.input`)

`readJoystick(port)` reads `JOY0DAT` (port 0) or `JOY1DAT` (port 1) and the
CIAAPRA fire bit. The result is `JOY_UP=1`, `JOY_DOWN=2`, `JOY_LEFT=4`,
`JOY_RIGHT=8`, `JOY_FIRE=16`. The decode is the 1.3 Hardware Reference table:
right bit 1, left bit 9, down bit 1 XOR bit 0, up bit 9 XOR bit 8; fire is
active low on CIAAPRA bit 6 (port 0) or 7 (port 1).

## 20. Timer (`std.amiga.timer`)

`openTimer()` opens `timer.device` `UNIT_VBLANK`; `timerWait(timer, ticks)`
sends `TR_ADDREQUEST` and waits. The request converts PAL 50 Hz ticks to
seconds/microseconds (`tv_secs = ticks / 50`, `tv_micro = (ticks % 50) *
20000`). The close releases the device.

## 21. Text and fonts (`std.amiga.text`)

The module wraps the 1.3 `graphics.library` font and text calls and the
optional `diskfont.library`. A `Font` owns a `TextFont` when it came from
`openFont`/`openDiskFont`; `defaultFont` borrows `GfxBase->DefaultFont` and is
never closed. The ROM `topaz.font` opens with `openFont` and measures
`Clorraine layers` at 128x8 on the A500. A bare ADF has no `FONTS:` volume, so
`openDiskFont` returns `null`; a caller should fall back.

| API | Returns | Rule |
|---|---|---|
| `openFont(name: cstring, size: u16, style: u8)` | `Font?` | `graphics.library/OpenFont`; ROM fonts work without a `FONTS:` disk |
| `openDiskFont(name, size, style)` | `Font?` | `diskfont.library/OpenDiskFont`; `null` when no disk font is available |
| `defaultFont()` | `Font?` | borrowed `GfxBase->DefaultFont`; never closed |
| `freeFont(take font)` | `void` | closes owned fonts; the default font is left open |
| `textWidth(borrow font, const text: u8[])` | `i32` | `TextLength` on a temporary RastPort |
| `textHeight(const font)` | `i32` | the font's `tf_YSize` |
| `drawText(borrow screen, borrow font, x, y, pen, const text)` | `void` | sets the screen RastPort font and pen, then `Move`/`Text`, and restores the previous font |
| `windowText(borrow window, borrow font, x, y, pen, const text)` | `void` | sets the window RastPort font and pen, then `Move`/`Text`, and restores the previous font; coordinates are inner-window pixels |

The text bytes are the Amiga 8-bit glyph codes; there is no Unicode translation
in v1. `drawText`, `windowText`, and `layerText` restore the RastPort's previous
font after the draw, so the font handle may be freed before the screen, window,
or layer is released. The acceptance ADF has no `LIBS:diskfont.library` or
`FONTS:` volume; the demo's `openDiskFont` call returns `null` and the
disk-font row is deferred.

## 22. Layers (`std.amiga.layers`)

The module wraps `layers.library` on a caller-owned custom display bitmap.
Layers are not created on Intuition screens; windows are the supported
Intuition path. `newLayer` owns a `Layer_Info` created by `NewLayerInfo` and
calls `CreateUpfrontLayer(..., LAYERSIMPLE, NULL)`. It does not write
`Layer_Info.bounds`; doing so corrupts the 1.3 structure.

| API | Returns | Rule |
|---|---|---|
| `newLayer(borrow bitmap, x0, y0, x1, y1)` | `Layer?` | inclusive rectangle inside the bitmap (`0 <= x0 <= x1 < width`, `0 <= y0 <= y1 < height`); free before the bitmap |
| `freeLayer(take layer)` | `void` | `DeleteLayer` + `DisposeLayerInfo` |
| `layerMove(borrow layer, dx, dy)` / `layerSize(borrow layer, dw, dh)` | `bool` | `MoveLayer` / `SizeLayer`; false when the new bounds leave the bitmap |
| `layerClear(borrow layer, pen)` | `void` | `SetRast` on the layer RastPort |
| `layerFill(borrow layer, x1, y1, x2, y2, pen)` | `void` | `RectFill`; coordinates are layer-local |
| `layerText(borrow layer, borrow font, x, y, pen, const text)` | `void` | `SetFont`/`SetAPen`/`Move`/`Text`; coordinates are layer-local |
| `layerBeginUpdate` / `layerEndUpdate` | `bool` / `void` | `BeginUpdate` reports whether drawing may proceed; `EndUpdate(... TRUE)` completes the redraw |

The layer RastPort uses layer-local coordinates: (0,0) is the layer's
upper-left, and the layers library adds the layer origin and clips every draw
to the layer bounds. A simple layer is observed to clear its area to pen 0 when
it is created, so draw its background explicitly. Each layer owns a separate
`Layer_Info`: layers do not stack, occlude, or restore one another, and a
simple-refresh layer leaves stale pixels when moved or resized; repaint the
background after such an operation.

## 23. DOS (`std.amiga.dos`)

The module wraps the 1.3 `dos.library` path, lock, directory, and volume calls.
The `File` handle stays the existing libc-backed stream, so `seekFile`,
`filePosition`, and `fileLength` use the stream's `fseek`/`ftell`; locks and
path/volume information use `dos.library`.

| API | Returns | Rule |
|---|---|---|
| `seekFile(borrow f, offset, whence)` | `bool` | `SEEK_SET`/`SEEK_CUR`/`SEEK_END`; unknown `whence` is false |
| `filePosition(borrow f)` / `fileLength(borrow f)` | `i32` | -1 on failure; `fileLength` restores the position |
| `lockPath(path)` / `unlockPath(take lock)` | `Lock?` / `void` | shared `dos.library` lock; null-safe release |
| `pathInfo(path)` | `PathInfo?` | captures one `FileInfoBlock`; the lock is released before return |
| `pathInfoIsDir(const info)` / `pathInfoSize(const info)` | `bool` / `i32` | reads `fib_DirEntryType` / `fib_Size` |
| `pathInfoName(const info, borrow buf)` | `i32` | copies `min(name, buf.len)` bytes; no NUL, no overflow |
| `volumeInfo(path)` | `VolumeInfo?` | captures `InfoData` and the `DeviceNode`/`Examine` name |
| `volumeName(const info, borrow buf)` | `i32` | bounded name copy |
| `volumeFreeBytes` / `volumeUsedBytes` | `i32` | blocks × bytes per block, clamped to `LONG_MAX` |
| `makeDir(path)` / `deleteFile(path)` / `renameFile(old, new)` | `bool` | `CreateDir` / `DeleteFile` / `Rename` |
| `setCurrentDir(path)` | `bool` | `CurrentDir`; the old lock may be zero at the boot root |
| `currentDirName(borrow buf)` / `parentDirName(borrow buf)` | `i32` | bounded copies through `Lock("")` and `ParentDir` |

Assigns are resolved by DOS inside `Lock`/`Open`; enumerating the assign list
needs `LockDosList` (2.0+) and is outside the 1.3 row. `ReadArgs` and
`datatypes.library` are likewise 2.0+/3.0+ and are deferred.

## 24. IFF/ILBM (`std.iff`)

`std.iff` is pure Clorraine and runs on the host as well as the Amiga profiles.
`ilbmInfo` validates `FORM`/`ILBM`, walks the chunks with subtraction-based
window checks, and records `BMHD`, `CMAP`, and the first `BODY`. `ilbmDecode`
clears the destination, decodes uncompressed or ByteRun1 row-interleaved planes
into one byte per pixel, and returns false on a short buffer, truncated body,
bad RLE stream, or inconsistent info. `ilbmPalette` copies whole RGB triplets;
`ilbmPixelHash` is FNV-1a over the chunky pixels.

| API | Returns | Rule |
|---|---|---|
| `ilbmInfo(const data: u8[])` | `IlbmInfo` | `ok` requires `BMHD` + one non-empty `BODY`; width/height 1..1024, depth 1..8, masking 0, compression 0/1 |
| `ilbmDecode(const data, const info, borrow pixels)` | `bool` | clears and overwrites `width * height` pixels; never traps on bad input |
| `ilbmPalette(const data, const info, borrow palette)` | `u16` | whole triplets only; missing `CMAP` returns 0 |
| `ilbmPixelHash(const pixels)` | `u32` | FNV-1a over the decoded bytes |

The accepted subset is the common ILBM shape. HAM, EHB, masking, multi-`BODY`,
and images deeper than eight planes are outside it.

## 25. Image loader and viewer (`std.amiga.ilbm`)

`std.amiga.ilbm.loadIlbm(path, borrow palette)` opens the file, reads at most
128 KB, parses and decodes it, creates a bitmap through `std.amiga.video`, and
copies the chunky indices with `bitmapFromIndices`. It returns an `IlbmImage`
with an `ok` gate, the owned bitmap, the copied color count, the pixel hash, and
the geometry. An image wider than 320 or taller than 256 is rejected to stay
inside the A500 display and chip-memory budget.

`std.amiga.video.bitmapFromIndices(borrow bitmap, const indices)` clears the
planes and sets one bit per plane from each chunky index; a short view leaves
the bitmap cleared.

`examples/amiga/imageviewer.clo` runs the DOS self-test, loads
`DF0:image.ilbm`, applies its 16 colors, draws the caption layer, and prints
`img: ok 320x200x4 hash <N> dos 255 vol 1` on the A500.

## 26. Diagnostics

| Code | Trigger |
|---|---|
| E10010 | a nullable allocation result used before a null check |
| E10022 | moving or freeing an owner while a view of it is live |
| E10031 | mutating an owner while a shared view of it is live |
| E10604 | volatile access outside `unsafe` (inside a wrapper, not application code) |
| E11211 | runtime fault: an index or span offset out of range |

All message text lives in [Ch 14](14-diagnostics.md).
