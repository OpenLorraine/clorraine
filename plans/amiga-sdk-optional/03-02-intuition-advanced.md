# Intuition Advanced: Bundle 9

> **Document**: 03-02-intuition-advanced.md
> **Parent**: [Index](00-index.md)

## Overview

This component adds `SDK-INT-006` gadtools, `SDK-INT-007` ASL requesters, and `SDK-INT-008`
BOOPSI. The classic `std.amiga.intuition` module stays 1.3-safe; the new modules are
`hosted-amiga` only. gadtools/ASL are 2.0+; BOOPSI is 3.0+ and uses the native
`colorwheel.gadget`. [AR-8, AR-10, AR-14–AR-16]

## Architecture

### Current Architecture

`std.amiga.intuition` exposes screens/windows and a blocking requester. Classic gadgets and menus
are private nodes owned by `cl_window`; event IDs are resolved only through that private list
(`compiler/runtime/shim_hosted_amiga.c:2480-2493`). `cl_window_activate_gadget` also walks only the
private list (`:2214-2227`). gadtools/BOOPSI gadgets are ordinary `struct Gadget` nodes, so the
shared lookup must walk `window->FirstGadget` to find the node and read its `GadgetID`.

### Proposed Changes

Add `stdlib/std.amiga.gadtools.clo`, `stdlib/std.amiga.asl.clo`, and
`stdlib/std.amiga.boopsi.clo`; put C in the optional suffix. Add the shared event fallback and the
optional `GT_*` event path. The ASL library is installed from the Workbench 2.04/3.1 ADF; the
colorwheel class is installed from the Workbench 3.0/3.1 ADF. [AR-7, AR-14–AR-16]

## Implementation Details

### `std.amiga.gadtools` (2.0+)

| Export | Signature | Ownership / behavior |
|---|---|---|
| `GadgetList` | `handle freedBy cl_free_gadget_list` | Owns `VisualInfo`, `NewGadget + TextAttr` copies, `Gadget *` |
| `newGadgetList` | `(borrow window: Window, fontName: cstring, fontSize: u16): GadgetList?` | Copies the font name; size 1..255; window must outlive |
| `addButton` | `(borrow list, id: i32, text: cstring, x: i32, y: i32, width: i32, height: i32): bool` | Copies text; id 1..65535; unique |
| `addText` | `(borrow list, id: i32, text: cstring, x, y, width, height): bool` | Copies text |
| `addString` | `(borrow list, id: i32, initial: cstring, maxChars: i32, x, y, width, height): bool` | Copies initial; maxChars 1..255 |
| `addInteger` | `(borrow list, id: i32, value: i32, maxChars: i32, x, y, width, height): bool` | Initial value only; no runtime min/max |
| `buildGadgets` | `(borrow list): bool` | `CreateContext`, one `CreateGadgetA` per spec, `RefreshGList`, `GT_RefreshWindow`; idempotent |
| `freeGadgetList` | `(take list): void` | `RemoveGList`, `FreeGadgets`, `FreeVisualInfo`, free copies |
| `gadgetWaitEvent` | `(borrow window: Window): WindowEvent` | `GT_GetIMsg`, refresh begin/end, reply; returns the shared `WindowEvent` |
| `gadgetPollEvent` | `(borrow window: Window): WindowEvent` | Non-blocking companion |

`CreateGadgets` does not exist; `CreateContext` + `CreateGadgetA` is the 2.0 API. The list must be
freed before its window; `cl_window_destroy` does not know about it. The gate follows that order.
[AR-14]

### `std.amiga.asl` (2.0+; save mode 2.04+)

| Export | Signature | Ownership / behavior |
|---|---|---|
| `FileRequester` | `handle freedBy cl_free_file_requester` | Owns `struct FileRequester *` and copied title/path |
| `newFileRequester` | `(title: cstring, initialPath: cstring, save: bool): FileRequester?` | Splits `initialPath` at the last `/` or `:` into `ASLFR_InitialDrawer`/`InitialFile` |
| `requestFile` | `(borrow request, borrow window: Window): bool` | Blocking `AslRequest`; true on positive; sets `last_ok` |
| `fileRequesterPath` | `(borrow request, borrow buf: u8[]): i32` | Copies `fr_Drawer` + `fr_File` (absolute file wins); -1 on cancel/none |
| `freeFileRequester` | `(take request): void` | `FreeAslRequest`; frees copies |

`ASLFR_DoSaveMode` is V38/2.04; the A600/KS2.05 gate meets that. The demo writes `asl-ready.txt`
before `AslRequest` and `asl-done.txt` after. [AR-15]

### `std.amiga.boopsi` (3.0+)

| Export | Signature | Ownership / behavior |
|---|---|---|
| `BoopsiObject` | `handle freedBy cl_dispose_object` | Owns `Object *`; tracks the attached window |
| `newColorWheel` | `(borrow window: Window, id: i32, x: i32, y: i32, width: i32, height: i32): BoopsiObject?` | `NewObjectA(NULL, "colorwheel.gadget", GA_*, WHEEL_Screen)` |
| `attachObject` | `(borrow window: Window, borrow object): bool` | `AddGList` + `RefreshGList`; false if already attached |
| `detachObject` | `(borrow object): void` | `RemoveGList` + `RefreshGList` |
| `setColorWheelHue` | `(borrow object, hue: u32): bool` | `SetGadgetAttrsA(WHEEL_Hue)` when attached |
| `colorWheelHue` | `(borrow object): u32` | `GetAttr(WHEEL_Hue)` |
| `disposeObject` | `(take object): void` | Detaches if attached, then `DisposeObject` |

`colorwheel.gadget` requires `WHEEL_Screen`; the shim passes `((cl_window *)window)->window->WScreen`.
The object must be disposed before the window; `disposeObject` detaches if the window is still alive.
No custom class, no ClassAct. [AR-16]

## Shared-Shim Changes

| Change | Reason |
|---|---|
| `cl_window_gadget_id` walks `window->FirstGadget` to find the node, then reads `GadgetID` | Avoids a blind dereference of a removed gadget; gadtools/BOOPSI IDs |
| `cl_window_activate_gadget` walks `window->FirstGadget` after the private list | Keyboard focus for string/integer gadgets |

Both use only 1.3-era structures and remain in the shared shim.

## Error Handling

- Acquisition returns `null`; `buildGadgets`/`requestFile`/`attachObject` return false on failure.
- ASL cancel returns false; `fileRequesterPath` returns -1.
- A list/object must be freed before its window; docs and gate enforce the order.
- Gadget IDs are 1..65535; zero is rejected.

## Tests

- Compile all three surfaces on `hosted-amiga`; reject host/1.3/freestanding with `E11200`.
- Cross-build `nix20`; no `nix13` for these modules.
- Pin `GetVisualInfo`, `CreateContext`, `CreateGadgetA`, `RefreshGList`, `GT_RefreshWindow`,
  `GT_GetIMsg`, `GT_ReplyIMsg`, `GT_BeginRefresh`, `GT_EndRefresh`, `FreeGadgets`,
  `FreeVisualInfo`, `AllocAslRequest`, `AslRequest`, `FreeAslRequest`, `NewObjectA`,
  `SetGadgetAttrsA`, `GetAttr`, `AddGList`, `RemoveGList`, `DisposeObject`.
- Runtime: A600/KS2.05 for gadtools/ASL; A1200/KS3.0 for BOOPSI.
- Gate lines: `gt: ok gad 4`; `asl: ok path asl.txt`; `boopsi: ok hue 42`.
