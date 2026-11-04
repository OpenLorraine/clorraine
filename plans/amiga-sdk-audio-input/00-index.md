# Amiga SDK Bundle 3: Audio and Input

> **Status**: Done (9/9 tasks; make verify green)
> **Created**: 2026-11-04
> **Bundle**: `3-audio-input` (core; min OS 1.3; OCS/68000/512K)
> **Tracking**: [issue #4](https://github.com/OpenLorraine/clorraine/issues/4)
> **Registry**: [coverage.md](../../docs/amiga-sdk/coverage.md)
> **CodeOps Artifact Schema**: 1

## Purpose

Add audio.device open/close, four-channel sample playback, period/volume/loop
control, gameport joystick input, and timer.device delays. The acceptance is a
Pong-style paddle game with sound on the real A500/KS1.3 machine. The two
advanced `could` rows (EClock, ReadJoyPort) stay deferred to the advanced
bundles.

## Documents

| Document | Contents |
|---|---|
| [00-ambiguity-register.md](00-ambiguity-register.md) | Gate register; all items resolved |
| [01-requirements.md](01-requirements.md) | Scope, requirements, acceptance |
| [02-current-state.md](02-current-state.md) | Existing surface and gaps |
| [03-01-audio-api.md](03-01-audio-api.md) | `std.amiga.audio` design |
| [03-02-input-timer-game.md](03-02-input-timer-game.md) | Joystick, timer, paddle game, smoke |
| [05-findings.md](05-findings.md) | Review and runtime evidence |
| [07-testing-strategy.md](07-testing-strategy.md) | Specification cases and verification |
| [99-execution-plan.md](99-execution-plan.md) | Phases and task checklist |

## Definition of done

1. `SDK-AUD-001`–`005` and `007` are done with evidence.
2. `SDK-AUD-006` and `008` are deferred with the advanced-tier reason.
3. `examples/amiga/paddle.clo` runs on FS-UAE A500/KS1.3, shows the game, and
   prints `paddle: ok hits <N> audio <mask> test <bits> decode <bits> input 0`.
4. The live screen and paddle-line crops are committed.
5. `make check`, `make verify`, `make sdk-registry`, and the paddle smoke pass.
6. The hello, Boing, and sprite A500 gates stay green.
7. Issue #4 is closed and the roadmap/programme show Bundle 3 done.

## Specialist Agents

None as a project-local specialist — the bundle adds embedded standard-library
sources and C shim code, not compiler semantics, ownership lowering, IR, or
emission. The new shim has device and chip-memory paths; the phase review
includes an independent correctness/memory-safety pass. The host ASan auditor
cannot execute the m68k shim; the A500 runtime gates cover the target path.
The `analyze-agents` check on 2026-11-04 recorded `None` with 58 plan folders
in `codeops/specialist-check.json`.
