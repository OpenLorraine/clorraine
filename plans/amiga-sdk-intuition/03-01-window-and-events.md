# Window and Events: Amiga SDK Bundle 6

> **Document**: 03-01-window-and-events.md
> **Parent**: [Index](00-index.md)
> **Covers**: R1–R3, R11–R12; AR-8–AR-12, AR-14, AR-17, AR-23

## Overview

This component adds the `Window` handle, its custom-screen lifecycle, the
window-relative drawing and text calls, and the IDCMP event loop. It is the base
for gadgets (`03-02`) and requesters/pointers (`03-03`). The window owns every
allocation that Intuition does not own itself: the wrapper, the copied title,
gadget nodes, menu nodes, string buffers, and the pointer image.

## Architecture

### Current Architecture

`std.amiga.intuition` owns only the custom `Screen` handle
(`stdlib/std.amiga.intuition.clo:14-31`). `std.amiga.graphics` and
`std.amiga.text` take a borrowed `Screen` and draw into `screen->RastPort`
(`compiler/runtime/shim_hosted_amiga.c:1099-1150`, `:3440-3458`). No window
struct, IDCMP port, or event type exists.

### Proposed Changes

Add a private `cl_window` wrapper in the C shim and a public `Window` handle in
`std.amiga.intuition`. The wrapper has this shape:

```c
typedef struct cl_window_event {
    LONG kind;        /* EVENT_* */
    LONG code;        /* raw IntuiMessage.Code */
    UWORD qualifier;  /* raw Intuition qualifier */
    LONG mouse_x;
    LONG mouse_y;
    LONG gadget;      /* user id, 0 when none */
    LONG menu;        /* user id, 0 when none */
} cl_window_event;

typedef struct cl_window {
    struct Window *window;
    struct cl_gadget_node *gadgets;
    struct cl_menu_node *menus;
    char *title;
    UWORD *pointer;             /* 16x16, two planes, chip memory */
    int pointer_shown;
    cl_window_event last_event;
} cl_window;
```

`cl_window` is the `Window` contract handle. Every public shim function takes
`void *` and casts it back. A null handle is a no-op or a `null`/`false` result.

## Implementation Details

### New Types and Constants

```clorraine
module std.amiga.intuition;

export handle Window freedBy cl_close_window;

/// One IDCMP event copied out of Intuition.
export struct WindowEvent {
    kind: i32;        // EVENT_*
    code: i32;        // raw Intuition code
    qualifier: u16;
    mouse_x: i32;     // inner-window coordinate
    mouse_y: i32;     // inner-window coordinate
    gadget: i32;      // user gadget id, 0 when none
    menu: i32;        // user menu-item id, 0 when none
}

export const EVENT_NONE: i32 = 0;
export const EVENT_CLOSE: i32 = 1;
export const EVENT_GADGET: i32 = 2;
export const EVENT_MENU: i32 = 3;
export const EVENT_MOUSE: i32 = 4;
export const EVENT_KEY: i32 = 5;
export const EVENT_ACTIVE: i32 = 6;
export const EVENT_INACTIVE: i32 = 7;
```

`EVENT_NONE` exists because the language rejects nullable struct values
(`E10011`). `waitEvent` returns `EVENT_NONE` only for an invalid or closed
window; `pollEvent` returns it when the queue is empty.

### Window functions

