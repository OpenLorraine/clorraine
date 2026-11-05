# GUI Demo and Smoke: Amiga SDK Bundle 6

> **Document**: 03-04-gui-demo-and-smoke.md
> **Parent**: [Index](00-index.md)
> **Covers**: R9–R10, R13; AR-5, AR-18–AR-20

## Overview

`examples/amiga/gui.clo` is the acceptance program. It opens a 320x200x4 custom
screen and a window, builds two gadgets and two menus, shows a Yes/No requester
and an alert, installs the built-in pointer, runs an event loop, and prints a
deterministic report line after the screen and window close. `make sdk-a500-gui`
boots it on the real A500/KS1.3 machine, drives it with `xdotool`, and compares
the held frame and the report line to committed goldens.

## Demo layout

### Screen and palette

| Item | Value |
|---|---|
| Screen | 320x200, depth 4, title `Clorraine GUI` |
| Pen 0 | dark background `(8, 8, 24)` |
| Pen 1 | light text and button fill `(220, 220, 220)` |
| Pen 2 | select highlight `(255, 128, 0)` |
| Pen 3 | status text `(0, 200, 80)` |
| Pen 17 | pointer white `(255, 255, 255)` |
| Pen 18 | pointer black `(0, 0, 0)` |

Pen 16 is the unused pointer-color-0 entry; pen 19 is unused. The screen keeps
the 4-bit bitmap palette at pens 0–3 and the sprite palette at 16–19.

### Setup

The demo binds the screen title to a local `String` and keeps it alive for the
screen's lifetime:

```clorraine
let screenTitle: String = stringOf("Clorraine GUI");
let screen: Screen? = openScreen(320, 200, 4, toCString(screenTitle));
if (screen == null) { return 1; }
```

It opens the ROM Topaz font and falls back to `defaultFont`; a null font returns
an error before any window work:

```clorraine
let font: Font? = openFont(toCString(stringOf("topaz.font")), 8, 0);
if (font == null) { font = defaultFont(); }
if (font == null) { return 2; }
```

`windowText` takes a `u8[]` view, not a string literal. The demo calls
`windowText(window, font, x, y, pen, bytes(stringOf("...")));`; the draw is
synchronous, so the temporary `String` does not outlive its use.

### Window

| Item | Value |
|---|---|
| Outer rectangle | `(24, 24)` size `272x136` |
| Title | `Clorraine GUI` (copied by `openWindow`) |
| System gadgets | close, drag, depth |
| Background | `windowClear(window, 0)` |
| Title text | `windowText(window, font, 8, 8, 1, bytes(stringOf("Clorraine GUI demo")))` |
| Separator | `windowDraw(window, 8, 22, windowWidth(window) - 9, 22, 1)` |
| Status box | `windowFill(window, 8, windowHeight(window) - 20, windowWidth(window) - 9, windowHeight(window) - 9, 0)` |
| Status text | `windowText(window, font, 8, windowHeight(window) - 12, 3, bytes(stringOf("ready")))` |

The inner-size and drawing calls are part of the demonstration, not only the
compile surface. The status box and separator are visible in the held frame.

### Gadgets and menus

| Element | Geometry | Label / initial | Id |
|---|---|---|---|
| Button | `(8, 28)` size `80x18` | `Press` | 1 |
| String | `(8, 56)` size `160x18` | `Name:` / empty | 2 |
| File → Hello | menu 0 | `Hello` | 10 |
| File → Quit | menu 0 | `Quit` | 11 |
| Help → About | menu 1 | `About` | 20 |

The string starts empty so the smoke's `abc` typing yields exactly three bytes.
The button uses the pen-1 fill and pen-2 select image; the label is pen 0.

### Event flow

