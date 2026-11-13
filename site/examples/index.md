---
title: Examples
description: "Real Clorraine programs: hello world, a string parser, Boing graphics, and an Intuition GUI."
---

# Examples

These are not sketches. They are programs from the repository, built by the
compiler and, where noted, run under `vamos` or FS-UAE.

<div class="cl-example-grid">
  <ExampleCard
    href="/examples/hello"
    src="/screenshots/hello-card.png"
    alt="A terminal panel showing hello.clo and its output."
    title="Hello, Amiga"
    description="The smallest complete program, its output, and a generated C excerpt."
    meta="Start here"
  />
  <ExampleCard
    href="/examples/string-parser"
    src="/screenshots/parser-card.png"
    alt="A terminal panel showing the string parser's enum and output."
    title="String parser"
    description="Loops, enums, switch, strings, and hex formatting without library help."
    meta="Language tour"
  />
  <ExampleCard
    href="/examples/graphics"
    src="/screenshots/a500-boing.webp"
    alt="The Clorraine Boing demo on an Amiga screen."
    title="Boing graphics"
    description="Chip memory, copper, blitter, double buffering, and a frame checksum."
    meta="Amiga SDK"
  />
  <ExampleCard
    href="/examples/gui"
    src="/screenshots/a500-gui.webp"
    alt="The Clorraine Intuition GUI running on an Amiga screen."
    title="Intuition GUI"
    description="Screens, windows, gadgets, menus, requesters, and a keyboard-driven event loop."
    meta="Amiga SDK"
  />
</div>

## More programs in the repository

| Program | What it shows | Source |
|---|---|---|
| ILBM viewer | DOS, asset loading, ILBM decoding, custom screen | [`examples/amiga/imageviewer.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/imageviewer.clo) |
| Paddle game | Audio channels, input, timers, double buffering | [`examples/amiga/paddle.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/paddle.clo) |
| Sprite demo | Hardware sprites and collision | [`examples/amiga/sprite-demo.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/sprite-demo.clo) |
| Scrolling text | Topaz font, layers, clipping | [`examples/amiga/scroller.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/scroller.clo) |
| Exec demo | Tasks, ports, messages, interrupts | [`examples/amiga/exec-demo.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/exec-demo.clo) |
| Hamurabi | A larger Amiga port with a third-party notice | [`examples/amiga/hamurabi.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/hamurabi.clo) |

## Running an example

From a checkout with the SDK:

```sh
make run FILE=examples/hello/hello.clo
make run FILE=examples/amiga/boing.clo PROFILE=hosted-amiga-13 EMULATOR=1
```

The first runs on the host profile. The second builds for A500/Kickstart 1.3 and
boots FS-UAE. See the [quick start](/guide/quick-start) for the prerequisites.

## A note on the screenshots

The examples use the project's own custom screens. They do not use Amiga
Corporation or Hyperion logos, and the site does not use Workbench screenshots
as marketing images. See the [screenshot gallery](/amiga/screenshots).
