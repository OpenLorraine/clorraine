# Appendix A - Amiga Platform Notes

> **Status:** draft (Phase C - rewritten against the ledger and the grammar)
> **Depends on:** Ch 12 (contracts), Ch 14 (diagnostics), Ch 15 (profiles), Ch 17 (codegen) *(informational)*
> **Grammar:** none; the surface is the frozen [grammar.ebnf.md](grammar.ebnf.md)
> **Ledger:** [decisions.md](decisions.md): C1-C4, F7, T6, T7, X1-X6, L5, P14, P15; validated by [findings](../docs/findings-phase0.md)

This appendix is the practical companion to the `hosted-amiga` profile
([Ch 15](15-profiles.md)). It names the AmigaOS API the v1 standard library
wraps, the boundary types, and the rules for hardware, interrupts, and
asynchronous work. Application code stays above this layer: the library holds
the `unsafe` code and the raw addresses.

## 1. API surface for v1

The v1 library wraps two AmigaOS libraries. Each area sits behind ownership
contracts (X3; [Ch 12](12-ffi-contracts.md)), so a wrapper, not the caller,
holds the FFI declaration.

| Library | Area | Covered in v1 | State behind it |
|---|---|---|---|
| `exec.library` | memory | allocate and free (`AllocVec`/`FreeVec`); pools later | `Buffer`, `Array<T>` |
| `exec.library` | libraries | open and close (`OpenLibrary`/`CloseLibrary`) | `handle Library` |
| `exec.library` | ports | create and delete message ports | `handle MsgPort` |
| `exec.library` | signals | wait for and set signals (`Wait`/`SetSignal`) | event loop (§6) |
| `dos.library` | files | open, close, read, and write; `Seek` planned, not yet wrapped | `handle File` |
| `dos.library` | locks | lock and unlock a path (planned; not yet wrapped) | `handle Lock` (planned) |
| `dos.library` | output | write to standard output (planned; print helpers not yet wrapped) | `println` (planned) |
| `intuition` / `graphics` | custom screen, drawing | open and close a custom screen; set colors and fill, line, and clear | `handle Screen` |

The library is written in copy, not in the surface. Application code imports
wrappers and never writes an `ffi` block (X6).

The `intuition`/`graphics` surface is the smallest that draws on a real screen:
a custom screen owned by the caller and four drawing calls, with no windows. The
`RastPort` stays inside the shim, and the screen's `title` is a `cstring` the
caller must keep alive for the screen's lifetime.

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

Windows and their event loop, text and fonts, bitmaps and blit, sprites and
copper, `SetRGB32`, and palettes beyond four bits are out of scope for v1.

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
handle Message freedBy ReplyMsg;   // a fetched message is owned until replied

ffi dos {
    func Open(name: cstring, mode: i32): File?;   // a BPTR of 0 becomes null
    func Close(take f: File): void;
    func Write(borrow f: File, buf: u8[], len: i32): i32;
    func IoErr(): u32;
}

ffi exec {
    func GetMsg(port: MsgPort): Message?;
    func ReplyMsg(take msg: Message): void;
}
```

`take File` transfers ownership to `Close`; the caller's binding is
invalidated. `File?` says the open may fail, and the caller must check for
`null` before use (E10010). `take Message` returns a fetched message to its
sender; the handle's `freedBy` deleter is `ReplyMsg`. There are no contract
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
| IH-1 | Register and unregister a handler with the dispatcher (`addHandler`, `removeHandler`); the wrapper returns a `HandlerToken` |
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
