# GUI Demo and Smoke: Amiga SDK Bundle 6

> **Document**: 03-04-gui-demo-and-smoke.md
> **Parent**: [Index](00-index.md)
> **Covers**: R9–R10, R13; AR-5, AR-18–AR-20, AR-25

## Overview

`examples/amiga/gui.clo` is the acceptance program. It opens a light
640x256x3 custom screen and a 576x208 window, builds two gadgets and two menus,
shows Yes/No requesters for the main action and the About item, installs the
built-in pointer, and writes a native screen capture plus a deterministic report
line. The smoke boots it on the real A500/KS1.3 machine under FS-UAE, drives it
with the host keyboard on a private Xvfb display, and extracts the capture and
the report from the writable ADF. No host-window screenshot is used: the app
captures its own Amiga bitmap, so the artifact has the native resolution and is
independent of emulator scaling or cropping.

## Demo layout

### Screen and palette

| Item | Value |
|---|---|
| Screen | 640x256, depth 3 (eight colours), title `Clorraine` |
| Pen 0 | desktop `(150, 165, 190)` |
| Pen 1 | ink `(20, 30, 50)` |
| Pen 2 | accent `(240, 130, 20)` |
| Pen 3 | success `(0, 140, 70)` |
| Pen 4 | paper `(252, 252, 250)` |
| Pen 5 | muted `(80, 110, 160)` |
| Pen 6 | panel `(226, 231, 240)` |
| Pen 7 | white `(255, 255, 255)` |
| Pen 17 | pointer white |
| Pen 18 | pointer black |

The screen clears to pen 0. A footer strip at y = 240..255 uses pen 1 with pen 4
text based at y = 252; the right-hand text is kept inside the 640-pixel row so
it cannot wrap onto the left footer. The window interior uses pen 4 (paper), a 4-pixel pen-2 accent stripe on
the left, and pen-6 panels for the badge and status box. The palette is set
before the window opens, so Intuition's title bar picks up the screen pens.

### Setup

The demo binds the screen title to a local `String` and keeps it alive for the
screen's lifetime:

```clorraine
let screenTitle: String = stringOf("Clorraine");
let screen: Screen? = openScreen(640, 256, 3, toCString(screenTitle));
if (screen == null) { return failedAt(1); }
```

It opens the ROM Topaz font and falls back to `defaultFont`; a null font is a
stage-8 failure before any window work.

### Window

| Item | Value |
|---|---|
| Outer rectangle | `(32, 32)` size `576x208` |
| Title | `Clorraine GUI` (copied by `openWindow`) |
| System gadgets | close, drag, depth |
| Background | `windowClear(window, 4)` |
| Inner width / height | `windowWidth(window)` / `windowHeight(window)` |
| Header | `Clorraine GUI demo` at (20, 14), pen 1 |
| Subtitle | `A500 / Kickstart 1.3` at (20, 30), pen 5 |
| Badge | pen-6 box at `(w - 190, 14)..(w - 20, 38)`, text `8 colours - 640x256` |
| Separator | `windowDraw(window, 20, 50, w - 20, 50, 6)` |
| Command hint | `H Hello   A About   Q Quit` at (20, 152), pen 5 |
| Status box | pen-6 fill at `(12, h - 32)..(w - 12, h - 10)` with pen-5 borders |

Only the window's inner coordinates are used. The fixed 576x208 outer size gives
`GZZWidth`/`GZZHeight` of roughly 574x196 on KS1.3.

### Gadgets and menus

| Element | Geometry | Label / initial | Id |
|---|---|---|---|
| Button | `(20, 94)` size `150x24` | `Say hello` | 1 |
| String | `(72, 62)` size `320x20` | empty, max 32 characters | 2 |
| File → Hello | menu 0 | `Hello`, Right Amiga+H | 10 |
| File → Quit | menu 0 | `Quit`, Right Amiga+Q | 11 |
| Help → About | menu 1 | `About`, Right Amiga+A | 20 |

The string starts empty so the smoke's `abc` typing yields exactly three bytes.
The button uses the pen-1 fill and pen-2 select image; the label is pen 0. The
string gadget uses a pen-1 `Border` and a `StringInfo` with `MaxChars = 33`.

The string gadget sets **both** `GACT_IMMEDIATE` and `GACT_RELVERIFY`.
Kickstart only sends `IDCMP_GADGETUP` for a string gadget that has
`GACT_RELVERIFY`; without it, Return deactivates the field silently.

### Event flow

| Event | Action |
|---|---|
| `EVENT_CLOSE` | exit the loop |
| `EVENT_GADGET` id 2 | read the string into a 64-byte buffer; then fall through to the requester |
| `EVENT_GADGET` id 1 or 2 | `askYesNo(window, "Continue?")`; set status `yes`/`no`; increment `req` only on Yes |
| `EVENT_MENU` id 10 | set status `hello` |
| `EVENT_MENU` id 11 | exit the loop |
| `EVENT_MENU` id 20 | `askYesNo(window, "Clorraine GUI / A500")`; increment `req` on Yes; set status `about` |
| `EVENT_MOUSE`, `EVENT_KEY`, `EVENT_ACTIVE`, `EVENT_INACTIVE` | ignore |

