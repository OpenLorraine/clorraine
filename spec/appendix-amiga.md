# Appendix A - Amiga Platform Notes

> **Status:** stable (v1.0; v1.1 adds the A500/Kickstart 1.3 profile and its classic Intuition surface)
> **Depends on:** Ch 12 (contracts), Ch 14 (diagnostics), Ch 15 (profiles), Ch 17 (codegen) *(informational)*
> **Grammar:** none; the surface is the frozen [grammar.ebnf.md](grammar.ebnf.md)
> **Ledger:** [decisions.md](decisions.md): C1-C4, F7, T6, T7, X1-X6, L5, P14, P15; validated by [findings](../docs/findings-phase0.md)

This appendix is the practical companion to the `hosted-amiga` profiles
([Ch 15](15-profiles.md)). It names the AmigaOS API the v1 standard library
wraps, the boundary types, and the rules for hardware, interrupts, and
asynchronous work. Application code stays above this layer: the library holds
the `unsafe` code and the raw addresses.

The `hosted-amiga-13` profile targets a stock A500 (68000, OCS, 512 KB chip,
Kickstart 1.3). It provides the pre-2.0 subset documented here: screens,
windows, gadgets, menus, IDCMP events, requesters, the pointer, drawing, DOS
delay, VBlank interrupts, and custom registers. The same classic Intuition
surface compiles on `hosted-amiga`; the 2.0+ additions (gadtools, ASL
requesters, and BOOPSI) are not part of this surface and are deferred. The host
profile has no AmigaOS Intuition and rejects the module with `E11200`. The
KS1.3 A500 gate is the runtime proof; the `hosted-amiga` profile is
compile-verified. The classic Exec surface (private ports, worker tasks,
libraries, and generic devices) is available on both hosted profiles; the
KS1.3 A500 Exec gate is its runtime proof. The 2.0+ Exec additions (signal
semaphores, hooks, and the utility library) are deferred.

## 1. API surface for v1

The v1 library wraps the AmigaOS libraries the profiles provide. Each area
sits behind ownership contracts (X3; [Ch 12](12-ffi-contracts.md)), so a
wrapper, not the caller, holds the FFI declaration.

| Library | Area | Covered in v1 | State behind it |
|---|---|---|---|
| `exec.library` | memory | allocate and free (`AllocVec`/`FreeVec`); pools later | `Buffer`, `Array<T>` |
| `exec.library` | tasks | create, join, and remove a worker task (`CreateTask`/`RemTask`) | `handle Task` |
| `exec.library` | libraries | open, close, and read the version (`OpenLibrary`/`CloseLibrary`) | `handle Library` |
| `exec.library` | ports | create and delete private message ports | `handle MsgPort` |
| `exec.library` | signals | wait for and set signals (`Wait`/`SetSignal`) | event loop (§6) |
| `exec.library` | devices | open, close, and run `IOStdReq` requests (`DoIO`/`SendIO`/`WaitIO`) | `handle Device` |
| `dos.library` | files | open, close, read, write, seek, position, and length | `handle File` |
| `dos.library` | locks | lock and unlock a path, inspect it, create/delete/rename entries, current/parent directory | `handle Lock`, `PathInfo` |
| `dos.library` | volumes | volume name, free bytes, used bytes | `VolumeInfo` |
| `dos.library` | output | write to standard output | `println` |
| `iff` | assets | pure IFF/ILBM parser, ByteRun1, palette, chunky hash | `IlbmInfo` |
| `ilbm` / `graphics` | image loading | read an ILBM into a `Bitmap` and palette | `IlbmImage`, `bitmapFromIndices` |
| `intuition` | screens, windows, gadgets, menus, events | open and close a custom screen or window; add classic buttons, string gadgets, and menus; wait for or poll IDCMP events; requesters, alerts, and the pointer | `handle Screen`, `handle Window`, `WindowEvent` |
| `graphics` | drawing | set colors and fill, line, and clear a screen or window | `handle Screen`, `handle Window` |
| `graphics` | native video | chip memory, bitmaps, blitter copies, copper list, double-buffered `LoadView` display, 4-bit palette | `ChipBuffer`, `Bitmap`, `Copper`, `Display` |
| `graphics` | hardware sprites | eight OCS channels, pixels/move/show/hide, attached pairs, collision, animation | `Sprite`, `SpriteAnimation` |
| `graphics/text` | fonts and text | ROM and disk fonts, measurement, screen and window drawing | `Font` |
| `layers` | clipped layers | front layers, clipping, move/size, damage updates | `Layer` |