```clorraine
ffi stdintuition {
    func cl_open_window(borrow screen: Screen, x: i32, y: i32,
                        width: i32, height: i32, title: cstring): Window?;
    func cl_close_window(take window: Window): void;
    func cl_window_width(borrow window: Window): i32;
    func cl_window_height(borrow window: Window): i32;
    func cl_window_next_event(borrow window: Window, wait: bool): bool;
    func cl_window_event_kind(borrow window: Window): i32;
    func cl_window_event_code(borrow window: Window): i32;
    func cl_window_event_qualifier(borrow window: Window): u16;
    func cl_window_event_mouse_x(borrow window: Window): i32;
    func cl_window_event_mouse_y(borrow window: Window): i32;
    func cl_window_event_gadget(borrow window: Window): i32;
    func cl_window_event_menu(borrow window: Window): i32;
}

export func openWindow(borrow screen: Screen, x: i32, y: i32,
                       width: i32, height: i32, title: cstring): Window? {
    return cl_open_window(screen, x, y, width, height, title);
}

export func closeWindow(take window: Window): void {
    cl_close_window(window);
}

export func windowWidth(borrow window: Window): i32 {
    return cl_window_width(window);
}

export func windowHeight(borrow window: Window): i32 {
    return cl_window_height(window);
}
```

`waitEvent` and `pollEvent` build the value struct from the shim accessors:

```clorraine
func readEvent(borrow window: Window): WindowEvent {
    return WindowEvent {
        kind: cl_window_event_kind(window),
        code: cl_window_event_code(window),
        qualifier: cl_window_event_qualifier(window),
        mouse_x: cl_window_event_mouse_x(window),
        mouse_y: cl_window_event_mouse_y(window),
        gadget: cl_window_event_gadget(window),
        menu: cl_window_event_menu(window),
    };
}

export func waitEvent(borrow window: Window): WindowEvent {
    if (!cl_window_next_event(window, true)) {
        return WindowEvent { kind: EVENT_NONE, code: 0, qualifier: 0,
                             mouse_x: 0, mouse_y: 0, gadget: 0, menu: 0 };
    }
    return readEvent(window);
}

export func pollEvent(borrow window: Window): WindowEvent {
    if (!cl_window_next_event(window, false)) {
        return WindowEvent { kind: EVENT_NONE, code: 0, qualifier: 0,
                             mouse_x: 0, mouse_y: 0, gadget: 0, menu: 0 };
    }
    return readEvent(window);
}
```

### Window open

`cl_open_window` validates and opens:

1. Reject a null screen, a null title, a title over 64 bytes, `x < 0`, `y < 0`,
   `width <= 0`, `height <= 0`, `width > screen->Width`, `height > screen->Height`, `x > screen->Width - width`,
   or `y > screen->Height - height`. The subtraction checks avoid a signed
   overflow before the values are narrowed into `WORD` fields.
2. Allocate `cl_window` with `AllocMem(sizeof *handle, MEMF_CLEAR)`; return
   `null` on failure. The clear matters: the gadget, menu, and pointer fields
   must start null so the close path is safe.
3. Copy the title into a fixed 65-byte `AllocMem(65, MEMF_CLEAR)` buffer with a
   bounded copy; free the wrapper and return `null` if the copy fails. The
   buffer is freed with the same 65-byte size.
4. `memset(&nw, 0, sizeof nw)` and fill `struct NewWindow`:

| Field | Value |
|---|---|
| `LeftEdge`, `TopEdge` | caller's `x`, `y` |
| `Width`, `Height` | caller's outer width and height |
| `DetailPen`, `BlockPen` | 1, 0 |
| `IDCMPFlags` | `CLOSEWINDOW`, `GADGETUP`, `MENUPICK`, `MOUSEBUTTONS`, `RAWKEY`, `VANILLAKEY`, `ACTIVEWINDOW`, `INACTIVEWINDOW`, `REFRESHWINDOW` |
| `Flags` | `WFLG_DRAGBAR`, `WFLG_DEPTHGADGET`, `WFLG_CLOSEGADGET`, `WFLG_ACTIVATE`, `WFLG_GIMMEZEROZERO` |
| `FirstGadget`, `CheckMark` | `NULL` |
| `Title` | the copied title |
| `Screen` | the caller's `struct Screen *` |
| `BitMap` | `NULL` |
| `MinWidth`, `MinHeight`, `MaxWidth`, `MaxHeight` | 0 |
| `Type` | `CUSTOMSCREEN` |

