# Amiga Shim Crash-Safety Implementation Plan

> **Feature**: Fix the hosted-amiga shim paths that crash or hang a real Amiga: Intuition opened on demand, screen text clipped, the clipboard read finished, borrowed clip regions and guide messages tracked
> **Status**: Done (2026-11-27; 39/39 tasks; `make verify` passed; runtime decisions AR-17..AR-19)
> **Created**: 2026-11-27
> **Implements**: none (standalone plan; no RD) — closes DEF-585, DEF-587, DEF-588, DEF-593 and theme issue #44 (findings #57, #59, #60, #65)
> **CodeOps Artifact Schema**: 1

## Overview

Four safe stdlib entry points on the hosted-amiga profiles can crash or hang a real machine.
`showAlert` and `savePublicScreen` call Intuition through a NULL base when the program has not
opened a screen yet. `drawText` writes glyph rows past the screen bitmap when the text runs off
the screen, because a screen `RastPort` has no layer to clip it. `clipboardRead` never finishes
the read, so the next write on that unit blocks for ever. And a program the compiler accepts can
free a clip region a layer still uses, or reply an AmigaGuide message after its context closed.

The plan fixes each in the shim with no change to the stdlib signatures or the frozen spec
(AR-12): the two Intuition entries open the base on demand (AR-2); `drawText` keeps its current
path for text that fits and clips at the pixel edge otherwise (AR-3..AR-5); the clipboard read
ends with the device's documented read-past-the-end (AR-10, AR-11); freeing a region uninstalls it
from its layers, freeing a layer ends an open update and uninstalls its region, and closing a
guide replies the messages the program still holds (AR-6..AR-9). It builds on the exit registry
from `amiga-fault-exit-cleanup`.

Evidence runs on four legs: `vamos` with Intuition switched off, the A1200/KS3.1 add-ons gate,
one clipboard probe in the A600/KS2.05 add-ons gate, and the A500/KS1.3 fault-cleanup gate
(AR-13).

## Minimum-Sufficient Baseline

