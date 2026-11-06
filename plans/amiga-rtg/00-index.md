# Amiga RTG: Picasso96 on UAEGFX and emulated Picasso IV

> **Feature**: Add a capability-complete, headless-tested Picasso96 RTG surface for the Amiga SDK, gated on an A1200/KS3.1 with FS-UAE's UAEGFX card and an A4000/KS3.1 with the emulated Picasso IV Zorro III card.
> **Status**: Done — 26/26; Bundle 10 2/2; both gates green
> **Created**: 2026-11-06 13:30
> **Last Updated**: 2026-11-06 22:35
> **Implements**: R1–R18
> **CodeOps Artifact Schema**: 1

## Overview

RTG is the last optional Amiga graphics capability. The registry previously kept
`SDK-OPT-004` (RTG/Picasso96/CyberGraphX) as `wont` / out-of-scope because the
programme had no chosen stack, no installed driver, and no emulated card in the
gate. The user has now approved a focused implementation:

- **Stack:** Picasso96 2.0 from Aminet (`driver/video/Picasso96.lha`), with the
  P96 developer definitions from `dev/misc/P96Develop.lha`.
- **Cards:** FS-UAE's `uaegfx` Zorro III card (32-bit true alpha) and the
  emulated Picasso IV Zorro III card (24-bit truecolor).
- **Baselines:** A1200/KS3.1, 68020, 2 MB chip + fast memory; A4000/KS3.1,
  68040, 2 MB chip + 8 MB fast.
- **Surface:** open a 24- or 32-bit RTG screen, discover modes, allocate an
  offscreen bitmap, draw and read pixels, blit a bitmap to the screen, read RGB
  pixels for a capture, and close everything exactly once.
- **Gates:** headless FS-UAE, a report line, and a PPM capture whose header
  and pixels are checked on the host for both cards.

The implementation follows the existing Amiga SDK pattern: one stdlib module
(`std.amiga.picasso96`), a separate C shim suffix, opaque handles with exactly
once `freedBy` cleanup, specification tests, implementation pins, and a
headless gate. No language, grammar, diagnostic, golden, or ABI change is
included.

## Minimum-Sufficient Baseline

**Smallest viable design:** use the P96 API directly for mode discovery,
screen opening, offscreen bitmaps, pixels, and blits. Publish only the
operations the gate can prove; do not wrap PIP/overlay, CLUT palette editing,
CyberGraphX, or real graphics-card drivers.

**Excluded machinery:** a new graphics abstraction layer, a software renderer,
a font/text surface, a screen-mode preference editor, and a second RTG stack.
Those are separate capabilities, not prerequisites for the approved row.

**Approved complexity:** one new stdlib module, one C shim, one gate script,
one Makefile target, one registry bundle, and the corresponding documentation.
The P96 API header used by the shim is vendored as a small, attributed
definition file so the compiler build does not depend on an SDK include path.

## Document Index

| # | Document | Description |
|---|---|---|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Decisions and evidence for the approved scope |
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Functional, compatibility, and acceptance requirements |
| 02 | [Current State](02-current-state.md) | Registry, P96 media, FS-UAE, and existing SDK patterns |
| 03-01 | [API and Ownership](03-01-api.md) | Public module, handles, functions, and error rules |
| 03-02 | [Runtime Shim](03-02-runtime.md) | P96 calls, formats, bounds, and teardown |
| 03-03 | [Gate and Media](03-03-gate-media.md) | A1200/UAEGFX staging, demo, report, and PPM check |
| 04 | [Picasso IV Gate](04-picasso-iv-gate.md) | A4000/emulated Picasso IV staging, 24-bit path, and shared gate script |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec cases, implementation pins, and verification matrix |
| 00a | [Requirements Preflight](00-preflight-report-requirements.md) | Preflight report for the requirements set |
| 99 | [Execution Plan](99-execution-plan.md) | Phases and task checklist |
| 00b | [Plan Preflight](00-preflight-report.md) | Preflight report for the execution plan |

## Quick Reference

```clorraine
import { RtgScreen, RtgBitmap, openRtgScreen, rtgScreenWidth,
         rtgScreenHeight, rtgScreenDepth, rtgScreenFormat,
         allocRtgBitmap, rtgScreenWritePixel, rtgScreenFillRect,
         rtgScreenBlitBitmap, rtgScreenReadPixels, rtgBitmapFillRect,
         rtgBitmapWritePixel, closeRtgScreen, freeRtgBitmap } from std.amiga.picasso96;

func draw(): i32 {
    let created: RtgScreen? = openRtgScreen(640, 480, 32, "Clorraine RTG");
    if (created == null) { return 1; }
    let screen: RtgScreen = created;
    rtgScreenWritePixel(screen, 0, 0, 0x00FF0000);
    rtgScreenFillRect(screen, 10, 10, 20, 20, 0x0000FF00);

    let bitmapCreated: RtgBitmap? = allocRtgBitmap(32, 32, 32);
    if (bitmapCreated == null) { return 2; }
    let bitmap: RtgBitmap = bitmapCreated;
    rtgBitmapFillRect(bitmap, 0, 0, 31, 31, 0x000000FF);
    rtgBitmapWritePixel(bitmap, 0, 0, 0x00FFFF00);
    rtgScreenBlitBitmap(screen, bitmap, 210, 210, 0, 0, 32, 32);
    closeRtgScreen(screen);
    freeRtgBitmap(bitmap);
    return 0;
}
```

The screen and bitmap handles are dropped exactly once by the generated
cleanup. The module is `hosted-amiga` only; the host, A500/KS1.3, and
freestanding profiles reject it with `E11200`.

## Registry Change

The old placeholder row `SDK-OPT-004` (`wont`, out-of-scope) is replaced by a
new optional bundle `10-rtg` and row `SDK-RTG-001`. Bundle 9 remains the
completed 20/20 in-scope bundle; RTG is tracked as its own optional bundle and
by issue #11. This keeps the historical closeout truthful while giving RTG a
real plan and acceptance row.
