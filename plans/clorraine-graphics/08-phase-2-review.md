# Phase 2 Review: Clorraine Amiga Graphics

> **Document**: 08-phase-2-review.md
> **Parent**: [Index](00-index.md)
> **Dispatch**: phase-reviewer (correctness, semantics, security) · scope mode: strict
> **Baseline tree**: 1ed1162917ac16afb0489fdc2dc29d75f42942cc
> **Reviewed revision**: fd30de3
> **Verification evidence**: `make check` green (15/15); `make corpus-amiga` (graphics builds, no warnings); `make corpus-fs-uae` (both legs match); `make verify` all legs passed
> **Date**: 2026-09-24 15:20

## Summary

Independent review of the Phase 2 diff (7 files: the graphics corpus program,
golden, README, the host-rejection test, the two corpus scripts, and the plan).
No critical or major findings. Five minor findings.

Verified correct: the golden is byte-exact (`graphics: ok\n`); `clc --profile
hosted-amiga` on the corpus program exits 0 with empty stderr; `--profile host`
is rejected with `E11200` naming `host`; the `run_leg` refactor is a semantic
copy of the former interrupt-counter logic, so no regression; the shared
`build/run/out.txt` is safe because `run.sh` wipes `build/run` on each invocation
and each leg kills and waits before the next; no shell quoting or injection
issue.

## Findings

| # | Severity | Finding | Location | Evidence | Recommended fix |
|---|----------|---------|----------|----------|-----------------|
| RV2-001 | 🟡 Minor | The corpus test module doc mentions only `interrupt-counter`, not the new `graphics` rejection test. | `compiler/tests/corpus_spec_test.rs:6-11` | The module doc says only interrupt-counter "must be rejected"; the new test doc names "screen and graphics libraries" while the first diagnostic is `std.amiga.dos`. | Add graphics to the module doc; name `std.amiga.dos` or drop the library list |
| RV2-002 | 🟡 Minor | Makefile comments are stale: they say `make corpus-fs-uae` boots only the interrupt-counter. | `Makefile:9-13` | Two programs now run under FS-UAE. | Say "the FS-UAE-only programs" and name graphics |
| RV2-003 | 🟡 Minor | The README says the program "holds the screen briefly", but `delayTicks(1500)` is about 30 seconds (1 tick = 1/50 s). | `spec/tests/corpus/graphics/README.md:3-4`; `program.clo:34` | `stdlib/std.amiga.dos.clo` defines one tick as one VBlank interval | State "about 30 seconds", or lower the value |
| RV2-004 | 🟡 Minor | The sentinel is printed before the 30-second hold, so the leg can stop the emulator before the owned-screen drop runs; the "owned-handle cleanup on real AmigaOS" claim is therefore fragile. In practice the AmigaDOS output buffer flushes at exit, so cleanup did run (the run's `bootlog.txt` contains `returned`). | `spec/tests/corpus/graphics/README.md:3`; `corpus-fs-uae.sh` (run_leg sentinel kill) | Sentinel at `program.clo:33`, hold at `:34`; drops at program exit | Print the sentinel after the screen's lifetime (for example, draw in a helper that owns the screen and print in `main`), or soften the cleanup claim |
| RV2-005 | 🟡 Minor | The corpus program's zero-diagnostic contract is not enforced by `make check`: no test compiles `spec/tests/corpus/graphics/program.clo` for hosted-amiga and asserts empty stderr. | `spec/tests/corpus/graphics/README.md:5` | `graphics_spec_test.rs` compiles a different synthetic program (no `delayTicks`); `corpus-amiga` enforces the C compiler's warnings but not the compiler's own diagnostics | Add a hosted-amiga clean-compile assertion for the corpus program |

## Security lens

Nothing to report. No shell injection surface (`name` is a literal; no `eval`).

## Rulings

The user approved fixing RV2-001 through RV2-005; all were applied and verified
(`make check` 15/15; `make verify` all legs, including both FS-UAE legs).

| # | Ruling | Action |
|---|--------|--------|
| RV2-001 | Fixed | The corpus test module doc now names both hosted-amiga-only programs and their diagnostics. |
| RV2-002 | Fixed | The Makefile comments now say `make corpus-fs-uae` boots the FS-UAE-only programs (interrupt-counter and graphics). |
| RV2-003 | Fixed | The README states the screen is held "about thirty seconds (1500 display ticks)". |
| RV2-004 | Fixed | The corpus program now draws in a helper that owns the screen; `main` prints the sentinel after the helper returns, so the screen drop runs before the emulator leg can stop. |
| RV2-005 | Fixed | `graphics_spec_test.rs` now compiles `spec/tests/corpus/graphics/program.clo` for hosted-amiga and asserts zero diagnostics. |