The library is written in copy, not in the surface. Application code imports
wrappers and never writes an `ffi` block (X6).

The `std.amiga.intuition` surface is one classic Intuition API shared by both
hosted profiles: a custom screen, a window on that screen, classic gadgets and
menus, the IDCMP event loop, requesters, and the built-in pointer. The
`RastPort` stays inside the shim. The screen's `title` is a `cstring` the
caller must keep alive for the screen's lifetime; window, gadget, and menu text
is copied.

```clorraine
import { Screen, openScreen } from std.amiga.intuition;
import { setColor, fill, clear } from std.amiga.graphics;
import { String, stringOf, toCString } from std.text;

func draw(): i32 {
    let title: String = stringOf("Clorraine graphics");
    let screen: Screen? = openScreen(320, 256, 4, toCString(title));
    if (screen == null) { return 1; }
    setColor(screen, 1, 255, 32, 32);
    clear(screen, 0);
    fill(screen, 10, 10, 100, 100, 1);
    return 0;   // the screen drops here, closing it on real AmigaOS
}
```

### 1.1 Native video

`std.amiga.video` is the low-level path for demos that need the whole display.
It exposes four opaque handles: `ChipBuffer` (chip memory), `Bitmap`
(displayable planes), `Copper` (a 1.3 user copper list builder), and `Display`
(two compiled views over two caller-owned bitmaps). The display uses the
`InitView` → `InitVPort` → `MakeVPort` → `MrgCop` → `LoadView` pipeline. The
first flip compiles both views and saves `GfxBase->ActiView`; the close restores
that view, waits two frames, and frees the copper lists. Intuition is not
involved, so the demo owns the whole screen while it runs.

The two frame bitmaps are caller-owned and must outlive the display. Declare
them before the display so the generated drops release the display first, and
do not return a `Display` from a function whose locals own the bitmaps. A
bitmap is created with `AllocRaster` planes, and its palette is a `GetColorMap`
table set before the first flip. Animated colors use the copper, not runtime
palette changes.

```clorraine
import { Bitmap, Display, newBitmap, freeBitmap, openDisplay, displayFlip,
         freeDisplay } from std.amiga.video;

func demo(): i32 {
    let front: Bitmap? = newBitmap(320, 256, 4);
    if (front == null) { return 1; }
    let back: Bitmap? = newBitmap(320, 256, 4);
    if (back == null) { return 1; }
    let display: Display? = openDisplay(front, back);
    if (display == null) { return 1; }
    displayFlip(display);       // builds both views and shows `front`
    displayFlip(display);       // shows `back`
    freeDisplay(display);       // restores the system view
    freeBitmap(back);
    freeBitmap(front);
    return 0;
}
```

The blitter wrappers use `BltBitMap` for a plain copy and
`BltMaskBitMapRastPort` for a cookie-cut image. The frame checksum is FNV-1a
over the plane bytes, so a deterministic drawing sequence has a stable value.

### 1.2 Hardware sprites

`std.amiga.sprite` uses the `graphics.library` simple-sprite machine
(`GetSprite`, `ChangeSprite`, `MoveSprite`, `FreeSprite`). The 1.3 API attaches
sprites to a `ViewPort`; the supported path is an Intuition `Screen`, whose
viewport already has a compiled display list. `spriteAttachScreen` sets the
`SPRITES` view flag, adds the sprite list, and calls `RethinkDisplay`.
`spriteDetach` clears the back-pointer; the supported lifetime is hide,
detach, close the screen, then free the sprite. The shim also clears a channel
when its screen closes, so freeing after a close is safe even without the
explicit detach.

