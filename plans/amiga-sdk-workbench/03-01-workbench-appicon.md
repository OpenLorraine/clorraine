# Design: Workbench Application Objects and Messages

> **Document**: 03-01-workbench-appicon.md
> **Parent**: [Index](00-index.md)
> **Covers**: R1, R4, R5, R8 (Workbench part), AR-2–AR-5, AR-10, AR-11

## Public API

```clorraine
module std.amiga.workbench;

import { MsgPort } from std.amiga.exec;
import { Window } from std.amiga.intuition;

export handle AppIcon freedBy cl_remove_app_icon;
export handle AppWindow freedBy cl_remove_app_window;
export handle AppMenuItem freedBy cl_remove_app_menu_item;
export handle AppMessage freedBy cl_reply_app_message;

ffi stdworkbench {
    func cl_add_app_icon(borrow port: MsgPort, title: cstring,
                         id: u32, user_data: u32): AppIcon?;
    func cl_remove_app_icon(take icon: AppIcon): void;
    func cl_add_app_window(borrow window: Window, borrow port: MsgPort,
                           id: u32, user_data: u32): AppWindow?;
    func cl_remove_app_window(take window: AppWindow): void;
    func cl_add_app_menu_item(borrow port: MsgPort, title: cstring,
                              id: u32, user_data: u32): AppMenuItem?;
    func cl_remove_app_menu_item(take item: AppMenuItem): void;
    func cl_wait_app_message(borrow port: MsgPort): AppMessage?;
    func cl_app_message_type(borrow message: AppMessage): i32;
    func cl_app_message_id(borrow message: AppMessage): u32;
    func cl_app_message_user_data(borrow message: AppMessage): u32;
    func cl_app_message_class(borrow message: AppMessage): i32;
    func cl_app_message_arg_count(borrow message: AppMessage): i32;
    func cl_reply_app_message(take message: AppMessage): void;
    func cl_save_public_screen(name: cstring, path: cstring): bool;
}
```

The module is available only on `hosted-amiga`; `host`, `hosted-amiga-13`,
and `freestanding` reject it with `E11200`.

## Base Globals

The 2.0 shim defines and assigns the NDK base globals the `clib` stubs use:

```c
struct Library *WorkbenchBase;
struct Library *IconBase;
struct Library *KeymapBase;
struct Library *ExpansionBase;
struct Library *CardResource;
```

`WorkbenchBase`/`IconBase` are assigned from `OpenLibrary("workbench.library",
37)` and `OpenLibrary("icon.library", 37)` in the lazy open helper.
`CardResource` is assigned from `OpenResource("card.resource")` before
`ReadCardStatus` and cleared after `RemResource`. The 1.3 build guards the
whole section out, so its zeroed stubs are never used.

## Application-Object Ownership

| Handle | Underlying record | Created by | Released by |
|---|---|---|---|
| `AppIcon` | `cl_app_icon { struct AppIcon *icon; struct DiskObject *object; char *title; }` | `AddAppIconA` with a default `WBTOOL` object whose `do_Type` is cleared to 0 | `RemoveAppIcon`, `FreeDiskObject`, `FreeMem(title)`, `FreeMem(record)` |
| `AppWindow` | `struct AppWindow *` | `AddAppWindowA` on the caller's `cl_window.window` | `RemoveAppWindow` |
| `AppMenuItem` | `cl_app_menu_item { struct AppMenuItem *item; char *title; }` | `AddAppMenuItemA` | `RemoveAppMenuItem`, `FreeMem(title)`, `FreeMem(record)` |
| `AppMessage` | `struct AppMessage *` | Workbench or the test fixture | `ReplyMsg` exactly once |

The AppIcon record owns the default `DiskObject` because Workbench may redraw
the icon until it is removed. The AppWindow borrows the `Window`; the window
must stay open until the AppWindow is removed. The demo declares the window
before the AppWindow, so the AppWindow drops first.

## AppIcon Creation

`cl_add_app_icon`:

1. Reject a null port or title with `NULL`.
2. Open `workbench.library` 37 and `icon.library` 37 lazily; a failure returns
   `NULL`.
3. Copy `title` into a public allocation; a failure returns `NULL`.
4. `GetDefDiskObject(WBTOOL)`; a null result frees the copy and returns `NULL`.
5. Set `object->do_Type = 0`, `object->do_CurrentX = 400`, and
   `object->do_CurrentY = 140` so the icon is visible at a pinned position.
