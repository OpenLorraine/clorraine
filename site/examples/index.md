---
title: Examples
description: "First Light, a live Amiga demo written in Clorraine, plus the acceptance programs behind the Amiga SDK."
---

# Examples

These are real programs from the repository. The first one runs live in your
browser; the rest are the acceptance programs that prove the Amiga SDK surface.

## First Light

<DocFigure
  src="/screenshots/first-light.webp"
  alt="First Light: a lighthouse on a dark headland beside a calm sea, under a glowing sunrise and a field of stars."
  caption="First Light · A500 · 320×256 · 16 colours · compiled from one .clo file."
/>

A dawn seascape with an ordered-dither sky, a radial sun glow, chip-memory
bitplanes, and a broken water reflection. The moving water is pre-rendered into
eight phases, so a frame costs one blit and one flip on a 7 MHz 68000.

[Run First Light live →](/examples/first-light) ·
[Read the source](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/first-light.clo) ·
[Download the ADF](/demos/first-light.adf)

## Technical demos

The programs below are acceptance tests, not showcase pieces. Some screens are
deliberately diagnostic: they prove a bitplane, collision, or palette path
rather than a visual style.

| Program | What it proves | Source |
|---|---|---|
| Boing | Chip memory, bitmaps, copper, masked blits, double buffering | [`examples/amiga/boing.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/boing.clo) |
| Intuition GUI | Screens, windows, gadgets, menus, requesters | [`examples/amiga/gui.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/gui.clo) |
| ILBM viewer | DOS, asset loading, ILBM decoding, custom screen | [`examples/amiga/imageviewer.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/imageviewer.clo) |
| Paddle | Audio channels, input, timers, double buffering | [`examples/amiga/paddle.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/paddle.clo) |
| Sprite demo | Hardware sprites and collision | [`examples/amiga/sprite-demo.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/sprite-demo.clo) |
| Scrolling text | Topaz font, layers, clipping | [`examples/amiga/scroller.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/scroller.clo) |
| Exec demo | Tasks, ports, messages, interrupts | [`examples/amiga/exec-demo.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/exec-demo.clo) |
| Hamurabi | A larger Amiga port with a third-party notice | [`examples/amiga/hamurabi.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/hamurabi.clo) |

## Running an example

From a checkout with the SDK:

```sh
make run FILE=examples/hello/hello.clo
make run FILE=examples/amiga/first-light.clo \
  PROFILE=hosted-amiga-13 EMULATOR=1
```

The first runs on the host profile. The second builds for A500/Kickstart 1.3 and
boots FS-UAE. See the [quick start](/guide/quick-start) for the prerequisites.

## A note on the screenshots

The examples use the project's own custom screens. They do not use Amiga
Corporation or Hyperion logos, and the site does not use Workbench screenshots
as marketing images. See the [screenshot gallery](/amiga/screenshots).