The eight channels are global. Channel 0 is normally the system pointer, so a
program should use another channel. Two adjacent channels form an attached
pair: the even channel carries the low two color bits, the odd channel carries
the high two, and the odd control word has the ATTACH bit. The pair uses sprite
palette registers 16..31; a single channel uses the three registers starting at
`16 + ((channel & 6) << 1) + 1`.

`enableCollisions` writes `CLXCON = 0xF000`, which includes the odd channel of
each pair. `collisions` reads and clears `CLXDAT`; sprite-sprite groups are bits
9..14 and the playfield bits are 1..8. Hardware collision is edge-triggered, so
clear the register after moving the sprites if only the final overlap matters.

### 1.3 Audio and input

`std.amiga.audio` wraps `audio.device` with a private reply port and one
`IOAudio` block per allocated channel. A `Sample` is a chip-memory copy of
signed 8-bit data with a period, volume, and cycle count; `CMD_WRITE` with
`ADIOF_PERVOL` starts it, `ADCMD_PERVOL` changes it, and `ADCMD_FINISH` aborts
it. The four OCS channels are allocated by the device, not hardcoded.

`std.amiga.input` reads the 1.3 gameport registers: `JOY0DAT`/`JOY1DAT` for the
directions and CIAAPRA bits 6/7 for the active-low fire buttons. The decode is
the Hardware Reference XOR table; port 0 is the left controller pair.

`std.amiga.timer` opens `timer.device` `UNIT_VBLANK`. The device's `TimeVal` is
always seconds/microseconds, even for the VBlank unit, so one PAL tick is
20,000 microseconds.

### 1.4 Text, fonts, and layers

`std.amiga.text` wraps the 1.3 font and text calls. `openFont` opens the ROM
`topaz.font` without a `FONTS:` disk; `openDiskFont` needs a disk font and
returns `null` on the bare A500 acceptance image. `defaultFont` borrows
`GfxBase->DefaultFont` and is never closed. `textWidth` measures with a
temporary `RastPort`, `textHeight` returns `tf_YSize`, and `drawText` draws on
an Intuition screen's `RastPort`.

`std.amiga.layers` creates front layers on a **custom display bitmap**, not on
an Intuition screen. `newLayer(bitmap, x0, y0, x1, y1)` owns a `Layer_Info` and
uses `CreateUpfrontLayer` with `LAYERSIMPLE`; the rectangle must lie inside the
bitmap. Do not write `Layer_Info.bounds`; the 1.3 structure is initialized by
`NewLayerInfo`, and the extra write crashes `CreateUpfrontLayer`. A layer's
`RastPort` uses layer-local coordinates, and the layers library adds the layer
origin and clips every draw. `layerMove`/`layerSize` reject bounds outside the
bitmap. Each layer has its own `Layer_Info`, so layers do not stack or occlude
one another; a simple layer leaves stale pixels when moved or resized. Free the
layer before its bitmap. `drawText`/`layerText` restore the RastPort font after
the draw, so a font may be freed before the screen or layer.

### 1.5 DOS and assets

`std.amiga.dos` adds file positioning (`seekFile`, `filePosition`,
`fileLength`), shared locks, `PathInfo` and `VolumeInfo` handles, directory and
file mutation, and current/parent directory names. The `File` handle remains the
libc-backed stream; locks and path/volume information use `dos.library`. Name
copies are bounded by the caller buffer. Assigns resolve through `Lock`/`Open`;
assign-list enumeration, `ReadArgs`, and `datatypes.library` are 2.0+/3.0+ and
deferred.

