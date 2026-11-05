# Requesters and Pointer: Amiga SDK Bundle 6

> **Document**: 03-03-requesters-pointer.md
> **Parent**: [Index](00-index.md)
> **Covers**: R7–R8, R11; AR-7, AR-13, AR-14, AR-15, AR-17

## Overview

This component adds the modal Yes/No requester, the recovery alert, and the
built-in pointer. All three use classic Kickstart 1.3 calls: `AutoRequest`,
`DisplayAlert`, `SetPointer`, and `ClearPointer`. Text is copied into temporary
shim-owned storage for the duration of the blocking call; the pointer image is
allocated in chip memory and owned by the window.

## Architecture

### Current Architecture

No requester, alert, or pointer code exists. `std.amiga.intuition` exposes only
the screen (`stdlib/std.amiga.intuition.clo:14-31`). The 1.3 SFD lists
`AutoRequest`, `DisplayAlert`, `SetPointer`, and `ClearPointer`; the 1.3 header
defines `IntuiText`, `RECOVERY_ALERT`, and the sprite/pointer fields on
`struct Window`.

### Proposed Changes

Add three public functions and their shim helpers:

```clorraine
ffi stdintuition {
    func cl_window_ask_yes_no(borrow window: Window, body: cstring): bool;
    func cl_show_alert(body: cstring): void;
    func cl_window_set_pointer(borrow window: Window, show: bool): void;
}

export func askYesNo(borrow window: Window, body: cstring): bool {
    return cl_window_ask_yes_no(window, body);
}

export func showAlert(body: cstring): void {
    cl_show_alert(body);
}

export func setPointer(borrow window: Window, show: bool): void {
    cl_window_set_pointer(window, show);
}
```

## Implementation Details

### Yes/No requester

`cl_window_ask_yes_no`:

1. Reject a null window or body, or a body over 28 bytes, with `false`.
2. Copy the body into a temporary 29-byte NUL-terminated buffer.
3. Build three stack `struct IntuiText` records:

| Record | `FrontPen` | `BackPen` | `DrawMode` | `IText` |
|---|---|---|---|---|
| body | 1 | 0 | `JAM2` | body copy |
| positive | 1 | 0 | `JAM2` | `"Yes"` |
| negative | 1 | 0 | `JAM2` | `"No"` |

4. Call the 1.3 eight-argument form
   `AutoRequest(window, &body, &positive, &negative, 0, 0, 240, 40)`.
   The two flags are IDCMP flags (zero here), not pen flags. The 1.3 call uses
   the width and height directly; `0,0` does **not** auto-size. The 240x40
   requester fits the 272-pixel window and the 28-byte (224-pixel) body.
5. Free the body copy. Return `result != 0`.

`AutoRequest` blocks until the user selects Yes or No. The requester is centered
in the window and uses the window's screen. A failed allocation returns
`false` without showing anything.

### Recovery alert

`DisplayAlert` takes an **AlertMessage**, not a plain C string. Its format is a
16-bit big-endian x coordinate, an 8-bit y coordinate, the text bytes, a
terminating NUL, and a zero continuation byte (two NULs total).

`cl_show_alert`:

1. Reject a null body or a body over 28 bytes; a rejected call is a no-op.
2. Allocate a zeroed `CL_ALERT_MESSAGE_MAX` (33-byte) buffer.
3. Write the header `{0x00, 0x14, 0x14}` (x=20, y=20), copy the body to offset
   3, and rely on the zero fill for the text NUL and continuation NUL.
4. Call `DisplayAlert(1, message, 20)`. The alert number's type bits are zero,
   so this is a `RECOVERY_ALERT`; the height 20 is two text lines.
5. Free the message with `CL_ALERT_MESSAGE_MAX`.

`DisplayAlert` blocks until the user selects Continue. It uses the default
screen, so the caller's custom screen is temporarily covered; the window layer
restores its content when the alert returns. The 28-byte body (224 pixels at
Topaz 8) plus x=20 fits the 320-pixel alert display.

### Built-in pointer

`cl_window_set_pointer(window, show)`:

1. Reject a null window.
2. If `show` is false: call `ClearPointer(window)`, clear `pointer_shown`, and
   return. The allocated image stays for a later show.
3. If `show` is true and `window->pointer` is null:
   - Allocate 64 bytes with `AllocMem(64, MEMF_CHIP | MEMF_CLEAR)`. The size is
     16 rows × 2 planes × 2 bytes.
   - Copy the static 16x16 arrow template into the chip buffer.
   - On allocation failure, return with the pointer hidden.
4. Call `SetPointer(window, pointer, 16, 16, 0, 0)` and set `pointer_shown = 1`.

The pointer is a two-plane sprite: color 0 is transparent, color 1 is white, and
color 2 is black. The sprite palette entries are pen 16 (color 0), pen 17
(color 1), pen 18 (color 2), and pen 19 (color 3). The demo sets pens 17 and 18
with `setColor(screen, 17, 255, 255, 255)` and
`setColor(screen, 18, 0, 0, 0)`; pen 16 is unused. `setColor` accepts 0–31
because the screen `ColorMap` has 32 entries: 0–15 are the depth-4 playfield
colors and 16–31 are the sprite/pointer registers.

`cl_close_window` clears the pointer if it is shown, then frees the chip buffer
with `FreeMem(pointer, 64)`. No pointer memory survives the window.

The template is a simple upper-left arrow: rows 0–10 form the shaft and head in
white with a black outline; rows 11–15 taper to the tip. The exact pixels are
pinned by the A500 held-frame golden, not by a host test.

## Code Example

```clorraine
import { Screen, Window, openWindow, askYesNo, showAlert,
         setPointer } from std.amiga.intuition;
import { setColor } from std.amiga.graphics;
import { String, stringOf, toCString } from std.text;

func prompt(screen: borrow Screen, window: borrow Window): bool {
    setColor(screen, 17, 255, 255, 255);
    setColor(screen, 18, 0, 0, 0);
    setPointer(window, true);
    let answer: bool = askYesNo(window, toCString(stringOf("Continue?")));
    if (answer) {
        showAlert(toCString(stringOf("Done")));
    }
    return answer;
}
```

## Error Handling

| Error Case | Handling | AR Ref |
|---|---|---|
| Null window/body | `false` or no-op | AR-15 |
| Body over 28 bytes | `false` or no-op | AR-14 |
| Body copy allocation failure | `false` or no-op; no requester/alert | AR-15 |
| `AutoRequest` returns No | `false` | AR-13 |
| Pointer chip allocation failure | return with the pointer hidden | AR-15 |
| Null window in `setPointer` | no-op | AR-15 |
| Window close with the pointer shown | `ClearPointer` then `FreeMem` | AR-15 |

## Testing Requirements

- Compile the requester, alert, and pointer surface on both hosted profiles and
  cross-build with `nix13`/`nix20`.
- Pin `AutoRequest`, `DisplayAlert`, `SetPointer`, `ClearPointer`, and
  `MEMF_CHIP` in the shim implementation tests.
- Prove the requester, alert, and custom pointer on the A500 demo frame and
  report line.
- Reject 2.0+ alternatives (`EasyRequest`, `ASL`, `SetRGB32`) in the
  implementation suite.
