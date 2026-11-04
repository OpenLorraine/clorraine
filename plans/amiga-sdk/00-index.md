# Amiga SDK Programme

> **Status**: Executing — Bundles 0–3 done; Bundle 4 next
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
| 9 | Optional add-ons | optional | BOOPSI/bsdsocket/etc. |

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
two advanced `could` rows are deferred. Bundle 4 (text, fonts, layers) is
next.
