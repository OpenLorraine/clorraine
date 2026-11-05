# Gadgets and Menus: Amiga SDK Bundle 6

> **Document**: 03-02-gadgets-and-menus.md
> **Parent**: [Index](00-index.md)
> **Covers**: R4–R6, R11–R12; AR-2, AR-8, AR-10, AR-14–AR-17

## Overview

This component adds classic Kickstart 1.3 gadgets and a menu bar to an open
`Window`: `GTYP_BOOLGADGET` buttons, one `GTYP_STRGADGET` text field, and a
classic `Menu`/`MenuItem` strip. The window owns every node, label, image,
`IntuiText`, `StringInfo`, and string buffer so a close releases all of them.

## Architecture

### Current Architecture

No gadget or menu code exists. `std.amiga.intuition` exposes only the screen
(`stdlib/std.amiga.intuition.clo:14-31`). The 1.3 SDK provides the classic
structures and the `AddGList`/`RefreshGList` and `SetMenuStrip`/`ClearMenuStrip`
vectors; `ItemAddress` maps a menu-pick code back to a `MenuItem`.

### Proposed Changes

Add a gadget node and a menu node to the private `cl_window` wrapper
(`03-01`). Both are singly linked lists in insertion order. The public surface
is `addButton`, `addString`, `windowString`, `addMenu`, and `addMenuItem`.

```c
typedef struct cl_gadget_node {
    struct cl_gadget_node *next;
    struct Gadget *gadget;
    struct Image *normal_image;   /* button only */
    struct Image *select_image;   /* button only */
    struct Border *box;           /* string only */
    WORD *box_xy;                 /* string only */
    struct IntuiText *label;
    char *label_text;             /* fixed 65-byte copy */
    struct StringInfo *string_info; /* string only */
    char *string_buffer;            /* string only, MaxChars bytes */
    LONG id;
} cl_gadget_node;

typedef struct cl_menu_item_node {
    struct cl_menu_item_node *next;
    struct MenuItem *item;
    struct IntuiText *label;
    char *label_text;
    LONG id;
} cl_menu_item_node;

typedef struct cl_menu_node {
    struct cl_menu_node *next;
    struct Menu *menu;
    struct cl_menu_item_node *items;
    char *title;
} cl_menu_node;
```

## Implementation Details

### New Functions

```clorraine
ffi stdintuition {
    func cl_window_add_button(borrow window: Window, x: i32, y: i32,
                              width: i32, height: i32,
                              label: cstring, id: i32): bool;
    func cl_window_add_string(borrow window: Window, x: i32, y: i32,
                              width: i32, height: i32, label: cstring,
                              initial: cstring, max_chars: i32,
                              id: i32): bool;
    func cl_window_string(borrow window: Window, id: i32,
                          buf: raw u8, cap: u32): i32;
    func cl_window_add_menu(borrow window: Window, title: cstring): i32;
    func cl_window_add_menu_item(borrow window: Window, menu: i32,
                                 label: cstring, id: i32): bool;
}

export func addButton(borrow window: Window, x: i32, y: i32,
                      width: i32, height: i32, label: cstring,
                      id: i32): bool {
    return cl_window_add_button(window, x, y, width, height, label, id);
}

export func addString(borrow window: Window, x: i32, y: i32,
                      width: i32, height: i32, label: cstring,
                      initial: cstring, maxChars: i32, id: i32): bool {
    return cl_window_add_string(window, x, y, width, height, label,
                                initial, maxChars, id);
}

export func windowString(borrow window: Window, id: i32,
                         borrow buf: u8[]): i32 {
    unsafe {
        return cl_window_string(window, id, raw u8(buf.ptr), u32(len(buf)));
    }
}

export func addMenu(borrow window: Window, title: cstring): i32 {
    return cl_window_add_menu(window, title);
}

export func addMenuItem(borrow window: Window, menu: i32,
                        label: cstring, id: i32): bool {
    return cl_window_add_menu_item(window, menu, label, id);
}
```

### Common validation

Every add call rejects a null window or text, an id below 1, an id already used
by another gadget or menu item, a label over 64 bytes, `x < 0`, `y < 0`,
`width <= 0`, `height <= 0`, and a rectangle that leaves the inner window:
`x > GZZWidth - width` or `y > GZZHeight - height`, with `width`/`height`
checked against the inner size first so the subtraction cannot wrap. `addString`
additionally requires `maxChars` in `1..64` and an initial string no longer than
`maxChars`; a null initial is rejected. A rejected call returns `false` and
allocates nothing that outlives it. A label wider than its gadget is allowed and
is clipped by Intuition.

### Buttons

`cl_window_add_button` allocates in this order, every record with
`AllocMem(sizeof *, MEMF_CLEAR)` or `memset` so every `Next*`, `ITextFont`, and
flag starts null:

1. `cl_gadget_node`, `struct Gadget`, and two `struct Image` records.
2. The 65-byte label copy, `struct IntuiText`, and the `IText` pointer.