`std.iff` is a pure parser for the common ILBM shape: one `FORM ILBM`, one
`BMHD`, an optional `CMAP`, and one row-interleaved `BODY` with no compression
or ByteRun1. It rejects wrapped lengths, truncated chunks, zero-length or
multiple bodies, masking, and bad geometry without trapping. `ilbmPixelHash`
gives the host tests and the A500 report the same FNV-1a value.

`std.amiga.ilbm.loadIlbm` reads at most 128 KB, caps the image at 320x256, builds
the bitmap, and fills the caller's palette. The acceptance viewer runs a DOS
self-test, shows the committed 320x200x4 fixture, applies its 16 colors, draws a
ROM-font caption layer, and prints `img: ok 320x200x4 hash <N> dos 255 vol 1`.
The A500 smoke builds it in release mode so the executable and chip allocations
fit the 512 KB machine.

The 2.0+ regions/damage API and the 2.0+ Intuition additions (gadtools, ASL
requesters, and BOOPSI) are not part of this surface. `SetRGB32`, runtime
palette animation after the first flip, and palettes beyond four bits are out
of scope for v1.

## 2. Amiga boundary types

AmigaOS talks in untyped pointers. Clorraine keeps them opaque until a header
importer can produce typed views; the wrapper layer is the only place the raw
spellings appear.

| Amiga type | Clorraine | Notes |
|---|---|---|
| `STRPTR` (`char *`) | `cstring` | a raw zero-terminated pointer for FFI; read it into a `str` view to use text |
| `BPTR` file handle | `handle File` | a handle; the zero value means failure and maps to `null` |
| `BPTR` lock | `handle Lock` | same rule |
| `APTR` | `raw T` or `handle Memory` | opaque until typed by a wrapper |
| `struct Library *` | `handle Library` | opaque; the delete func is `CloseLibrary` |
| `BSTR` (BCPL string) | none | path helpers (`pathToBSTR`-style) live in the runtime shim, not in the language |

Inside the library, the wrappers declare the handles and the raw calls once:

```clorraine
// Inside the standard library (application code never writes this).
handle File    freedBy Close;
handle MsgPort freedBy cl_delete_msg_port;
handle Message freedBy cl_reply_msg;   // a fetched message is owned until replied

ffi dos {
    func Open(name: cstring, mode: i32): File?;   // a BPTR of 0 becomes null
    func Close(take f: File): void;
    func Write(borrow f: File, buf: u8[], len: i32): i32;
    func IoErr(): u32;
}

ffi exec {
    func cl_get_msg(borrow port: MsgPort): Message?;
    func cl_reply_msg(take msg: Message): void;
}
```

The sketch is illustrative; the delivered wrapper surface, including message creation and
`NewMessage`/`Message` ownership, is in
[appendix-stdlib.md](appendix-stdlib.md) §13.

`take File` transfers ownership to `Close`; the caller's binding is
invalidated. `File?` says the open may fail, and the caller must check for
`null` before use (E10010). `take Message` returns a fetched message to its
sender; the handle's `freedBy` deleter is `cl_reply_msg`. There are no contract
clause words; the mode and the `?` carry the meaning (X3).

## 3. Error reporting

AmigaOS reports failure with a return code plus `IoErr()`. Clorraine has no
exceptions (E2), so the wrapper layer turns that pair into a value: a `T?`
when "empty" is the whole story, or a **status struct** when detail matters
(E1, E6). The caller checks the status.

| Failure shape | Wrapper returns | Use when |
|---|---|---|
| no useful detail | `T?` | "it failed" is enough |
| a code to inspect | status struct | the caller must react to the code |
| a value written out | `borrow` out-parameter | the call fills caller storage |

```clorraine
struct IoResult {
    ok: bool;
    error: u32;      // the IoErr()-style code
}

// A stdlib wrapper: it turns IoErr() into a value, never an exception (E2).
export func readInto(f: File, borrow buf: u8[]): IoResult {
    let n = read(f, buf);
    let r = IoResult { ok: false, error: 0 };
    if (n < 0) {
        r.error = IoErr();
        return r;
    }
    r.ok = true;
    return r;
}
```

