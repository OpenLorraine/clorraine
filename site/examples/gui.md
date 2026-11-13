---
title: Intuition GUI
description: "Clorraine Studio: a 640x256 hires, 16-colour Intuition dashboard on an A1200 / Kickstart 3.1 screen, with native windows, gadgets, and menus."
---

# Intuition GUI

**Clorraine Studio** is the visual Intuition showcase: a 640×256 hires,
16-colour screen with a native window, a string gadget, a button, and a menu.
It runs on an A1200 / Kickstart 3.1 and uses the system's own window borders,
gadgets, and menu strip.

<DocFigure
  src="/screenshots/gui31-showcase.png"
  alt="Clorraine Studio on Kickstart 3.1: a dark header with the Clorraine Studio wordmark, a sidebar, build-target cards, a progress bar, and a recent-output panel."
  caption="Native capture from an A1200 / Kickstart 3.1 run. 640×256 hires, 16 colours."
/>

## What it exercises

| Capability | Use in the demo |
|---|---|
| Screen | A 640×256 hires custom screen with 4 bitplanes and 16 colours |
| Window | Native Kickstart 3.1 window borders, drag bar, and close gadget |
| Gadgets | A string field and a Build button |
| Menu | A Project menu with Build and Quit items |
| Layout | A sidebar, target cards, a progress bar, and a recent-output panel |
| Event loop | `WaitPort` / `GetMsg` until the close gadget or Quit |
| Palette | Sixteen pens chosen for a dark navy, paper, amber, and green UI |

The acceptance GUI (`examples/amiga/gui.clo`) is a separate, gated program. It
keeps the 640×256 / 8-colour A500 / Kickstart 1.3 baseline and exercises
requesters and command-key menus. This page shows the visual showcase.

## A source excerpt

```clorraine
let screen: Screen? = openScreenHires(
    640, 256, 4, toCString(screenTitle));
if (screen == null) {
    return 1;
}

let window: Window? = openWindow(screen, 24, 24, 592, 220,
                                 toCString(windowTitle));
if (window == null) {
    return 1;
}
```

The full program is
[`examples/amiga/gui-showcase.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/gui-showcase.clo).

## Run it

```sh
make run FILE=examples/amiga/gui-showcase.clo \
  PROFILE=hosted-amiga EMULATOR=1
```

The `hosted-amiga` profile uses the configured Kickstart ROM; the screenshot
was taken with an A1200 / Kickstart 3.1 ROM. The program needs a display, so use
FS-UAE rather than `vamos`.

## What to inspect

- The window borders, close gadget, string field, and button are native
  Intuition objects, not a custom toolkit.
- The screen is hires, so a 640-pixel-wide window fits without the doubled-pixel
  clipping of a low-resolution screen.
- The palette is sixteen pens: dark navy, paper, amber, green, and a small
  accent ramp.
- The event loop is a normal `while` loop over `waitEvent`; there is no hidden
  GUI runtime.
- The program closes its window before exiting.

## Learn more

- [SDK coverage](/amiga/sdk-coverage)
- [Amiga profiles](/amiga/profiles)
- [Emulator workflow](/amiga/emulator)
- [`std.amiga.intuition`](https://github.com/OpenLorraine/clorraine/blob/main/stdlib/std.amiga.intuition.clo)