| Event | Action |
|---|---|
| `EVENT_CLOSE` | exit the loop (the smoke clicks the close gadget) |
| `EVENT_GADGET` id 1 | `askYesNo(window, "Continue?")`; set status `yes`/`no`; increment `req` only when the answer is Yes |
| `EVENT_GADGET` id 2 | `windowString(window, 2, buf)` into a 64-byte buffer; set status to the fixed label `name`; store the byte count |
| `EVENT_MENU` id 10 | set status `hello` |
| `EVENT_MENU` id 11 | exit the loop (the smoke uses the close gadget instead) |
| `EVENT_MENU` id 20 | `showAlert("Clorraine GUI")`; set status `about` |
| `EVENT_MOUSE`, `EVENT_KEY`, `EVENT_ACTIVE`, `EVENT_INACTIVE` | ignore |

Before the blocking loop, the demo drains pending messages with `pollEvent`
until it returns `EVENT_NONE`, which exercises the empty-queue contract. The
loop then waits with `waitEvent` and redraws the status line after each handled
event. The window layer preserves the content; no resize is possible.

### Negative checks

The demo first attempts a throwaway `openWindow(screen, -1, 0, 100, 100,
title)` and requires `null`. It then opens the real window and runs the
remaining checks; it prints `gui: err <n>` and returns 1 if any expectation
fails:

1. The throwaway `openWindow` with `x = -1` returns `null` (before the real
   window opens).
2. `addButton(window, 1000, 0, 20, 20, "x", 1)` returns `false`.
3. `addString(window, 0, 0, 20, 20, "x", "y", 0, 2)` returns `false`.
4. `addButton(window, 0, 0, 20, 20, "x", 0)` returns `false` (id 0).
5. `addMenu(window, overlongTitle)` returns `-1`; the demo builds a 70-byte
   title with `stringOf` and `toCString`.

The implementation suite pins the same limits on the host side.

### Report line

```text
gui: ok win 1 gad 2 menu 2 str 3 req 1
```

`win` is the opened window; `gad` is the successfully added gadgets; `menu` is
the number of menu headers (not items); `str` is the byte count read from the
string gadget; `req` is the number of Yes answers to the requester. The line is
38 bytes. It is printed by `main` after `runGui` returns, so the window and
screen drops have already restored the shell.

## A500 smoke

### Build and boot

`make sdk-a500-gui` builds `examples/amiga/gui.clo` for `hosted-amiga-13` with
`--release`, copies the binary to `S/gui` on a minimal ADF, decrypts the licensed
KS1.3 ROM locally, and boots FS-UAE as an A500, mirroring
`docker/scripts/a500-assets-smoke.sh`. The release build keeps the executable and
the window layer inside the 512 KB machine.

### Interaction sequence

1. Wait for the FS-UAE window; activate/focus it with
   `xdotool windowactivate --sync`, move the pointer to a fixed inner position,
   and capture the held frame. Compare `docs/amiga-sdk/a500-gui.png`; require
   two consecutive zero-diff captures.
2. Left-click the Press button. The requester appears; left-click Yes.
3. Left-click the string gadget, type `abc`, and press Return.
4. Open the Help menu with a right-button press-drag-release:
   `mousemove --window` to the Help header, `mousedown 3`, move to About,
   `mouseup 3`. The alert appears; capture and compare
   `docs/amiga-sdk/a500-gui-alert.png`, then left-click Continue.
5. Open the File menu the same way and choose Hello; this proves a menu pick.
6. Left-click the window close gadget, which sends `EVENT_CLOSE`.
7. Poll the shell line and compare `docs/amiga-sdk/a500-gui-line.png`.

Classic Kickstart 1.3 menus use the right mouse button, not a left click. The
exact host coordinates are calibrated once on the first A500 run and then pinned
in the script. The script keeps the last screenshot on failure and prints the
FS-UAE log tail when the window is missing. The poll budget starts at 30 seconds
and uses 5-second steps, like the Bundle 5 assets smoke; the interaction steps
sleep briefly and verify the expected next frame before each click so a lost
focus or a slow message does not desynchronise the sequence.

### Goldens and evidence