The caller chooses the shape it needs; the library does not hide a failure
behind a panic.

## 4. Custom chips (hardware access)

The custom chips live at fixed addresses. The language has three controlled
mechanisms for touching them (T7); every one of them is inside `unsafe` (T6).

| Mechanism | Form | Rule |
|---|---|---|
| volatile overlay struct | `struct Regs { dmacon: volatile u16; ... }` | the struct is an overlay type; access is only through a `raw` pointer (T11) |
| six hardware intrinsics | `readVolatile8/16/32`, `writeVolatile8/16/32` | the audited primitives; no folding, caching, or reordering |
| register base constant | `const CUSTOM_BASE: raw Regs = raw Regs(0x00DFF000);` | a constant `raw` cast of an address; no re-deriving the base in every func |

An overlay type cannot be a local, copied, dropped, borrowed, or passed by
value (E10207). You reach it through a `raw` pointer, and the field access
implies the dereference:

```clorraine
// Wrapper-internal: the standard library owns the register base (T7).
struct CustomRegs {
    dmacon: volatile u16;
    cop1lc: volatile u32;
}

const CUSTOM_BASE: raw CustomRegs = raw CustomRegs(0x00DFF000);
const COP_DMA: u16 = 0x0080;

export func enableCopper(): void {
    unsafe {
        CUSTOM_BASE.dmacon = CUSTOM_BASE.dmacon | COP_DMA;
    }
}
```

The six intrinsics are the offset-based form, used when a wrapper walks a
register block:

```clorraine
export func setColor(index: u32, value: u16): void {
    let base = raw u16(0x00DFF180);
    unsafe {
        writeVolatile16(base + index, value);   // pointer arithmetic needs unsafe
    }
}
```

The standard-library per-chip wrappers hold the register bases. Application
code calls the wrappers and never writes a hardcoded chip address:

```clorraine
import { enableCopper } from std.amiga.custom;

enableCopper();
```

## 5. Interrupts and hooks

The compiler has no `interrupt` declaration. Interrupts and hooks go through a
standard-library dispatcher (F7): a C-side qualified handler plus a token-based
add/remove pair. A handler is a plain `func(): void` value that the dispatcher
calls (L5).

| Rule | Statement |
|---|---|
| IH-1 | Register and unregister a handler with the dispatcher (`addHandler` for VBlank, `addPortHandler` for `INTB_PORTS`, `removeHandler` for either); the wrapper returns a `HandlerToken` |
| IH-2 | A handler is allocation-free, drop-free, and panic-free; it must not `panic` (E4). This is an audited convention in v1; effect-marker enforcement is a named Phase 2 follow-up |
| IH-3 | State shared with a handler is a `volatile` module-level `let` (T7); a polling loop would otherwise read a stale value |
| IH-4 | `stores` and `transfers` contracts are deferred to v2 (E90003) |
| IH-5 | C-hook trampolines for capturing lambdas arrive in v1.1 (L5); a capture-less lambda already matches `func(...)` |

```clorraine
module Demo.Timer;

import { addHandler, removeHandler, HandlerToken } from std.amiga.interrupt;
import { Status } from std.mem;

volatile let ticks: u32 = 0;

func onTick(): void {
    ticks = ticks + 1;      // shared with the interrupt handler
}

export func start(): Status {
    let t: HandlerToken = addHandler(onTick);   // a bare func value; no capture, no allocation
    return Status { ok: true, code: 0 };        // the wrapper reports failure in the status struct
}

export func stop(t: HandlerToken): void {
    removeHandler(t);
}
```

The dispatcher's C-side handler is the only code with the machine calling
convention. Application handlers stay ordinary `func`s and keep the
allocation-free, drop-free, panic-free contract.

## 6. Asynchronous work