Before the blocking loop, the demo drains pending messages with `pollEvent`
until it returns `EVENT_NONE`, which exercises the empty-queue contract. The
loop then waits with `waitEvent` and redraws the status line after each handled
event. The window layer preserves the content; no resize is possible.

The About item deliberately uses `askYesNo`, not `DisplayAlert`. `showAlert` is
implemented, spec-tested, and documented, but a `DisplayAlert` opens a private
modal screen that the headless harness cannot reliably focus and dismiss. The
requester keeps the automated acceptance on one screen and one input path.
`showAlert` remains part of the API for real applications that need a modal
system alert.

### Negative checks

The demo runs these before the main window work. It prints `gui: err <n>` and
returns 1 if any expectation fails:

1. `openScreen` fails → stage 1.
2. A throwaway `openWindow` with `x = -1` returns null → stage 2.
3. The real `openWindow` returns null → stage 3.
4. `addButton(window, 1000, 0, 20, 20, "x", 1)` returns false → stage 4.
5. `addString(window, 0, 0, 20, 20, "x", "y", 0, 2)` returns false → stage 5.
6. `addButton(window, 0, 0, 20, 20, "x", 0)` returns false (id 0) → stage 6.
7. `addMenu(window, overlongTitle)` returns `-1` → stage 7.
8. `openFont` and `defaultFont` both fail → stage 8.
9. `addString` with the real geometry fails → stage 9.
10. `addButton` with the real geometry fails → stage 10.
11. `addMenu("File")` does not return 0 → stage 11.
12. `addMenuItemKey("Hello", 10, 'H')` fails → stage 12.
13. `addMenuItemKey("Quit", 11, 'Q')` fails → stage 13.
14. `addMenu("Help")` does not return 1 → stage 14.
15. `addMenuItemKey("About", 20, 'A')` fails → stage 15.
16. `activateGadget(window, 2)` fails → stage 16.
17. `saveScreen(screen, "DF0:gui.cscr")` fails → stage 17.

### Native capture

`saveScreen` writes the `CSCR` format described in
`stdlib/std.amiga.intuition.clo`: the four ASCII bytes `CSCR`, the width,
height, depth, and bytes per row as big-endian 16-bit values, the palette as
8-bit RGB triples, then the raw bitplanes. For the 640x256x3 screen the file is
61,476 bytes, small enough to write to a floppy in a bounded time. The smoke
enables FS-UAE turbo floppy mode while it runs so the capture and the report do
not stall the event loop.

`docker/scripts/cscr2png.py` converts the capture to a PNG with the Python
standard library. The smoke writes the committed golden at 2x
nearest-neighbour scale (`1280x512`) so the 8-pixel Topaz text stays readable.

### Report line

```text
gui: ok win 1 gad 2 menu 2 str 3 req 2
```

`win` is the opened window; `gad` is the number of successfully added gadgets;
`menu` is the number of menu headers (not items); `str` is the byte count read
from the string gadget; `req` is the number of Yes answers (the Continue
requester and the About requester). The line is 38 bytes. It is printed on the
Amiga console and written to `DF0:gui.txt`; the smoke polls the raw ADF for the
line so it can stop after the write commits.

## A500 smoke

### Build and boot

`make sdk-a500-gui` builds `examples/amiga/gui.clo` for `hosted-amiga-13` with
`--release`, copies the binary to `S/gui` on a minimal ADF, decrypts the
licensed KS1.3 ROM locally, and boots FS-UAE as an A500 with a 640x256 window,
scale 1, and turbo floppy mode. The harness runs FS-UAE on a private Xvfb
display, so no emulator window appears on the user's desktop.

### Input mapping

FS-UAE maps host keys by physical position:

| Amiga key | Host key |
|---|---|
| Left Amiga | Left Windows/Super (`Super_L`) |
| Right Amiga | Right Windows/Super, Page Down, or Menu (`Menu`) |
| Left Alt | `Alt_L` |

The smoke uses `Super_L` for Left Amiga and `Menu` for Right Amiga. It types
`abc`, presses Return to end the string edit, answers the Continue requester
with Left Amiga+V, selects File→Hello with Right Amiga+H, selects Help→About
with Right Amiga+A, answers that requester with Left Amiga+V, and selects
File→Quit with Right Amiga+Q.

### Interaction sequence

1. Wait for the `gui-ready` handshake in the raw ADF, which proves the UI is
   built, the native capture is committed, and the event loop is ready.
