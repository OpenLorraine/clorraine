---
title: First Light
description: "A live Amiga demo written in Clorraine: a dawn seascape with a lighthouse, rendered in 16 colours on an A500 and running in your browser."
---

# First Light

A dawn seascape written in Clorraine and compiled to one C99 translation unit.
The demo opens a 320×256, 16-colour display on an Amiga 500, draws the scene
into chip-memory bitplanes, and animates the sun's reflection as an eight-phase
filmstrip. It is a release build of a real program, not a mockup.

<FirstLightPlayer />

## What you are looking at

| Detail | Value |
|---|---|
| Machine | Amiga 500, 68000, OCS, 512 KB chip + 512 KB slow RAM |
| Display | 320×256, 4 bitplanes, 16 colours, PAL |
| Runtime | Clorraine `hosted-amiga-13` release build, libnix `nix13` |
| Emulator | [Copperline](https://copperline.dev/) 1.1.0 with its bundled AROS ROM |
| Source | [`examples/amiga/first-light.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/first-light.clo) |

The live view restores a saved machine state, so the demo appears without
waiting for AROS to boot. The first click still downloads the emulator bundle
(about 8 MB), the AROS ROMs (1 MB), and the machine state (about 1 MB); after
that the browser caches them. No Kickstart ROM or Workbench media is downloaded
or distributed.

## The scene

The 16-colour palette is a single dawn ramp from near-black indigo through
plum and rust to a pale sun. Every gradient is an ordered 4×4 Bayer dither, the
classic Amiga way to stretch sixteen colours across a sky.

| Element | Technique |
|---|---|
| Sky | Six solid bands with short dithered seams, so the sky reads as bands rather than a checkerboard |
| Sun and glow | A radial lookup table indexed by squared distance; the fractional level is applied with the same 4×4 dither |
| Clouds | Tapered horizontal banks with a lit underside near the sun |
| Headland | A per-column silhouette with a warm rim, rock facets, and grass tufts |
| Lighthouse | A tapered silhouette with a lit lantern room |
| Sea | A shallow gradient, broken swell lines, and a broken reflection column |
| Animation | Eight pre-rendered reflection phases in one tall bitmap; each frame is one blit and one flip |

## Why it is fast on a 68000

The obvious implementation — call the graphics library once per pixel — is far
too slow. First Light renders the static scene into a chunky byte buffer and
converts it to bitplanes once with `bitmapFromIndices`. The moving water is
rendered into eight 120×90 phases ahead of time and packed into a single
`120×720` bitmap. At runtime the 68000 only issues one blit for the reflection
and one view flip per frame.

The demo also exposed two runtime improvements that benefit every program:
`bitmapFromIndices` now writes bitplane bytes directly instead of repeating the
bounds check for every set bit, and the custom-view builder accepts AROS's
`MCOP_OK` merge status in addition to the classic `LOFCprList` check.

## Controls

- **Run the demo** starts the machine and restores the saved state.
- **Pause / Resume** stops and restarts the frame loop.
- **Restart** reloads the saved state, returning to the opening frame.
- **Fullscreen** puts the player on the whole screen.
- The demo itself is ambient; it needs no keyboard or joystick input.

If the live player cannot start, the error panel links to the same ADF in
Copperline's hosted player and to the ADF download.

## Run it on your own Amiga

The bootable ADF is available from the player caption and here:

- [Download `first-light.adf`](/demos/first-light.adf) — A500, Kickstart 1.3 or
  AROS, 880 KB.

From a checkout with the SDK, build and run the source directly:

```sh
make run FILE=examples/amiga/first-light.clo \
  PROFILE=hosted-amiga-13 EMULATOR=1
```

The program needs an Amiga display, so use FS-UAE or Copperline rather than
`vamos`. The build is a single `.clo` file with no external assets.

## Read the source

The program is deliberately one file. Start with `buildMaster` and
`drawReflectionRegion`, then follow `main` for the display lifecycle.

- [`examples/amiga/first-light.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/first-light.clo)
- [Native video API](/reference/stdlib) — `std.amiga.video`
- [Amiga profiles](/amiga/profiles)
- [SDK coverage](/amiga/sdk-coverage)