It fills:

| Field | Value |
|---|---|
| `Gadget.LeftEdge`, `TopEdge`, `Width`, `Height` | caller's geometry |
| `Gadget.Flags` | `GFLG_GADGIMAGE | GFLG_GADGHIMAGE` |
| `Gadget.Activation` | `GACT_RELVERIFY` |
| `Gadget.GadgetType` | `GTYP_BOOLGADGET` |
| `Gadget.GadgetRender` | normal image |
| `Gadget.SelectRender` | select image |
| `Gadget.GadgetText` | label `IntuiText` |
| `Gadget.SpecialInfo` | `NULL` |
| `Gadget.GadgetID` | low 16 bits of the caller id |

Both images set `LeftEdge = 0`, `TopEdge = 0`, `Width`/`Height` from the gadget
box, `Depth = 1`, `ImageData = NULL`, `NextImage = NULL`, `PlanePick = 0`, and
`PlaneOnOff = 1` (normal) or `2` (select). This paints a solid pen-1 button that
switches to pen 2 while selected. The label uses `FrontPen = 0`, `BackPen = 1`,
`DrawMode = JAM1`, `ITextFont = NULL`, and `NextText = NULL`, so it stays
readable over the filled button. The caller sets pens 0, 1, and 2 with
`setColor`.

After the structures are valid, the shim calls
`AddGList(window, gadget, -1, 1, NULL)` and then
`RefreshGList(gadget, window, NULL, 1)`. `AddGList` returns the insertion
position, not a status; 0 is a valid first position, so the return is ignored.
`RefreshGList` is `VOID`. The wrapper node is appended to the window's gadget
list after the calls and the function returns `true`.

A `GADGETUP` message has `IAddress` pointing at the `struct Gadget`; the event
loop finds the matching node and reports its id as `WindowEvent.gadget`.

### String gadget

`cl_window_add_string` allocates the node, `struct Gadget`, `struct StringInfo`,
a `struct Border` plus its five-point `WORD[10]` XY array, the label copy,
`IntuiText`, and a `maxChars + 1` byte buffer. It fills:

| Field | Value |
|---|---|
| `Gadget.Flags` | 0 (the render is a `Border`, not an `Image`) |
| `Gadget.Activation` | `GACT_IMMEDIATE` |
| `Gadget.GadgetType` | `GTYP_STRGADGET` |
| `Gadget.GadgetRender`, `SelectRender` | the same `Border` |
| `Gadget.GadgetText` | label `IntuiText` |
| `Gadget.SpecialInfo` | the `StringInfo` |
| `StringInfo.Buffer` | the shim-owned `maxChars + 1` buffer |
| `StringInfo.UndoBuffer` | `NULL` |
| `StringInfo.BufferPos` | 0 |
| `StringInfo.MaxChars` | `maxChars + 1` (the header includes the NUL) |
| `StringInfo.DispPos` | 0 |

The `Border` sets `LeftEdge = 0`, `TopEdge = 0`, `FrontPen = 1`, `BackPen = 0`,
`DrawMode = JAM2`, `Count = 5`, `XY` = `{0,0, w-1,0, w-1,h-1, 0,h-1, 0,0}`,
`NextBorder = NULL`. The label uses `FrontPen = 1`, `BackPen = 0`, `DrawMode =
JAM1`, `ITextFont = NULL`, `NextText = NULL`.

The initial text is copied up to `maxChars` and NUL-terminated;
`StringInfo.NumChars` is its length. Intuition edits the buffer in place.

`cl_window_string` finds the node by id, requires a string gadget, copies
`min(StringInfo.NumChars, StringInfo.MaxChars - 1, cap)` bytes from the buffer,
and returns the copied count. It never uses `strlen` on the Intuition-edited
buffer; if the caller's buffer has room, it writes a trailing NUL. An unknown or
non-string id returns 0.

A `GADGETUP` on the string gadget reports the caller's id; the demo then calls
`windowString` to read the typed text.

### Menus

`cl_window_add_menu` validates the title and allocates a `cl_menu_node`,
`struct Menu`, and a fixed 65-byte title copy, all zeroed. It sets `MenuName`,
`Flags = MENUENABLED`, zero geometry, `FirstItem = NULL`, and `NextMenu = NULL`.
It appends the header to the window's menu list and returns its zero-based
index. The header is **not** attached yet: Intuition does not accept a menu with
an empty `FirstItem` chain. The new `struct Menu` starts with `NextMenu = NULL`,
`LeftEdge = index * 48`, `Width = 48`, and `Height = 11`; the wrapper's menu
list is separate from the `Menu.NextMenu` chain. The first `addMenuItem` on that
menu detaches the strip, rebuilds the `Menu.NextMenu` chain from the wrapper
list, links the item, and reattaches.

Every menu must receive at least one item before an item is added to a different
menu; otherwise `SetMenuStrip` sees an empty menu and can reject the strip. The
demo adds File's items before creating Help and Help's item. A menu that never
receives an item stays invisible.