**Original goal:** Fix the shim paths that crash or hang a real machine: open Intuition on demand,
clip screen text, finish the clipboard read protocol, track borrowed clip regions and guide
messages (issue #44).
**Smallest viable design:** Local fixes inside the existing shim functions, reusing `cl_lib_open`,
the exit registry and its walk-by-release-function precedent, and the existing FS-UAE gates
for target evidence (two extended, plus one probe line in `sdk-a600-addons`).
**Excluded machinery:** new gate scripts; a test-only C fixture that inspects layer state; a
per-region layer list; a full-depth copy-back renderer (AR-4, AR-7, AR-13).
**Approved complexity:** None.

## Planning Coverage

> **Coverage**: 4/4 clusters · 12/12 categories · 30 candidates (14 material after dedupe, 0 optional, 0 complexity) · 6 residual limits · Effort: high/high/high/xhigh→unavailable (fail-open) · Independence: uniform-effort
> **Freshness**: Coverage as of 2026-11-27; artifacts: packet D2–D7, shims at `9cef1491`, spec appendices, NDK 3.2 autodocs, NDK 1.3 headers

| Cluster | Area | Examined | Not examined / limit | Candidates | Notes / limit | Confidence |
|---------|------|----------|----------------------|------------|---------------|------------|
| C1 Product and scope | stdlib docs, spec rows, the four findings | the six stdlib modules, spec §14.9/14.18/14.22/§21, shims | library autodocs | 6 → AR-5, AR-11, AR-12, AR-13, AR-16 | CV-005 colour fonts → AR-5 | medium |
| C2 Behavior and edges | registry/exit order, text, regions, guide, clipboard | shims, NDK 3.2 `layers.doc`/`clipboard.doc`/`graphics.doc` | no target runs; V34 behaviour from BUGS notes only | 11 → AR-5, AR-6, AR-8, AR-10, AR-11, AR-13; 2 out of scope | CV-009 write path, CV-010 public-screen lock → out of scope | medium |
| C3 Technical and data | V33/V34 availability, ink box, temp raster, clipboard, vamos | NDK 1.3 protos and `text.h`, NDK 3.2 autodocs, amitools `vamos` config | V34 ROM, layers/amigaguide internals | 6 → AR-4, AR-5, AR-10, AR-13 | BltTemplate, InstallClipRegion, BltMaskBitMapRastPort in the 1.3 protos | medium |
| C4 Risk and stakeholders | memory safety, size, chip, gates, goldens | gate scripts, examples, bench harness, verifier probes and verdicts | no target runs | 7 → AR-4, AR-5, AR-10, AR-13, AR-14, AR-15, AR-12 | the reproduced clipboard trace read 0 bytes | medium-high |

**Source note:** per-row sources in [00-ambiguity-register §Coverage Map](00-ambiguity-register.md#coverage-map). No Discovery `DQ-*` keys.

## Document Index

| #   | Document                                       | Description                                 |
| --- | ---------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                           | This document                               |
| 01  | [Requirements](01-requirements.md)             | Requirements and scope                      |
| 02  | [Current State](02-current-state.md)           | The four defects at `9cef1491`              |
| 03-01 | [Intuition and clipboard](03-01-intuition-and-clipboard.md) | AR-2, AR-10, AR-11 |
| 03-02 | [Screen text clipping](03-02-text-clipping.md) | AR-3..AR-5 |
| 03-03 | [Borrowed regions and guide messages](03-03-borrowed-objects.md) | AR-6..AR-9 |
| 03-04 | [Gates and docs](03-04-gates-and-docs.md)    | AR-13..AR-16 |
| 07  | [Testing Strategy](07-testing-strategy.md)     | ST cases and verification                   |
| 99  | [Execution Plan](99-execution-plan.md)         | Phases and task checklist                   |

## Quick Reference

### Usage Examples

```clo
// Safe after this plan: an alert before any screen, text running off the edge,
// a region freed while installed, a guide closed while a message is held.
showAlert("disk full");
drawText(screen, font, -12, 4, 1, bytes(stringOf("SCROLLING")));
layerSetClipRegion(layer, region);
freeRegion(region);          // the layer is unclipped first, then the region goes
closeAmigaGuide(guide);      // held messages are replied; their getters now read 0
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Intuition base | opened on demand through `cl_lib_open` (AR-2) |
| `drawText` off the screen | pixel-edge clip; fits → unchanged path; fully outside → nothing (AR-3..AR-5) |
| Region freed while installed | uninstalled from every layer, then disposed (AR-6, AR-7) |
| Open update | ended only when its layer is freed (AR-8) |
| Guide closed with held messages | replied at close; handles orphaned (AR-9) |
| Clipboard read | the unit is opened per operation; no read past the end (AR-18, superseding AR-10) |
| Spec | unchanged (AR-12) |

## Specialist Agents

**None** — the capability-gap check found no gap a standing specialist would close. Phase reviews
use the correctness reviewer plus the existing `amiga-platform-reviewer` (required for every
`std.amiga.*`, hosted-amiga shim, and FS-UAE gate diff, AGENTS.md §CodeOps).

_Detection evidence: the change surface is hosted-amiga shims, `std.amiga.*` doc comments, three
FS-UAE gate scripts, and Rust spec/impl tests; `amiga-platform-reviewer` covers the Kickstart
1.3–3.1 availability matrix and gate mechanics; no ownership, IR, codegen, or language-rule
change, so the memory-safety, emission-integrity, and semantics specialists are not triggered._

## Related Files

- Modified:
  - `compiler/runtime/shim_hosted_amiga.c` (`cl_show_alert`, `cl_text_draw`, `cl_layer` and its
    update/free paths, a registry walk helper)
  - `compiler/runtime/shim_hosted_amiga_20.c` (`cl_save_public_screen`)
  - `compiler/runtime/shim_hosted_amiga_optional.c` (regions)
  - `compiler/runtime/shim_hosted_amiga_optional_net.c` (AmigaGuide)
  - `compiler/runtime/shim_hosted_amiga_optional_services.c` (clipboard read)
  - `stdlib/std.amiga.{text,intuition,workbench,regions,amigaguide,clipboard}.clo` (doc comments)
  - `docker/scripts/a1200-addons-smoke.sh`, `docker/scripts/a600-addons-smoke.sh`,
    `docker/scripts/a500-fault-cleanup-smoke.sh`, `Makefile`
  - `docs/known-limitations.md`, pins in `compiler/tests/amiga_*_impl_test.rs` only where shim text intentionally changes
- Created:
  - `compiler/tests/amiga_crash_safety_{spec,impl}_test.rs`
  - `examples/robustness/crash-safety/*.clo`
