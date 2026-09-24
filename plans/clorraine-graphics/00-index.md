# Clorraine Amiga Graphics Implementation Plan

> **Feature**: A minimal Amiga graphics surface for the hosted-amiga standard library: an owned custom screen and filled/line drawing.
> **Status**: Planning Complete
> **Created**: 2026-09-24 14:37
> **Implements**: — (standalone feature; tracked as roadmap `PLAN-08`)
> **CodeOps Artifact Schema**: 1

## Overview

The corpus-subset MVP gives Clorraine text and file output plus Amiga scheduler
and VBlank access, but nothing visual. This plan adds the smallest graphics
surface that makes a real Amiga program draw: a custom screen owned by a
`Screen` handle, and four drawing calls (`setColor`, `fill`, `draw`, `clear`).
The language already has everything needed — `ffi`, `handle … freedBy`,
`borrow`/`take`, and `unsafe` inside the library — so the feature is a
standard-library and runtime-shim addition, not a language change.

A spike (`spikes/05-graphics/`) proved the path end to end: a Clorraine program
opened a 320x256x4 custom screen and drew colored bars under FS-UAE with a real
Kickstart 3.1. This plan productizes that spike: two modules, the shim calls
behind them, a corpus program with its golden output, the CI legs that run it,
and the spec documentation.

## Minimum-Sufficient Baseline

**Original goal:** Let a Clorraine program draw on an Amiga screen, shown in the demo video and verified in CI.
**Smallest viable design:** Two modules — `std.amiga.intuition` owns the `Screen` handle, `std.amiga.graphics` owns drawing — over thin C shim functions, reusing the spike's validated `ffi` contract and the existing `std.amiga.interrupt`/`dos` patterns.
**Excluded machinery:** Windows/event loop, text and fonts, bitmaps/blit, sprites/copper, `SetRGB32`, a `ScreenSpec` struct, and a screenshot-golden test harness. (AR-4, AR-8, AR-26, AR-29–AR-34)
**Approved complexity:** None required — no material support surface beyond ordinary stdlib, shim, spec-golden, and script work. (AR-26)

## Document Index

| #   | Document                                          | Description                                 |
| --- | ------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)    | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                              | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                | Feature requirements and scope              |
| 02  | [Current State](02-current-state.md)              | Analysis of the current implementation      |
| 03-01 | [Screen Lifecycle](03-01-screen-lifecycle.md)   | `std.amiga.intuition` technical spec        |
| 03-02 | [Drawing](03-02-drawing.md)                     | `std.amiga.graphics` technical spec         |
| 03-03 | [Runtime Shim](03-03-runtime-shim.md)           | Hosted-amiga C shim and ABI                 |
| 03-04 | [Spec, Corpus, Video](03-04-spec-corpus-video.md) | Spec edits, corpus program, CI legs, video |
| 07  | [Testing Strategy](07-testing-strategy.md)        | Spec test cases and verification            |
| 99  | [Execution Plan](99-execution-plan.md)            | Phases, sessions, and task checklist        |

## Quick Reference

### Usage Examples

```clorraine
import { Screen, openScreen } from std.amiga.intuition;
import { setColor, fill, draw, clear } from std.amiga.graphics;
import { delayTicks } from std.amiga.dos;

func main(): i32 {
    let screen: Screen? = openScreen(320, 256, 4, "Clorraine graphics");
    if (screen == null) {
        return 1;
    }
    setColor(screen, 0, 0, 0, 0);
    setColor(screen, 1, 255, 32, 32);
    clear(screen, 0);
    fill(screen, 20, 20, 120, 120, 1);
    draw(screen, 0, 0, 319, 255, 1);
    delayTicks(250);
    return 0;   // the screen drops here, calling cl_close_screen
}
```

### Key Decisions

| Decision            | Outcome                                                       |
| ------------------- | ------------------------------------------------------------- |
| Display model       | Custom screen only (AR-2, AR-4)                               |
| Module split        | `std.amiga.intuition` + `std.amiga.graphics` (AR-5)           |
| Ownership           | Owned `Screen`; `RastPort` hidden; drawing takes `borrow` (AR-6, AR-18) |
| Value types         | `i32` coords, `u8` pen, `u8` RGB (AR-12)                      |
| Verification        | `make check` + `make verify`; FS-UAE sentinel leg (AR-19, AR-21) |
| Frozen-spec changes | Two appendices + one corpus program (AR-3, AR-27)             |

## Related Files

- `stdlib/std.amiga.intuition.clo` (new), `stdlib/std.amiga.graphics.clo` (new)
- `compiler/src/stdlib.rs` (register both modules)
- `compiler/runtime/shim_hosted_amiga.c` (screen and drawing functions)
- `spec/appendix-amiga.md`, `spec/appendix-stdlib.md` (documented surface)
- `spec/tests/corpus/graphics/` (program, golden output, README)
- `compiler/tests/graphics_spec_test.rs`, `compiler/tests/graphics_impl_test.rs` (new)
- `docker/scripts/corpus-fs-uae.sh` (graphics leg)
- `demo/story.sh` (emulator-only graphics chapter; PF-009)
- `AGENTS.md` (test naming map and stdlib/shim surface; PF-006)
- `plans/00-roadmap.md` (`PLAN-08`)