2. Type `abc`, press Return, and capture the requester frame.
3. Left Amiga+V answers Yes; the status changes to `yes`.
4. Right Amiga+H selects File→Hello; the status changes to `hello`.
5. Right Amiga+A selects Help→About; Left Amiga+V answers Yes; the status
   changes to `about`.
6. Right Amiga+Q selects File→Quit.
7. Poll the raw ADF for `gui: ok win 1`, then extract `gui.cscr` and `gui.txt`.
8. Convert `gui.cscr` to PNG and compare it with `docs/amiga-sdk/a500-gui.png`;
   compare `gui.txt` with the expected line.

The smoke retries each shortcut until the next state is visible. Fixed sleeps
alone are not reliable because the ADF writes and requester transitions can take
several seconds.

### Goldens and evidence

| Evidence | Path |
|---|---|
| Held window frame, native 2x | `docs/amiga-sdk/a500-gui.png` |
| Report line text | `DF0:gui.txt` inside the ADF; asserted by the smoke |
| Requester frame (diagnostic) | `build/amiga-sdk/gui-smoke/requester.png` (gitignored) |
| Last failure screen | `build/amiga-sdk/gui-smoke/screen.png` (gitignored) |

The alert golden is no longer required because the interactive About path uses a
requester. `DisplayAlert` is proved by the spec suite and the host C probe, not
by the headless A500 interaction.

### Registry closeout

| Row | Status | Acceptance | Evidence | Notes |
|---|---|---|---|---|
| SDK-INT-001 | done | `int_screens` | `stdlib/std.amiga.intuition.clo; amiga_intuition_spec_test; a500-gui-smoke` | Bundle 6 |
| SDK-INT-002 | done | `int_gadgets` | `stdlib/std.amiga.intuition.clo; a500-gui-smoke` | Bundle 6 |
| SDK-INT-003 | done | `int_events` | `stdlib/std.amiga.intuition.clo; a500-gui-smoke` | Bundle 6 |
| SDK-INT-004 | done | `int_requesters` | `stdlib/std.amiga.intuition.clo; amiga_intuition_spec_test` | Bundle 6 |
| SDK-INT-005 | done | `int_pointer` | `stdlib/std.amiga.intuition.clo; a500-gui.png` | Bundle 6 |
| SDK-INT-006 | deferred | `gadtools` | `-` | 2.0+ advanced; Bundle 9 |
| SDK-INT-007 | deferred | `asl` | `-` | 2.0+ advanced; Bundle 9 |
| SDK-INT-008 | deferred | `boopsi` | `-` | 3.0+ optional; Bundle 9 |
| SDK-INT-009 | done | `gui_demo` | `docs/amiga-sdk/a500-gui.png; make sdk-a500-gui` | Bundle 6 |

Set `bundles.tsv` row `6-intuition` to `done`, plan
`plans/amiga-sdk-intuition`, and regenerate `coverage.md` with
`make sdk-registry`. The `spec` column is `spec/appendix-stdlib.md` for the six
done rows; the demo row also cites `spec/appendix-amiga.md`.

### Memory budget

| Item | Bytes |
|---|---|
| Screen bitmap (640x256x3) | 61,440 |
| Window layer (576x208x3) | ~44,928 |
| Gadgets, menus, labels, string buffer | < 4,000 |
| Pointer | 64 |
| Release program | ~74,000 |
| Stack and runtime | ~16,000 |
| **Total** | **~200,000 of 512 KB** |

The demo uses one window and no double buffer, so the budget has wide margin.
The 576x208 window fits because the old 320x256 implementation cap is gone.

## Failure modes and handling

| Failure | Handling |
|---|---|
| Window never appears | The smoke keeps the last screenshot and exits nonzero |
| A shortcut is lost because the window is inactive | Each step refocuses the current FS-UAE window and retries until the next ADF marker or the report appears |
| The string gadget never sends GADGETUP | The shim sets `GACT_RELVERIFY`; the smoke's Return step would leave the app waiting and time out |
| The menu command key does not fire | The shim sets `COMMSEQ` and writes `Command` after the item is linked; the smoke retries Right Amiga+H/A/Q |
| A `DisplayAlert` private screen blocks the harness | The demo uses requesters for the interactive About path; `showAlert` stays spec-tested |
| The native capture stalls the event loop | The capture is 61 KB of bitplanes, and the smoke enables turbo floppy mode |
| The report line wraps | The line is 38 characters; the file is plain text, so no screen crop is involved |
| The demo runs out of chip RAM | The acceptance build is `--release`; the budget above leaves margin |
| A gadget is invisible | The button uses a classic `Image` render pair and the string field uses a `Border`; the native frame confirms both |

## Testing requirements

- The demo compiles and cross-builds on both hosted profiles.
- The demo contains the negative checks, the `pollEvent` drain, the
  `saveScreen` capture, and the exact report-line format.
- The A500 smoke asserts the native frame, the report line, and the quit path,
  and `make verify` runs it after the six earlier A500 gates.
- The golden is committed and the last failure screen stays gitignored.
