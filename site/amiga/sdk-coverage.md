---
title: SDK coverage
description: "The tracked Amiga SDK surface: 81 capabilities across 11 bundles, with acceptance gates."
---

# SDK coverage

The Clorraine SDK registry tracks a specific surface, not "all of AmigaOS". Each
capability has a stable ID, a tier, a minimum OS, an acceptance gate, and a
status.

**Current state: 81 of 81 tracked capabilities are done, with no must-have item
open.**

## Bundles

| Bundle | Tier | Min OS | Done | Total |
|---|---|---|---|---|
| 0 — baseline | core | 1.3 | 9 | 9 |
| 1 — native video | core | 1.3 | 11 | 11 |
| 2 — sprites | core | 1.3 | 6 | 6 |
| 3 — audio and input | core | 1.3 | 6 | 6 |
| 4 — text and layers | core | 1.3 | 4 | 4 |
| 5 — DOS and assets | core | 1.3 | 5 | 5 |
| 6 — Intuition | core | 1.3 | 6 | 6 |
| 7 — Exec and devices | core | 1.3 | 8 | 8 |
| 8 — Workbench | advanced | 2.0 | 4 | 4 |
| 9 — optional services | optional | 1.3 | 20 | 20 |
| 10 — RTG | optional | 3.0 | 2 | 2 |
| **Total** | | | **81** | **81** |

The authoritative, generated table is
[docs/amiga-sdk/coverage.md](https://github.com/OpenLorraine/clorraine/blob/main/docs/amiga-sdk/coverage.md).
It is generated from `registry.tsv` and `bundles.tsv` by `make sdk-registry`; do
not edit it by hand.

## What the bundles cover

### Native video

Chip-memory allocation, bitmap creation and drawing, blitter copy and masked
copy, copper lists, VBlank and double buffering, palette control, and
trigonometry helpers. The Boing acceptance program exercises the whole path.

### Sprites

Eight hardware sprite channels, sprite attachment, and hardware collision
detection. The A500 sprite gate checks the collision result on real emulated
hardware.

### Audio and input

Audio channel allocation, sample playback, volume and period control, mouse and
keyboard events, joystick reading, and the paddle game gate.

### Text and layers

Topaz font handling, text rendering, layer creation and clipping, and the
scrolling-text gate.

### DOS and assets

File and directory operations, path handling, delays, and ILBM loading. The
asset viewer gate decodes and displays a real ILBM image.

### Intuition

Screens, windows, gadgets, requesters, menus, and the event loop. The GUI gate
drives the demo headlessly with Amiga-key shortcuts.

### Exec and devices

Tasks, message ports, messages, interrupts, timers, library opening, and the
worker-task contract. The Exec gate checks the report line and a native screen
capture.

### Workbench

AppIcons, AppMessages, Workbench startup, and the advanced Workbench gates on
A600 and A1200.

### Optional services

ASL requesters, commodities, locale, ARexx, clipboard, serial/parallel/printer,
nonvolatile storage, BOOPSI, datatypes, AmigaGuide, network, Picasso96 RTG, and
pool memory. Each has a runtime gate where the environment can prove it, and a
documented boundary where it cannot.

## What is outside the tracked surface

The count is not a claim of complete AmigaOS coverage. Known boundaries include:

- CyberGraphX and CGX4, PIP/video overlay, CLUT palette editing, and RTG
  text/font rendering.
- Workbench and optional services that need media the project cannot
  redistribute.
- Services whose runtime behavior could not be proved in the emulator; those
  are documented rather than claimed.

See [Known limitations](/guide/known-limitations) for the user-facing list and
the [full limitations document](https://github.com/OpenLorraine/clorraine/blob/main/docs/known-limitations.md)
for the details.

## How the registry is checked

`tools/amiga-sdk/registry_test.sh` validates the TSV schema, IDs, tiers,
minimum OS values, and acceptance names. `make sdk-registry` regenerates the
coverage view and fails if the committed file is stale. The FS-UAE gates then
prove the runtime behavior for the tracked items.