| Evidence | Path |
|---|---|
| Held window frame | `docs/amiga-sdk/a500-gui.png` |
| Alert frame | `docs/amiga-sdk/a500-gui-alert.png` |
| Report line | `docs/amiga-sdk/a500-gui-line.png` |
| Requester frame (evidence, not golden) | `build/amiga-sdk/gui-smoke/requester.png` |
| Last failure screen | `build/amiga-sdk/gui-smoke/screen.png` (gitignored) |

The alert golden is required: a silent `showAlert` no-op would otherwise let the
line match. The requester is proved by `req 1` only when the user answered Yes;
a missing requester leaves the loop blocked and the smoke times out.

### Registry closeout

| Row | Status | Acceptance | Evidence | Notes |
|---|---|---|---|---|
| SDK-INT-001 | done | `int_screens` | `stdlib/std.amiga.intuition.clo; amiga_intuition_spec_test; a500-gui-smoke` | Bundle 6 |
| SDK-INT-002 | done | `int_gadgets` | `stdlib/std.amiga.intuition.clo; a500-gui-smoke` | Bundle 6 |
| SDK-INT-003 | done | `int_events` | `stdlib/std.amiga.intuition.clo; a500-gui-smoke` | Bundle 6 |
| SDK-INT-004 | done | `int_requesters` | `stdlib/std.amiga.intuition.clo; a500-gui-alert.png` | Bundle 6 |
| SDK-INT-005 | done | `int_pointer` | `stdlib/std.amiga.intuition.clo; a500-gui.png` | Bundle 6 |
| SDK-INT-006 | deferred | `gadtools` | `-` | 2.0+ advanced; Bundle 9 |
| SDK-INT-007 | deferred | `asl` | `-` | 2.0+ advanced; Bundle 9 |
| SDK-INT-008 | deferred | `boopsi` | `-` | 3.0+ optional; Bundle 9 |
| SDK-INT-009 | done | `gui_demo` | `docs/amiga-sdk/a500-gui*.png; make sdk-a500-gui` | Bundle 6 |

Set `bundles.tsv` row `6-intuition` to `done`, plan
`plans/amiga-sdk-intuition`, and regenerate `coverage.md` with
`make sdk-registry`. The `spec` column is `spec/appendix-stdlib.md` for the six
done rows; the demo row also cites `spec/appendix-amiga.md`.

### Memory budget

| Item | Bytes |
|---|---|
| Screen bitmap (320x200x4) | 32,000 |
| Window layer | ~32,000 |
| Gadgets, menus, labels, string buffer | < 4,000 |
| Pointer | 64 |
| Release program (estimate, Bundle 5 was 74,004) | ~80,000 |
| Stack and runtime | ~16,000 |
| **Total** | **~164,000 of 512 KB** |

The demo uses one window and no double buffer, so the budget has wide margin.

## Failure modes and handling

| Failure | Handling |
|---|---|
| Window never appears | The smoke keeps the last screenshot and exits nonzero |
| A click misses because the window moved or lost focus | The window is fixed at `(24,24)`; the smoke activates it, uses fixed coordinates, and checks the expected frame before each step |
| A classic menu is not selected by a left click | The smoke uses a right-button press-drag-release for every menu |
| `DisplayAlert` covers the custom screen | The smoke matches the alert golden before clicking Continue; the window layer restores the frame |
| The report line wraps | The line is 38 characters; the crop is calibrated to the full line |
| The demo runs out of chip RAM | The acceptance build is `--release`; the budget above leaves margin |
| A gadget is invisible | The button uses a classic `Image` render pair and the string field uses a `Border`; the first A500 frame confirms both |

## Testing requirements

- The demo compiles and cross-builds on both hosted profiles.
- The demo contains the negative checks, the `pollEvent` drain, the close-gadget
  exit, and the exact report-line format.
- The A500 smoke compares the held frame, the alert frame, and the line, and
  `make verify` runs it after the six earlier A500 gates.
- The goldens are committed and the last failure screen stays gitignored.
