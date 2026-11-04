# Execution Plan: Amiga SDK Bundle 3 — Audio and Input

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-04 19:15
> **Progress**: 9/9 tasks (100%)
> **CodeOps Artifact Schema**: 1

## Overview

Three phases: pin the new audio/input/timer surface with specification tests,
implement the shim and modules, then build and prove the paddle game on the
real A500/KS1.3 machine. Specification tests come before implementation.

## Phase 1: Specification tests (red)

> **Reasoning**: high — three new public modules and device paths.

**Tasks**: 2

- [x] 1.1.1 Write `compiler/tests/amiga_audio_spec_test.rs` for ST-1–ST-3 and ST-6; run it red
- [x] 1.1.2 Write `compiler/tests/amiga_audio_impl_test.rs` for ST-4; run it red

**Verify**: `cargo test --test amiga_audio_spec_test --test amiga_audio_impl_test`

## Phase 2: Audio, input, and timer implementation

> **Reasoning**: high — device allocation, chip-memory samples, custom-chip
> joystick decode, and timer.device.

**Tasks**: 4

- [x] 2.1.1 Add `stdlib/std.amiga.audio.clo`, `std.amiga.input.clo`, and `std.amiga.timer.clo`; register them and add them to the 1.3 whitelist; run ST-1–ST-3 green
- [x] 2.1.2 Implement audio.device open/close, channel allocation, samples, play/stop/control in the hosted-Amiga shim; cross-build with `nix13`/`nix20`
- [x] 2.1.3 Implement `readJoystick` and timer.device open/wait/free; cross-build
- [x] 2.1.4 Run the full `cargo test --workspace` and `make check`

**Verify**: `cargo test --workspace`; `make check`

## Phase 3: Paddle game and runtime close-out

> **Reasoning**: high — real KS1.3 emulator acceptance with audio and input.

**Tasks**: 3

- [x] 3.1.1 Add `examples/amiga/paddle.clo`; build it for `hosted-amiga-13`; verify the game, audio, joystick, and timer on the A500
- [x] 3.1.2 Add `docker/scripts/a500-paddle-smoke.sh` and `make sdk-a500-paddle`; capture the held screen and paddle line; add the leg to `make verify`
- [x] 3.1.3 Update the appendices, `AGENTS.md`, registry/coverage, issue #4, programme/roadmap; run `make verify` and `make sdk-registry`; close the plan

**Verify**: `make verify`; `make sdk-registry`; `docker/scripts/a500-paddle-smoke.sh build/clorraine/paddle/hosted-amiga-13/paddle`

## Dependencies

- Phase 1 is the red oracle.
- Phase 2.1.2/2.1.3 depend on 2.1.1; 2.1.4 depends on both.
- Phase 3 depends on Phase 2.
- The A500 smoke depends on the licensed ROM/`rom.key`, the host X display,
  and the Bundle 1 screen path.

## Success Criteria

1. All 9 tasks complete.
2. The three modules compile on both hosted profiles and are rejected by host.
3. The A500 paddle game shows the court and prints the deterministic line.
4. The hello, Boing, and sprite A500 gates stay green.
5. Registry Bundle 3 is 6/8 done with 2 deferred; issue #4 closed.
6. No language, grammar, diagnostic, or ABI change.

## Completion

Completed on 2026-11-04. `make sdk-a500-paddle` passes with the held court and
`paddle: ok hits 2 audio 0xF input 0x0`. The two advanced rows are deferred.
Registry Bundle 3 is 6/8 done; issue #4 is closed.