5. Call `OpenWindow`. On `NULL`, free the title and wrapper and return `null`.
6. Call `WindowToFront` and `ActivateWindow` so the new window owns the
   screen's input focus and its menu strip is visible.
7. Return the `cl_window` pointer.

`windowWidth`/`windowHeight` return `window->GZZWidth`/`GZZHeight`, the inner
drawable area created by `WFLG_GIMMEZEROZERO`.

The caller's `Screen` must outlive the window: the window stores
`Window.WScreen` and `CloseWindow` touches it. The `Window` handle has no
lifetime parameter, so the caller has to enforce the order:

- locals drop in reverse declaration order, so declare the screen before the
  window;
- struct fields drop in declaration order, so a struct that holds both must
  declare the `window` field before the `screen` field; and
- do not return a window from a function whose locals own the screen: the
  screen would close at the return and leave the window dangling.

The demo uses the local rule. The implementation suite pins both the local and
struct cleanup order.

### Window close

`cl_close_window` is null-safe and does this in order:

1. `ClearPointer` if the pointer is shown.
2. `ClearMenuStrip` if the window has a menu strip.
3. `CloseWindow`.
4. Free every gadget node: the string buffer (size `StringInfo.MaxChars`), the
   string `Border` and its `XY` array, the button normal/select `Image` records,
   the label `IntuiText`, the label copy (65 bytes), the `struct Gadget`, and
   the node.
5. Free every menu node: every `MenuItem` label copy (65 bytes) and
   `IntuiText`, every `MenuItem`, the menu title copy (65 bytes), `struct Menu`,
   and the node.
6. Free the pointer chip allocation (64 bytes), the title copy (65 bytes), and
   the wrapper (`sizeof *handle`).

Every allocation is paired with an exact-size `FreeMem`; variable text uses the
fixed 65-byte labels/titles, and the Intuition-edited string buffer uses
`StringInfo.MaxChars`, never `strlen`. The generated `Window` drop calls the
same function exactly once. `closeWindow` moves the handle, so the drop does not
run on the caller's binding.

Phase 1 implements step 6 only (the lists and pointer are null); the gadget,
menu, border, and pointer steps are added in 2.1.2, 2.1.3, and 3.1.3.

### Window drawing and text

`std.amiga.graphics` gains:

```clorraine
func cl_window_fill(borrow window: Window, x1: i32, y1: i32,
                    x2: i32, y2: i32, pen: u8): void;
func cl_window_draw(borrow window: Window, x1: i32, y1: i32,
                    x2: i32, y2: i32, pen: u8): void;
func cl_window_clear(borrow window: Window, pen: u8): void;

export func windowFill(borrow window: Window, x1: i32, y1: i32,
                       x2: i32, y2: i32, pen: u8): void;
export func windowDraw(borrow window: Window, x1: i32, y1: i32,
                       x2: i32, y2: i32, pen: u8): void;
export func windowClear(borrow window: Window, pen: u8): void;
```

`windowClear` fills `0,0` to `GZZWidth-1,GZZHeight-1`. All three are no-ops on a
null window.

`std.amiga.text` gains:

```clorraine
func cl_window_text_draw(borrow window: Window, borrow font: Font,
                         x: i32, y: i32, pen: u8,
                         text: raw u8, count: u32): void;

export func windowText(borrow window: Window, borrow font: Font,
                       x: i32, y: i32, pen: u8, const text: u8[]): void;
```

`cl_window_text_draw` saves the window RastPort's current font, sets the caller's
font, sets `APen`, moves, calls `Text`, and restores the old font. The count is
the caller's slice length.

### Event loop

`cl_window_next_event(window, wait)`:

1. If the wrapper, `struct Window`, or `UserPort` is null, return 0.
2. If `wait`, `WaitPort(window->UserPort)`; otherwise `GetMsg` directly.
3. `msg = GetMsg(window->UserPort)`. If null, return 0.
4. If `msg->Class == IDCMP_REFRESHWINDOW`, call `BeginRefresh`,
   `EndRefresh(window, TRUE)`, `ReplyMsg`, and loop to step 2/3. Refresh is
   never delivered.
