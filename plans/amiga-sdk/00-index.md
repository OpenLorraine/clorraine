# Amiga SDK Programme

> **Status**: Done — Bundles 0–9 complete; Bundle 9 closeout 20/21 (RTG out-of-scope, issue #11)
> **Created**: 2026-11-04
> **Coverage**: [docs/amiga-sdk/coverage.md](../../docs/amiga-sdk/coverage.md)
> **Registry**: [docs/amiga-sdk/registry.tsv](../../docs/amiga-sdk/registry.tsv)

## Purpose

This is the programme map for the Amiga SDK surface. It is not an execution
plan; each bundle is a separate CodeOps plan with its own requirements, design,
tests, and acceptance vehicle.

## Baseline

| Tier | Hardware | OS | Profile |
|---|---|---|---|
| Core | A500: 68000, OCS, 512 KB chip | Kickstart 1.3 | `hosted-amiga-13` |
| Advanced | ECS/68000/1 MB and up (A1200/AGA included) | Kickstart 2.0–3.1 | `hosted-amiga` |
| Optional | 3.0+/AGA add-ons | Optional extras | `hosted-amiga` |

The core tier is the default for Bundles 0–6 and the core Exec work. Advanced
rows declare `min_os`/`min_hw` in the registry.

## Bundle order

| # | Bundle | Tier | Acceptance |
|---|---|---|---|
| 0 | Baseline and registry | core | A500 hello on KS1.3 |
| 1 | Native Video | core | Boing on A500 |
| 2 | Sprites and collision | core | Sprite demo |
| 3 | Audio and input | core | Paddle game |
| 4 | Text, fonts, layers | core | Text/layers demo |
| 5 | DOS and assets | core | ILBM image viewer |
| 6 | Intuition UI | core + advanced | GUI demo |
| 7 | Exec and devices | core + advanced | Worker task + async IO |
| 8 | Workbench and services | advanced | Workbench integration |
| 9 | Optional add-ons and advanced closeout | optional | A500 diskfont, A600 2.05/3.1, A1200 3.0/3.1 headless gates; 20/21 done, RTG in #11 |

## Rules

- Capability-complete, not header-complete: the registry is the contract.
- No language, grammar, diagnostic, or ABI change; appendix updates only.
- Opaque handles and builders; no raw OS structs in public APIs.
- Every DMA-visible buffer is chip memory; every installed hardware resource is
  owned until it is removed.
- Each bundle is a separate release and must keep earlier bundles green.
- The registry is the source of truth; GitHub issues are a generated view.

## Current

Bundle 0 is `plans/amiga-sdk-baseline/`: it adds the A500/1.3 profile and the
registry. Bundle 1 is `plans/amiga-sdk-native-video/`: chip memory, bitmaps,
software drawing, the blitter, a copper builder, a double-buffered custom
display, fixed-point math, the frame checksum, and the Boing acceptance on the
real A500/KS1.3 machine. Bundle 2 is `plans/amiga-sdk-sprites/`: the eight OCS
sprite channels, attached 16-color pairs, hardware collision, animation
objects, and the sprite demo on the real A500/KS1.3 machine. Bundle 3 is
`plans/amiga-sdk-audio-input/`: audio.device samples, gameport joystick,
timer.device delays, and the paddle game on the real A500/KS1.3 machine. The
two advanced `could` rows are deferred. Bundle 4 is
`plans/amiga-sdk-text-layers/`: ROM fonts, text measurement, layers with
clipping, and the scrolling text demo on the real A500/KS1.3 machine. Diskfont
and 2.0+ regions are deferred. Bundle 5 is `plans/amiga-sdk-dos-assets/`:
the DOS file/lock/path/volume surface, the pure IFF/ILBM parser, the ILBM
loader, and the image viewer on the real A500/KS1.3 machine. `ReadArgs` and
`datatypes.library` are deferred. Bundle 6 (Intuition UI) is
`plans/amiga-sdk-intuition/`; all 17 tasks are verified. The A500 GUI acceptance
passes with a private Xvfb keyboard run, the native
`docs/amiga-sdk/a500-gui.png`, and the report line `gui: ok win 1 gad 2 menu 2
str 3 req 2`. The frozen-spec appendices document the classic 1.3 surface on
both hosted profiles, the full `make verify` gate passes headlessly, and issue
#7 is closed. Bundle 7 (Exec and devices) is
`plans/amiga-sdk-exec-devices/`: worker tasks with the V34-safe
`Forbid`/`RemTask` join, the existing private ports and signals, library
open/close/version, the generic `IOStdReq` device surface with its reply port,
the console-window helper, the VBlank and ports interrupt servers, and the
input/keyboard/console device coverage. The A500 Exec gate reports
`exec: ok task 464 lib 34 dev 512 io 13 key -2 in 1` with the 620x22
`docs/amiga-sdk/a500-exec.png`; issue #8 is closed. Bundle 8 (Workbench and
services) is `plans/amiga-sdk-workbench/`: the Workbench AppIcon, AppWindow,
AppMenuItem, and AppMessage surface, the icon library, expansion-board
enumeration, the card resource, and a keymap call. The A1200/KS3.1 gate reports
`wb: ok icon 1 win 1 menu 1 types 8 7 9 ids 7 8 9 users 42 43 44 classes 0 0 0 args 0 0 0`
and `svc: ok icon 3 exp 2 man 2011 prod 82 card 1 status 0 key 1`, with the
native `docs/amiga-sdk/a1200-workbench.png` and
`docs/amiga-sdk/a1200-workbench-icon.png`; the A600/KS2.05 gate asserts the
same lines; issue #9 is closed. Bundle 9 (Optional add-ons and advanced
closeout) is `plans/amiga-sdk-optional/`: signal semaphores, utility
tags/date/hooks, OS pools, gadtools, ASL, BOOPSI, ReadArgs, datatypes, regions,
EClock, lowlevel, commodities, locale, ARexx, clipboard, serial/parallel/
printer, nonvolatile, bsdsocket, and AmigaGuide. The licensed Workbench
1.3.4/2.04/2.1/3.0/3.1 media supplies the disk libraries. Five headless gates
report `diskfont: ok h 12`, the A600 2.05/3.1 lines, `pool: ok 1` and
`boopsi: ok val 42`, and `dt: ok w 320 h 200 depth 4 colors 16` / `net: ok send
5 recv 5` / `guide: ok msg 1 result 0`. The registry is 80 capabilities, all
done except the RTG `wont` row tracked by issue #11; issue #10 is the generated
Bundle 9 view.
