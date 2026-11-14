---
title: Boing
description: "Boing: a 32-colour neon-horizon Amiga demo with a dithered sunset, a perspective grid, and a chrome ball bouncing over two chip-memory screens."
---

# Boing: Neon Horizon

A 320×256, 32-colour sunset built from chip-memory bitplanes. The sky ramps
through eight pens with ordered dithering, the lower half is a perspective
grid, and a 48×48 chrome ball bounces across the scene through a one-plane
mask. The display flips
between two bitmaps, so the ball never tears.

<AmigaPlayer
  poster="/screenshots/boing.webp"
  state="/demos/boing.clstate"
  adf="/demos/boing.adf"
  badge="A500 · 32 colours"
  caption="Boing: Neon Horizon · compiled with clc · live in Copperline"
  hint="Runs in your browser with Copperline and AROS. The ball keeps bouncing until you stop it."
/>

## What you are looking at

| Detail | Value |
|---|---|
| Machine | Amiga 500, 68000, OCS, 512 KB chip + 512 KB slow RAM |
| Display | 320×256, 5 bitplanes, 32 colours, PAL |
| Runtime | Clorraine `hosted-amiga-13` release build, libnix `nix13` |
| Emulator | [Copperline](https://copperline.dev/) 1.1.0 with its bundled AROS ROM |
| Source | [`examples/amiga/boing-showcase.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/boing-showcase.clo) |

The live view restores a saved machine state, so the demo starts without
waiting for AROS to boot. The first click still downloads the emulator bundle
and the machine state; after that the browser caches them. No Kickstart ROM or
Workbench media is downloaded or distributed.

## The scene

| Element | Technique |
|---|---|
| Sky | An eight-pen indigo-to-orange gradient, dithered with a 4×4 Bayer matrix |
| Sun | A four-pen disc with dithered seams and horizontal slits |
| Mountains | Two ridge silhouettes with a warm rim |
| Grid | Converging verticals and depth-spaced horizontal rules on a dark ground |
| Ball | A pre-rendered 48×48 chrome sphere with a cyan rim |
| Motion | 8.8 fixed-point position and velocity, with edge bounces |
| Display | Two chip-memory bitmaps, one visible while the other is drawn |

## Why it is fast on a 68000

The static scene is drawn once into a chunky byte buffer and converted to
bitplanes in one call. Each frame only restores the small rectangle behind the
ball, blits the ball through its mask, and flips the view. The ball and mask are
pre-rendered, so the 68000 never walks the sphere pixel by pixel during play.

The demo runs at the full PAL rate: 400 frames in 400 ticks on an A500, the same
rate as the acceptance Boing.

The demo also drove two runtime improvements: `bitmapFromIndices` writes
bitplane bytes directly instead of repeating the per-bit bounds check, and
custom displays can be opened at an explicit screen offset so the emulator's
presenter sees the whole picture.

## Run it on your own Amiga

- [Download `boing.adf`](/demos/boing.adf) — A500, Kickstart 1.3 or AROS,
  880 KB.

From a checkout with the SDK:

```sh
make run FILE=examples/amiga/boing-showcase.clo \
  PROFILE=hosted-amiga-13 EMULATOR=1
```

The program needs a display, so use FS-UAE or Copperline rather than `vamos`.

## Read the source

The program is one file. Start with `drawBackground`, `buildBall`, and `main`.

- [`examples/amiga/boing-showcase.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/boing-showcase.clo)
- [Native video API](/reference/stdlib) — `std.amiga.video`
- [Amiga profiles](/amiga/profiles)
- [SDK coverage](/amiga/sdk-coverage)
