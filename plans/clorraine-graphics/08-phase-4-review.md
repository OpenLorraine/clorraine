# Phase 4 Review: Clorraine Amiga Graphics

> **Document**: 08-phase-4-review.md
> **Parent**: [Index](00-index.md)
> **Dispatch**: phase-reviewer (correctness / script and video) · scope mode: strict
> **Baseline tree**: f6cfc4753b90a0422f0aca1a5be789e23eb14c8f
> **Reviewed revision**: 89d63bc
> **Verification evidence**: `./demo/story.sh` wrote `demo-full.mp4` (161 s), `demo.gif` (39.7 s), `emulator.mp4` (9.0 s); a captured frame shows the bars and line; `make verify` all legs passed
> **Date**: 2026-09-24 20:56

## Summary

Independent review of the Phase 4 diff (README, `demo/story.sh`, and the plan).
No critical or major findings. One minor finding.

Checked clear: the story assembly is producer-before-consumer for every chapter
(variables, `write_text`, `normalize`, `card`, `concat`); the `program.clo` stem
collision between the string-parser and graphics corpus programs is not live
(each `record-emulator.sh` call blocks through its own build and capture);
the 9 s capture sits inside the ~30 s screen hold; the caption follows the
FS-UAE/emulator convention; the README matches the chapter order; no shell bug
under `set -euo pipefail`.

## Findings

| # | Severity | Finding | Location | Evidence | Recommended fix |
|---|----------|---------|----------|----------|-----------------|
| RV4-001 | 🟡 Minor | The plan's component spec still prescribes the caption `"Drawing on a real Amiga"`, which the repository convention forbids; a future agent could re-introduce it. The shipped caption is correct. | `plans/clorraine-graphics/03-04-spec-corpus-video.md` (Video Chapter) | Shipped `demo/story.sh` caption: `"Drawing on an Amiga screen — under FS-UAE (Amiga emulator)"`; all other captions say FS-UAE. | Update the plan line to the shipped wording |

## Rulings

RV4-001 is minor (report-only). Fixed: the component spec now matches the
shipped caption and the FS-UAE convention.