6. Allocate the `cl_app_icon` record; on failure free the object and title.
7. `AddAppIconA(id, user_data, title_copy, port, 0, object, tags)` with
   `WBAPPICONA_PropagatePosition, TRUE` and `TAG_DONE`. A null result frees the
   object, title, and record and returns `NULL`.
8. Return the record.

The AppIcon record owns the title copy and the `DiskObject` until removal. The
lock is null, matching the Workbench documentation example for an application
icon.

## AppWindow and AppMenuItem

`cl_add_app_window` validates the `Window` and `MsgPort`, opens
`workbench.library`, and calls `AddAppWindowA(id, user_data,
handle->window, port, NULL)`. The caller must keep the `Window` alive. The demo
uses a window on its own custom screen; the row proves the AppWindow lifecycle,
not a Workbench-screen drop zone or a drop message.

`cl_add_app_menu_item` validates the port and title, copies the title, opens
`workbench.library`, and calls `AddAppMenuItemA(id, user_data, title_copy,
port, NULL)`. The record owns the title copy.

`RemoveAppWindow`/`RemoveAppMenuItem` return a BOOL with unspecified high bits
on the 3.1 ROM; the wrappers are `void` and do not inspect it. The object
pointer is the ownership token.

## AppMessage

`cl_wait_app_message` calls `WaitPort(port)` and then `GetMsg(port)`. The port
is expected to carry only AppMessages; the caller owns the port. The accessors
read `am_Type`, `am_ID`, `am_UserData`, `am_Class`, and `am_NumArgs`.
`cl_reply_app_message` calls `ReplyMsg` exactly once; the generated drop does
the same for a message the caller never handled.

The AppMessage argument list (`am_ArgList`) is deliberately not exposed: it
contains Workbench-owned locks and names that need DOS ownership rules. The
demo reports the count only.

## Public Test Port

`std.amiga.exec.createPublicPort(name)` calls `CreatePort(name, 0)`. The
existing `MsgPort` handle and `cl_delete_msg_port` drop work unchanged.
`CreatePort` does not copy the name, so the caller keeps the `cstring` alive
until the port is deleted. The name must be non-empty and unique; the caller
drains the port before the drop. The demo declares the `portName` `String`
before the port and keeps it alive to the end of the function. The AppMessage
fixture finds `ClorraineWbTest` with `FindPort`.

## Public Screen Capture

`cl_save_public_screen(name, path)`:

1. Reject null name/path.
2. `screen = LockPubScreen(name)`; null returns `false`.
3. `ok = cl_screen_save_native(screen, path)`.
4. `UnlockPubScreen(NULL, screen)`.
5. Return `ok`.

`cl_screen_save_native` is the existing native `CSCR` writer; it reads the
screen bitmap and palette, so the capture is the Amiga image and is independent
of the host window.

## Test-Only Workbench Start

`docker/scripts/workbench-start.c` opens `workbench.library` 37 and calls the
private `StartWorkbench` vector at LVO -42 with `(0, 0)`, then waits 2 s and
locks `Workbench` to confirm the screen exists. This mirrors `C:LoadWB`, which
is not available without Workbench media. It is compiled only by the gate and
is never part of the public SDK. The probe returned `1` and a non-null
`LockPubScreen` on KS2.05 and KS3.1.

## Failure Values

| Call | Failure value |
|---|---|
| `addAppIcon`, `addAppWindow`, `addAppMenuItem` | `null` |
| `waitAppMessage` | `null` only for a null port |
| `appMessageType`, `appMessageId`, `appMessageUserData`, `appMessageClass`, `appMessageArgCount` | 0 for a null message |
| `savePublicScreen` | `false` |

No public call panics. The handles are move-only; `replyAppMessage` and the
generated drops are exactly-once.

## Test Hooks

The specification test compiles the full surface on `hosted-amiga` and
cross-builds it with `nix20`. The implementation test pins the emitted shim
calls, the AppIcon title/object record, the AppWindow borrow, the
AppMenuItem title copy, the AppMessage accessors, the public screen capture,
the base-global assignments, and the drop routing. The A1200 gate proves the live AppIcon, the three AppMessage types, and the
native capture/AppIcon crop; the A600 gate proves the live AppIcon and the
three AppMessage types without a capture comparison.
