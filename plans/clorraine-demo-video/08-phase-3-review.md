# Phase 3 Review: Clorraine Demo Video Toolkit

> **Document**: 08-phase-3-review.md
> **Parent**: [Index](00-index.md)
> **Dispatch**: correctness-reviewer + security-auditor · scope mode: strict
> **Baseline tree**: 48a7fb6bdc08dbb37ba508336188194955c9bda1
> **Reviewed revision**: da88c60
> **Verification evidence**: `make demo-check` (48 tests; 1 skipped) + `DEMO_E2E=1` smoke green
> **Date**: 2026-09-24 23:24

## Summary

Independent correctness and security review of the Phase 3 diff (terminal, emulator,
composition). Four MAJOR correctness findings and several minor/observational items; the
security audit found no critical or major issues. The user ruled **apply all fixes**;
all findings are resolved and the smoke test is green.

## Findings

| # | Severity | Finding | Location | Resolution |
|---|----------|---------|----------|------------|
| RV-301 | 🟠 Major | The still frame throttled the overlay to 25 fps, dropping half the 50 fps capture | `renderer/emulator.ts` | `-loop 1 -framerate <fps>` + `overlay=…:shortest=1` + `-r <fps>`; locked by `emulator.impl.test.ts` |
| RV-302 | 🟠 Major | `frame:false` emulator clips are raw but marked canvas-ready, so they were never scaled | `renderer/compose.ts` | `isCanvasReady` treats `emulator` + `frame:false` as raw |
| RV-303 | 🟠 Major | `frame.html` bezel subtracted 80px again over the body padding, overhanging the screen | `assets/frame.html` | `.bezel { height: 100% }` |
| RV-304 | 🟠 Major | AR-16's runtime xfade length check was missing | `renderer/compose.ts` | ffprobe the output vs `expectedTotal`; throw beyond 0.25s |
| RV-305 | 🟡 Minor | Caption-font check fired for card/code captions | `renderer/compose.ts` | Scoped to terminal captions |
| RV-306 | 🟡 Minor | `contentHeight` was dead code | `renderer/env.ts` | Used in `buildNormalizeArgs` and `buildTape` |
| RV-307 | 🟡 Minor | `formatSeconds` doc overstated precision | `renderer/compose.ts` | Doc corrected |
| RV-308 | 🟡 Minor (sec) | `scene.id` not allowlisted before use as a path/arg | `scenes.ts` | `^[A-Za-z0-9_-]+$` in `validateConfig` |
| RV-309 | 🟡 Minor (sec) | `--out` override skipped `resolveRepoPath` | `renderer/compose.ts` | Routed through `resolveRepoPath` |
| RV-310 | 🔵 Obs | `--only` unknown ids were silently ignored | `renderer/compose.ts` | Validated against the config |
| RV-311 | 🔵 Obs | `waitForExit` timer leaked; cleanup failure was silent | `util/chrome.ts` | `clearTimeout`; warn on final failure |
| RV-312 | 🔵 Obs | `run` mapped a signal-killed child to exit 0 | `util/proc.ts` | Non-zero on a signal |
| RV-313 | 🔵 Obs | `escapeTapeValue` missed a lone `\r` | `util/vhs.ts` | `/\r\n?|\n/g` |

## Fix verification

`make demo-check` — tsc clean, 48 tests (1 skipped); `DEMO_E2E=1` smoke green
(1100x700, 50 fps, 6.5s, no audio). The four MAJOR fixes were re-reviewed once on the fix
diff; see the re-review note.

## Security audit

No 🔴/🟠 findings. Command execution uses argument arrays with no shell string; `record-emulator.sh`
quotes TS-supplied values and has no `eval`; caption text reaches ffmpeg only through `textfile=`.
The two 🟡 items (RV-308, RV-309) are fixed.
