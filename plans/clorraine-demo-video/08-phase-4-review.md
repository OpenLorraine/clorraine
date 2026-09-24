# Phase 4 Review: Clorraine Demo Video Toolkit

> **Document**: 08-phase-4-review.md
> **Parent**: [Index](00-index.md)
> **Dispatch**: correctness-reviewer + security-auditor · scope mode: strict
> **Baseline tree**: 748e8b1ff31f2c35a1af3e9a04d03d136e259de5
> **Reviewed revision**: f5217cf
> **Verification evidence**: `make demo` (all four outputs) + `make demo-check` (58 tests)
> **Date**: 2026-09-25 00:35

## Summary

Independent correctness and security review of the Phase 4 diff (story migration, the
`render` CLI wiring, and the xfade/VHS path fixes). No critical or major findings. The
security audit was clean. Four minor correctness findings were fixed; one informational
note was accepted.

## Findings

| # | Severity | Finding | Location | Resolution |
|---|----------|---------|----------|------------|
| RV-401 | 🟡 Minor | A relative `--root` resolved twice for child processes (the tape/script path plus `cwd`) | `cli.ts`, `terminal.ts` | `buildRenderEnv` resolves the root to an absolute path once |
| RV-402 | 🟡 Minor | An xfade offset goes negative when a measured clip is shorter than the transition | `renderer/compose.ts` | Reject a clip not longer than the transition, with a clear message |
| RV-403 | 🟡 Minor | Unknown `--only` ids threw a plain error (exit 1) while other `--only` misuse exits 2 | `renderer/compose.ts` | Throw `ConfigError` so the CLI returns 2 |
| RV-404 | 🟡 Minor | The 86-line `source` chapter did not scroll, so only the first ~30 lines were visible | `demo/story.ts` | Set `scroll: true`; locked by `story.impl.test.ts` |
| RV-405 | 🔵 Info | The ownership chapter shows the compiler error but not the source file (the old tape showed it in Vim) | `demo/story.ts` | Accepted: matches the 03-04 table; no action |

## Security audit

**No findings.** `--out` goes through `resolveRepoPath`; caption text reaches ffmpeg only
via `textfile=`; VHS commands are escaped and run with an argv array (no shell); the
Makefile uses fixed literals; `demo/story.ts` has no secrets or host paths; the committed
`demo/poster.png` is a plain card frame. One informational note: `resolveRepoPath` is
check-then-use and could be raced if the repository tree were untrusted — not exploitable
under the local trusted-operator model, so no change.

## Fix verification

- `make demo-check` — tsc clean, 58 tests (1 skipped).
- `make demo` — wrote `demo/demo-full.mp4` (120.5s, 1100x700, 50 fps),
  `demo/demo.gif`, `demo/emulator.mp4` (7.0s), and `demo/poster.png`.
- `render … --only source` — 12.0s clip, confirming the scrolling chapter renders.
