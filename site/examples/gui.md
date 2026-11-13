---
title: Intuition GUI
description: "A Clorraine Intuition demo with screens, windows, gadgets, menus, requesters, and a keyboard-driven event loop on A500."
---

# Intuition GUI

The GUI demo opens an Intuition screen and window, adds gadgets and menus, and
runs a small event loop. It is built for the A500/Kickstart 1.3 profile and
driven headlessly by the acceptance gate.

<DocFigure
  src="/screenshots/a500-gui.webp"
  alt="A Clorraine Intuition window on an Amiga screen with a name field, a Say hello button, and a ready status line."
  caption="Native capture from the A500 / Kickstart 1.3 gate, 640×256, 8 colours."
/>

## What it exercises

| Capability | Use in the demo |
|---|---|
| Screen | A 640×256, 8-colour custom screen |
| Window | Title bar, borders, and a close gadget |
| Gadgets | A string gadget and a button |
| Menus | A menu strip with items and shortcuts |
| Requesters | A Yes/No requester opened from the keyboard |
| Event loop | Keyboard and gadget messages until quit |
| Report line | A deterministic line the gate compares |

## A source excerpt

```clorraine
let screenTitle: String = stringOf("Clorraine");
let screen: Screen? = openScreen(640, 256, 3, toCString(screenTitle));
if (screen == null) {
    return failedAt(1);
}
```

The event loop handles the window's messages and exits on the right-Amiga `Q`
shortcut. The full program is
[`examples/amiga/gui.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/gui.clo).

## Run it

The headless acceptance gate:

```sh
make sdk-a500-gui
```

It builds the demo for `hosted-amiga-13`, boots FS-UAE on a private Xvfb display,
drives the demo with Amiga-key shortcuts, captures the app's native screen, and
compares the report line.

For an interactive window:

```sh
make run FILE=examples/amiga/gui.clo PROFILE=hosted-amiga-13 EMULATOR=1
```

Then use the right-Amiga shortcuts shown in the window: `H` for hello, `A` for
about, and `Q` to quit.

## The gate line

```text
gui: ok win 1 gad 2 menu 2 str 3 req 2
```

The numbers record the objects created and exercised. The gate also compares a
native screen capture, so a window that opens but does not draw is not enough.

## What to inspect

- The screen is 8 colours and 640×256, which fits the A500/OCS baseline.
- The gadgets and menus are native Intuition objects, not a custom toolkit.
- The event loop is a normal `while` loop over `WaitPort`/`GetMsg`; there is no
  hidden GUI runtime.
- The program closes its window and screen before reporting.

## Learn more

- [SDK coverage](/amiga/sdk-coverage)
- [Amiga profiles](/amiga/profiles)
- [Emulator workflow](/amiga/emulator)
- [`std.amiga.intuition`](https://github.com/OpenLorraine/clorraine/blob/main/stdlib/std.amiga.intuition.clo)