The language is single-threaded (C1). Concurrency is the AmigaOS task model,
reached through FFI wrappers; the language itself has no async/await in v1
(C3) and no library-level runtime (C4). Asynchronous work uses three OS
mechanisms:

| Mechanism | What it does | Event-loop role |
|---|---|---|
| signals | a bit per event source | wake the loop when any source is ready |
| message ports | queue messages to a port | deliver work items |
| IO requests | carry a read or write to completion | report finished IO |

The pattern is a single loop that waits for signals and drains the ready
sources. `for (;;)` is the infinite loop (B2):

```clorraine
module Demo.EventLoop;

import { MsgPort, signals, waitSignals, getMsg, replyMsg } from std.amiga.exec;

export func run(port: MsgPort): void {
    for (;;) {
        let sig = waitSignals(signals(port));
        if ((sig & signals(port)) != 0) {
            let msg = getMsg(port);
            while (msg != null) {
                replyMsg(msg);          // consumes msg; the binding is then free
                msg = getMsg(port);     // re-initialize before the next check
            }
        }
    }
}
```

`getMsg` returns a nullable `Message?`; `replyMsg` takes `take Message`, so
the binding is moved and re-initialized for the next turn (O3). The library
wrappers hold the FFI calls and their contracts.

Worker tasks are plain Exec tasks, not DOS processes; their entry restrictions
are in [appendix-stdlib.md](appendix-stdlib.md) §13.1 and
[known limitations](../docs/known-limitations.md). `console.device` is the one
device with a typed open helper because it is bound to an Intuition window; see
[appendix-stdlib.md](appendix-stdlib.md) §13.4.

## 7. ROM and runner policy

Kickstart ROMs are copyrighted, so the SDK image never contains a ROM. Tests
use the emulator's built-in AROS replacement ROM; real Kickstart ROMs are
mounted from the host at run time. See
[the SDK readme](../docker/README.md) for the mount layout and configuration.

| Runner | Needs a ROM file | Use |
|---|---|---|
| `vamos` | no | fast CLI tests on the host |
| FS-UAE directory drive | no (AROS built-in) | full target behavior, fast rebuild |
| FS-UAE bootable ADF | no (AROS built-in) | bootable floppy; CI image path |
| FS-UAE with real Kickstart | yes (mounted) | hardware-accurate runs only |

Phase 0 confirmed that the `vamos` output matches the FS-UAE output for both
boot media, so the fast runner and the real target agree
([findings](../docs/findings-phase0.md)).

## 8. Toolchain and build matrix

The `hosted-amiga` toolchain facts from [Ch 15](15-profiles.md) section 3:

| Element | Value | Note |
|---|---|---|
| Default CPU | `-m68000` | maximum compatibility (P14) |
| Opt-in CPU | `-m68020` | per project |
| C runtime | `-mcrt=nix20` | libnix |
| Libraries | `-lamiga` | the stub archive opens only the libraries in use |
| Float ABI | soft-float | GCC and vbcc ABIs planned; not yet verified (P15) |
| FPU | opt-in `-m68881` | the fast floating-point leg, planned; not yet verified (P15) |

The exact compiler commands are normative in [Ch 17](17-codegen.md)
section 13. The validated matrix - GCC 68000 on `vamos` and both FS-UAE media,
GCC 68020 build-only, and vbcc on `vamos` - is recorded in
[the Phase 0 findings](../docs/findings-phase0.md). No float code was exercised,
so the float ABI legs remain unverified.

## 9. Diagnostics in this appendix

| Code | Trigger |
|---|---|
| E10010 | a nullable handle used before a `!= null` check |
| E10100 | an uncontracted extern call outside `unsafe` |
| E10207 | an overlay type used by value instead of through a `raw` pointer |
| E10604 | volatile intrinsic or overlay access outside `unsafe` |
| E90003 | a request for a `stores` or `transfers` contract (v2) |
| E11200 | a construct the `hosted-amiga` profile cannot lower |

Message text lives in [Ch 14](14-diagnostics.md).
