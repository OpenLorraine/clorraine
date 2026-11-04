# Findings: Amiga SDK Bundle 3 — Audio and Input

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Done

## Implementation findings

| # | Finding | Evidence | Disposition |
|---|---|---|---|
| F-1 | `audio.device` allocates channels through `OpenDevice` with a mask array; the device returns `io_Unit`/`ioa_AllocKey`. | The audio stage printed `mask 0xF` on the real A500. | The shim stores one `IOAudio` block per allocated channel plus a control block. |
| F-2 | timer.device `TimeVal` is seconds/microseconds even for `UNIT_VBLANK`. | The first paddle run moved at ~1 frame/s because `timerWait(1)` meant one second. | The shim converts PAL 50 Hz ticks: `tv_secs = ticks / 50`, `tv_micro = (ticks % 50) * 20000`. |
| F-3 | The joystick directions are not direct bits in `JOY0DAT`; the 1.3 table uses XORs. | The stage read `joy0 0x0 joy1 0x0` with no input. | `decodeJoystick` implements the HRM table and is self-tested with synthetic words. |
| F-4 | The game's idle path must be deterministic for the A500 smoke. | The auto-play paddle tracks the ball; the run produced `hits 2`. | The final court is parked at paddle x=136, ball (160,100). |
| F-5 | The review asked for positive evidence of four channels, loop/stop, and the decode. | The paddle self-test prints `audio 15 test 15 decode 127` on the A500. | The game runs the audio/input self-tests before the match. |

## Red phase

The spec and impl tests were written before the modules existed:

```text
error E11102: 'Audio' is not exported from module 'std.amiga.audio'
error E11102: 'readJoystick' is not exported from module 'std.amiga.input'
error E11102: 'Timer' is not exported from module 'std.amiga.timer'
...
test result: FAILED. 0 passed; 2 failed
```

## Green phase

| Gate | Result |
|---|---|
| `cargo test --test amiga_audio_spec_test` | 7 passed (host/A500 compile, host rejection for audio/input/timer, `nix13`/`nix20` cross-builds, Exec still gated) |
| `cargo test --test amiga_audio_impl_test` | 2 passed (wrapper routing, null-guarded drops) |
| `make sdk-a500-paddle` | OK; held court and result line matched |
| `make verify` | exit 0; hello, Boing, sprites, and paddle A500 gates passed |
| `make sdk-registry` | OK (79 capabilities, 10 bundles; Bundle 3 6/8 with 2 deferred) |

## Runtime evidence

| Evidence | Value |
|---|---|
| Live court | `docs/amiga-sdk/a500-paddle.png` (640x512, held frame) |
| Golden result line | `docs/amiga-sdk/a500-paddle-line.png` |
| Result | `paddle: ok hits 2 audio 15 test 15 decode 127 input 0` |
| Audio self-test | 4-channel play/replay (bits 1–2), loop/stop/replay (bits 4–8) = 15 |
| Decode self-test | right, left, down, up, fire, two diagonals = 127 |
| Machine | FS-UAE A500, real Kickstart 1.3 rev 34.5, decrypted licensed ROM |
| ROM SHA-1 | `891e9a547772fe0c6c19b610baf8bc4ea7fcb785` |

The FS-UAE container has no host audio output, so audible sound is a manual
check; the device allocation, four-channel writes, completion, loop, stop, and
replay are verified on the emulated A500.

## Review ledger

The independent review found no critical or memory-safety defect. Two major
evidence gaps and several minor documentation/test issues were fixed.

| ID | Severity | Finding | Fix |
|---|---|---|---|
| A1 | Major | Four-channel/loop/stop playback was not positively evidenced; only one one-shot channel was played. | The paddle self-test plays all four channels, waits, replays all four, starts a loop sample, stops it, and replays a one-shot; `test 15` is in the committed line. |
| A2 | Major | The joystick decode had no positive test; an always-zero implementation would pass. | `decodeJoystick` is a pure helper; the game self-tests right, left, down, up, fire, and two diagonals; `decode 127` is in the committed line. |
| P1 | Minor | Issue #4 was still open while the plan said done. | Closed after `make verify`; plan status updated. |
| P2 | Minor | The findings document was missing. | This file. |
| D1 | Minor | Plan text said `CMD_STOP`, `tv_secs = ticks`, and a one-channel fallback. | Corrected to `ADCMD_FINISH`, the 50 Hz conversion, and all-or-nothing four-channel allocation. |
| C1 | Minor | `SendIO`/`DoIO` audio-flag caveat. | Comment pins `ADIOF_PERVOL` and the absence of `ADIOF_NOWAIT`. |
| C2 | Minor | Sample length unbounded. | Cap at 128 KB; documented. |
| C3 | Minor | Timer conversion is PAL-only. | Documented in the module and appendix; the target is PAL. |
| T1 | Minor | Smoke had a stale sprite comment and a weak liveness check. | Comment fixed; the held-frame crop is the liveness gate. |
| T2 | Minor | Host rejection covered only audio; input values were unasserted. | Input/timer host rejection added; the decode self-test covers the values. |
| A3 | Minor | Specialist check predated Bundle 3. | Refresh after the plan is added; the state records 58 plan folders. |

Accepted residuals:

- A sample must stay alive while playing; `freeSample` cannot detect playback.
- `audioPlay` reports request acceptance, not completion; the self-test covers
  completion by replaying after a wait.
- `openAudio(4)` is all-or-nothing; a busy channel returns `null`.
- Timing is PAL 50 Hz; NTSC waits are approximate.
- Audible audio is a manual check because the container has no host audio.
