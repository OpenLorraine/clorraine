---
title: Examples
description: "First Light, Prism, and Boing: live Amiga programs written in Clorraine, plus the acceptance programs behind the Amiga SDK."
---

# Examples

These are real programs from the repository. The first three run in your
browser; the rest are the acceptance programs that prove the Amiga SDK surface.

<div class="cl-example-grid">
  <ExampleCard
    href="/examples/first-light"
    src="/screenshots/first-light.webp"
    ratio="4 / 3"
    alt="First Light: a lighthouse on a dark headland beside a calm sea, under a glowing sunrise and a field of stars."
    title="First Light"
    description="A 16-colour dawn seascape with a dithered sky, a radial sun glow, and an animated reflection."
    meta="Live · A500 · 16 colours"
  />
  <ExampleCard
    href="/examples/prism"
    src="/screenshots/prism.webp"
    ratio="4 / 3"
    alt="Prism: a colourful brick-breaker title screen with a prism triangle and the word PRISM."
    title="Prism"
    description="A 32-colour brick-breaker with a title screen, score, lives, and a joystick-playable ADF."
    meta="Arcade demo · A500 · 32 colours"
  />
  <ExampleCard
    href="/examples/graphics"
    src="/screenshots/boing.webp"
    ratio="4 / 3"
    alt="Boing: a neon sunset, a perspective grid, and a chrome ball bouncing across the screen."
    title="Boing: Neon Horizon"
    description="A dithered sunset, a perspective grid, and a masked chrome ball over two chip-memory screens."
    meta="Live · A500 · 16 colours"
  />
</div>

## Technical demos

The programs below are acceptance tests, not showcase pieces. Some screens are
deliberately diagnostic: they prove a bitplane, collision, or palette path
rather than a visual style.

| Program | What it proves | Source |
|---|---|---|
| Intuition GUI | Screens, windows, gadgets, menus, requesters | [`examples/amiga/gui.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/gui.clo) |
| ILBM viewer | DOS, asset loading, ILBM decoding, custom screen | [`examples/amiga/imageviewer.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/imageviewer.clo) |
| Paddle | Audio channels, input, timers, double buffering | [`examples/amiga/paddle.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/paddle.clo) |
| Sprite demo | Hardware sprites and collision | [`examples/amiga/sprite-demo.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/sprite-demo.clo) |
| Scrolling text | Topaz font, layers, clipping | [`examples/amiga/scroller.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/scroller.clo) |
| Exec demo | Tasks, ports, messages, interrupts | [`examples/amiga/exec-demo.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/exec-demo.clo) |
| Hamurabi | A larger Amiga port with a third-party notice | [`examples/amiga/hamurabi.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/hamurabi.clo) |

## Learn the language

These pages walk through the smallest programs and the language surface:

- [Hello, Amiga](/examples/hello) — the smallest complete program and its
  generated C.
- [String parser](/examples/string-parser) — loops, enums, switch, strings, and
  hex formatting without library help.

## Running an example

From a checkout with the SDK:

```sh
make run FILE=examples/hello/hello.clo
make run FILE=examples/amiga/prism.clo \
  PROFILE=hosted-amiga-13 EMULATOR=1
```

The first runs on the host profile. The second builds for A500/Kickstart 1.3 and
boots FS-UAE. See the [quick start](/guide/quick-start) for the prerequisites.

## A note on the screenshots

The examples use the project's own custom screens. They do not use Amiga
Corporation or Hyperion logos, and the site does not use Workbench screenshots
as marketing images. See the [screenshot gallery](/amiga/screenshots).