5. Zero `last_event`, then fill it:

| IntuiMessage | `WindowEvent` |
|---|---|
| `IDCMP_CLOSEWINDOW` | `kind = EVENT_CLOSE` |
| `IDCMP_GADGETUP` | `kind = EVENT_GADGET`; `gadget` from the `IAddress` node, else 0 |
| `IDCMP_MENUPICK` | `kind = EVENT_MENU`; `menu` from `ItemAddress(window->MenuStrip, Code)`, else 0 |
| `IDCMP_MOUSEBUTTONS` | `kind = EVENT_MOUSE` |
| `IDCMP_RAWKEY`, `IDCMP_VANILLAKEY` | `kind = EVENT_KEY` |
| `IDCMP_ACTIVEWINDOW` | `kind = EVENT_ACTIVE` |
| `IDCMP_INACTIVEWINDOW` | `kind = EVENT_INACTIVE` |
| anything else | reply and continue with the next message |

6. Copy `Code` to `code`, `Qualifier` to `qualifier`, and
   `window->GZZMouseX`/`GZZMouseY` to `mouse_x`/`mouse_y`. The `GZZ` fields are
   the inner-window coordinates that match drawing and gadget positions;
   `IntuiMessage.MouseX`/`MouseY` are relative to the outer window and are not
   used directly.
7. `ReplyMsg` and return 1.

The accessor functions read `last_event`; the wrapper copies every field before
the next call. There is no allocation per event and no queue owned by the
wrapper.

## Code Example

```clorraine
import { Screen, openScreen, Window, openWindow, closeWindow,
         waitEvent, WindowEvent, EVENT_CLOSE } from std.amiga.intuition;
import { setColor, windowClear } from std.amiga.graphics;
import { String, stringOf, toCString } from std.text;

func run(): i32 {
    // The screen keeps this pointer; it must outlive the screen.
    let screenTitle: String = stringOf("Win");
    let screen: Screen? = openScreen(320, 200, 4, toCString(screenTitle));
    if (screen == null) { return 1; }
    setColor(screen, 0, 0, 0, 0);
    // The window copies this title, so a temporary would also be safe here.
    let window: Window? = openWindow(screen, 24, 24, 272, 136,
                                      toCString(stringOf("Demo")));
    if (window == null) { return 2; }
    windowClear(window, 1);
    for (;;) {
        let event: WindowEvent = waitEvent(window);
        if (event.kind == EVENT_CLOSE) { break; }
    }
    return 0;
}
```

## Error Handling

| Error Case | Handling | AR Ref |
|---|---|---|
| Null screen, null title, over-long title | `null`, nothing allocated | AR-14 |
| Window outside the screen or non-positive size | `null`, nothing allocated | AR-14 |
| `OpenWindow` failure | free title and wrapper, `null` | AR-15 |
| Null or closed window in drawing/text/event calls | void drawing/text calls are no-ops; event calls return `EVENT_NONE` | AR-15, AR-23 |
| Unknown `IAddress` or menu item | `gadget`/`menu` = 0 | AR-16 |
| Refresh event | acknowledged and not delivered | AR-12 |
| Allocation failure during a later add | the boolean add call returns `false` (or `addMenu` returns `-1`) and frees its partial allocation | AR-15 |

## Testing Requirements

- Compile the full window/event surface on both hosted profiles and cross-build
  with `nix13`/`nix20`.
- Assert the host profile rejects `std.amiga.intuition` with `E11200`.
- Pin the `NewWindow` flags, the IDCMP set, the `AddGList`/`RefreshGList` calls,
  the `BeginRefresh`/`EndRefresh` auto-ack, and the event accessor names in the
  implementation suite.
- Prove the window, drawing, and close path on the A500 with the GUI demo.
