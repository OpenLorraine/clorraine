# Appendix B - Standard Library (v1)

> **Status:** stable (v1.0)
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
windows, text, sprites, and runtime palettes beyond four bits are outside v1.
The native-video path in §15 is the bitmap/blitter/copper alternative.

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
| `setColor(borrow screen, pen: u8, r: u8, g: u8, b: u8)` | `void` | maps 8-bit components to the Amiga 4-bit palette (`SetRGB4`); `pen` is valid in `0..(1 << depth)` |
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

## 13. Message ports (`std.amiga.exec`)

The module owns private `MsgPort` handles and two move-only message handles. It is available only
on `hosted-amiga`; the host, `hosted-amiga-13`, and freestanding profiles reject it with `E11200`. The event-loop
shape is in [appendix-amiga.md](appendix-amiga.md) §6.

| API | Returns | Ownership and rules |
|---|---|---|
| `createMsgPort()` | `MsgPort?` | private port; `null` on failure; delete it after draining |
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

The text bytes are the Amiga 8-bit glyph codes; there is no Unicode translation
in v1. `drawText` and `layerText` restore the RastPort's previous font after
the draw, so the font handle may be freed before the screen or layer is
released. The acceptance ADF has no `LIBS:diskfont.library` or `FONTS:`
volume; the demo's `openDiskFont` call returns `null` and the disk-font row is
deferred.

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
