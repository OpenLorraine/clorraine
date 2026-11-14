---
title: Prism
description: "Prism: a playable 32-colour brick-breaker written in Clorraine, running in your browser on an emulated Amiga 500."
---

# Prism

A small, complete brick-breaker written in Clorraine. **Prism** opens a
320×256, 32-colour custom display, draws a title screen, a six-digit score, a
row of coloured bricks, a glossy paddle, and a ball that bounces through the
wall. The live view shows the title screen; the ADF is joystick-playable on a
real A500.

<AmigaPlayer
  poster="/screenshots/prism.webp"
  state="/demos/prism.clstate"
  adf="/demos/prism.adf"
  badge="A500 · 32 colours"
  caption="Prism · a Clorraine arcade game"
  hint="Runs in your browser with Copperline and AROS. The saved machine sits on the title screen; the ADF is the playable version."
/>

## How to play

On a real Amiga, or from the ADF, the controls are:

| Control | Action |
|---|---|
| Joystick left / right | Move the paddle |
| Joystick fire | Start a game, or start again from the title |

The web player's **Restart** button reloads the saved machine. The live view
restores the title screen, so the demo appears without waiting for a boot. The
score counts ten points per brick. A game starts with three lives; the paddle
angle is set by where the ball lands on it. After the third miss the title
screen returns; press fire to play again on the ADF.

## What you are looking at

| Detail | Value |
|---|---|
| Machine | Amiga 500, 68000, OCS, 512 KB chip + 512 KB slow RAM |
| Display | 320×256, 5 bitplanes, 32 colours, PAL |
| Runtime | Clorraine `hosted-amiga-13` release build, libnix `nix13` |
| Emulator | [Copperline](https://copperline.dev/) 1.1.0 with its bundled AROS ROM |
| Source | [`examples/amiga/prism.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/prism.clo) |

## How it is built

| Part | Technique |
|---|---|
| Title | A full-screen bitmap with a five-colour wordmark and a prism triangle |
| Playfield | A background bitmap with a frame, starfield, and forty bricks |
| HUD | A 320×40 strip with the logo, score, and life icons |
| Score | Seven-segment digits drawn with bitplane fills |
| Paddle and ball | Direct bitplane fills; no per-frame sprite allocation |
| Bricks | Coloured fills with a white top edge and a dark shadow |
| Animation | Dirty rectangles: only the pixels under the paddle, ball, and trail are restored |
| Game loop | Joystick polling, fixed-point physics, and one view flip per frame |

The game avoids per-pixel work on the 68000. The playfield is built once,
destroyed bricks are replaced with a background fill, and each frame restores
only the small rectangles the moving objects covered. Every frame buffer keeps
its own record of what it still shows, so a flip never lands on a stale frame.

On an A500 the game runs at close to the 50 Hz PAL refresh: 400 frames took
423 ticks (8.5 seconds). Rendering the whole playfield every frame cost about
twice that.

## Run it on your own Amiga

- [Download `prism.adf`](/demos/prism.adf) — A500, Kickstart 1.3 or AROS,
  880 KB.

From a checkout with the SDK:

```sh
make run FILE=examples/amiga/prism.clo \
  PROFILE=hosted-amiga-13 EMULATOR=1
```

The game needs a display, so use FS-UAE or Copperline rather than `vamos`.

## Read the source

The program is one file. Start with `buildTitle`, `buildField`, `fillPaddle`,
and `main`.

- [`examples/amiga/prism.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/prism.clo)
- [Native video API](/reference/stdlib) — `std.amiga.video`
- [Amiga input API](/reference/stdlib) — `std.amiga.input`
- [Amiga profiles](/amiga/profiles)
