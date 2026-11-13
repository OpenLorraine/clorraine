---
title: Amiga
description: "Clorraine on 68K Amiga: profiles, the tracked SDK surface, emulator workflow, and real acceptance captures."
---

# Amiga

Clorraine is designed for 68K Amiga systems first. It emits plain C99 and builds
with the AmigaPorts m68k GCC toolchain, so the output fits the compilers,
libraries, and emulators the platform already has.

## Two baselines

| Baseline | What it means |
|---|---|
| **A500 / Kickstart 1.3** | 68000, OCS, 512 KB chip RAM, `-mcrt=nix13`. A first-class target, not an afterthought |
| **AmigaOS 2.0+** | Bigger machines and the 2.0+ modules: Workbench, icon, expansion, and the optional services |

The `hosted-amiga-13` profile provides the 1.3-safe subset. The `hosted-amiga`
profile provides the 2.0+ surface. A construct the selected profile cannot lower
is rejected with `E11200`; it is not silently emulated.

## What is covered

The SDK registry tracks **81 capabilities across 11 bundles**, all accepted:

| Area | Examples |
|---|---|
| Native video | Chip memory, bitmaps, blitter, copper, double buffering, palette |
| Sprites | Eight hardware channels, attachment, collision |
| Audio and input | Audio channels, samples, mouse, keyboard, joystick |
| Text and layers | Topaz fonts, text rendering, layer clips |
| DOS and assets | Files, directories, paths, ILBM loading |
| Intuition | Screens, windows, gadgets, requesters, menus |
| Exec and devices | Tasks, ports, messages, interrupts, timers, libraries |
| Workbench | AppIcons, AppMessages, Workbench startup |
| Optional services | ASL, commodities, locale, ARexx, clipboard, datatypes, network, Picasso96, and more |

See [SDK coverage](/amiga/sdk-coverage) for the full table and the tracked
boundaries.

## A real A500 run

<DocFigure
  src="/screenshots/a500-boing.webp"
  alt="The Clorraine Boing demo: a chequered Amiga screen with a shaded sphere."
  caption="The Boing acceptance program on A500 / Kickstart 1.3 / OCS."
/>

The Boing demo owns a 320×256×4 custom display built from chip-memory bitmaps.
It draws a checkerboard, blits a shaded ball through a mask, and changes
background colours with a copper list. The final frame checksum is compared on
every A500 gate.

## Emulator workflow

| Tool | Use it for | Needs a ROM? |
|---|---|---|
| `vamos` | Console programs, fast iteration, CI | No |
| FS-UAE | Graphics, audio, Intuition, timers | No with the built-in AROS ROM |
| FS-UAE with licensed media | Original Kickstart/Workbench behavior | Yes, user-supplied |

Inside the repository:

```sh
make run FILE=examples/hello/hello.clo PROFILE=hosted-amiga
make run FILE=examples/amiga/boing.clo PROFILE=hosted-amiga-13 EMULATOR=1
```

The first command runs under `vamos`. The second builds and boots FS-UAE. See
the [emulator workflow](/amiga/emulator) for the exact prerequisites.

## Profiles

| Profile | Machine | Runtime |
|---|---|---|
| `hosted-amiga` | AmigaOS 2.0+ | libnix `-mcrt=nix20` |
| `hosted-amiga-13` | A500, 68000, OCS, 512 KB chip, KS 1.3 | libnix `-mcrt=nix13` |
| `freestanding` | Provisional LorraineOS ABI | no libc |

The `host` profile exists for fast tests. It is not an Amiga artifact.

## Media policy

Clorraine does not distribute Kickstart ROMs or Workbench media. No ROM or
Workbench image is stored in the repository. FS-UAE's built-in AROS replacement
ROM is the default; licensed media is supplied by the person running the gate.

Read the [licensing and provenance page](/project/licensing#media-and-the-ndk).

## Screenshots

Every image in the [gallery](/amiga/screenshots) comes from an FS-UAE acceptance
run. No mockups, no Workbench marketing shots.