`cl_window_add_menu_item` validates the menu index, label, and id, then
allocates a zeroed `cl_menu_item_node`, `struct MenuItem`, 65-byte label copy,
and `IntuiText`. It sets:

| Field | Value |
|---|---|
| `MenuItem.Width`, `Height` | 48, 11 |
| `MenuItem.Flags` | `ITEMTEXT | ITEMENABLED` |
| `MenuItem.ItemFill` | the label `IntuiText` (`FrontPen = 1`, `BackPen = 0`, `JAM1`) |
| `MenuItem.SelectFill` | `NULL` |
| `MenuItem.Command` | 0 |
| `MenuItem.SubItem` | `NULL` |
| `MenuItem.NextItem` | `NULL` |
| `MenuItem.MutualExclude` | 0 |

The item is appended to the menu's item chain: the first item becomes
`Menu.FirstItem`; each later item becomes the previous item's `NextItem`. The
strip is modified only while detached:

1. `ClearMenuStrip` if the strip is attached.
2. Link the item and rebuild `Menu.NextMenu` for every menu in the wrapper list.
3. `SetMenuStrip(window, first_menu)`.

On a false `SetMenuStrip`, unlink the new item, rebuild the chain, reattach the
previous strip, free the item and its label, and return `false`. The 3.x
autodoc says `SetMenuStrip` always waits and returns true; the 1.3 prototype
returns `BOOL`, so the defensive branch stays, but the strip is already
detached when it runs and no dangling `MenuStrip` remains.

On `MENUPICK`, the event loop calls
`ItemAddress(window->MenuStrip, msg->Code)`. If it returns a `MenuItem`, the loop
walks the window's item nodes to find the matching pointer and reports its id as
`WindowEvent.menu`. `MENUNULL` (0xFFFF) or an unknown item reports 0; the shim
checks `MENUNULL` before calling `ItemAddress`.

### Ownership and free order

`cl_close_window` (`03-01`) frees the gadget and menu nodes after
`CloseWindow`. It frees each string buffer with the size stored in the gadget
node (`string_buffer_size`, also `StringInfo.MaxChars`), string `Border` and
`XY`, button `Image` records, label copy (65 bytes), `IntuiText`, `StringInfo`,
`Gadget`, `MenuItem`, menu title copy (65 bytes), `Menu`, and node exactly once.
The string buffer is freed even when a later `StringInfo` allocation failed.
The menu strip is cleared before the window closes.

## Code Example

```clorraine
import { Screen, Window, openScreen, openWindow, addButton, addString,
         addMenu, addMenuItem, windowString } from std.amiga.intuition;
import { String, stringOf, toCString } from std.text;

func build(screen: Screen): Window? {
    let window: Window? = openWindow(screen, 24, 24, 272, 136,
                                      toCString(stringOf("Gadgets")));
    if (window == null) { return null; }
    if (!addButton(window, 8, 28, 80, 18,
                   toCString(stringOf("Press")), 1)) {
        return null;
    }
    if (!addString(window, 8, 56, 160, 18,
                   toCString(stringOf("Name:")),
                   toCString(stringOf("A500")), 32, 2)) {
        return null;
    }
    let file: i32 = addMenu(window, toCString(stringOf("File")));
    if (file < 0) { return null; }
    if (!addMenuItem(window, file, toCString(stringOf("Quit")), 11)) {
        return null;
    }
    return window;
}
```

## Error Handling

| Error Case | Handling | AR Ref |
|---|---|---|
| Null window or text | `false` / `-1` / 0 | AR-15 |
| Id below 1 or duplicate | `false` for add; event id 0 | AR-16 |
| Label or menu title over 64 bytes | `false` / `-1` | AR-14 |
| String over 64 characters or `maxChars` out of range | `false` | AR-14 |
| Gadget outside the inner window or `x`/`y` negative | `false` | AR-14 |
| Partial allocation failure | free every completed allocation, including the string buffer before `StringInfo`, return failure | AR-15 |
| Menu with no items | not attached; a later item add attaches it; a menu that never gets an item stays invisible | AR-15 |
| `SetMenuStrip` returns false | the strip is already detached; unlink the item, rebuild the chain, re-attach the previous strip, free, `false` | AR-15 |
| `MENUNULL` menu code | report 0 without calling `ItemAddress` | AR-16 |
| Unknown gadget or menu id at read time | 0 / 0 in the event | AR-16 |
| `AddGList` return | insertion position, not a status; ignored after validation | AR-15 |

## Testing Requirements

- Compile the full gadget/menu surface on both hosted profiles and cross-build
  with `nix13`/`nix20`.
- Pin `AddGList`, `RefreshGList`, `SetMenuStrip`, `ClearMenuStrip`, and
  `ItemAddress` in the emitted C and the shim.
- Pin the label/string limits and the `StringInfo.MaxChars` NUL rule in the shim
  implementation tests.
- Prove the button click, string edit, menu pick, and close on the A500 demo.
