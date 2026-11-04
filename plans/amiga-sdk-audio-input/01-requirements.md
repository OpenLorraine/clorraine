# Requirements: Amiga SDK Bundle 3 — Audio and Input

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Functional Requirements

### Must Have

- **R1 — Audio device.** `openAudio(channels)` opens `audio.device`, creates a
  reply port, allocates 1..4 channels, and returns `null` on failure. The close
  frees the channels and closes the device.
- **R2 — Four channels.** The handle reports the allocated channel mask and
  exposes play/stop/control per allocated channel. The game opens four channels
  and plays on channel 0.
- **R3 — Samples.** `newSample(data, period, volume, cycles)` copies the
  caller's signed 8-bit samples into chip memory, pads an odd length, and
  stores period/volume/cycles. A zero-length or unallocated sample returns
  `null`.
- **R4 — Playback control.** `audioPlay` starts an asynchronous `CMD_WRITE`
  with `ADIOF_PERVOL`; `audioStop` sends `ADCMD_FINISH`; `audioControl` sends
  `ADCMD_PERVOL` for a playing channel. `cycles = 0` loops, `1` plays once.
- **R5 — Joystick.** `readJoystick(port)` reads `JOY0DAT`/`JOY1DAT` and the
  CIAAPRA fire bit and returns a bitmask (up, down, left, right, fire) for
  port 0 or 1 on 1.3.
- **R6 — Timer.** `openTimer()` opens timer.device `UNIT_VBLANK`;
  `timerWait(timer, ticks)` uses `TR_ADDREQUEST` and waits; the close releases
  the device.
- **R7 — Paddle game.** `examples/amiga/paddle.clo` opens a 320x256 Intuition
  screen, draws a Pong-style court, moves a paddle from the joystick (or a
  deterministic auto-play fallback), bounces a ball, plays a beep on each
  paddle hit, uses the timer for the frame loop, holds a deterministic final
  frame, and prints `paddle: ok hits <N> audio <mask> test <bits> decode <bits>
  input 0` after the audio/input self-tests.
- **R8 — Runtime proof.** `docker/scripts/a500-paddle-smoke.sh` boots the game
  on the real KS1.3 ROM, compares the held screen and the paddle line to
  committed goldens, and fails if either is missing.
- **R9 — Compatibility.** The three modules are available on `hosted-amiga`
  and `hosted-amiga-13`, rejected by host, and leave the earlier A500 gates
  green.
- **R10 — Documentation.** Update the appendices, `AGENTS.md`, registry,
  coverage, issue #4, and the programme/roadmap.

### Should Have

- **R11 — Deferred advanced rows.** `SDK-AUD-006` (ReadJoyPort, 3.1+) and
  `SDK-AUD-008` (EClock, 2.0+) are marked `deferred` with the advanced-tier
  reason; the core bundle does not claim them.
- **R12 — Deterministic hit count.** The auto-play paddle tracks the ball, so
  the final hit count is stable across the FS-UAE run.

### Won't Have (Out of Scope)

- Audio mixing, MOD playback, DMA sample streaming, or more than four channels.
- gameport.device/lowlevel.library; the 1.3 raw gameport is the core path.
- Keyboard/mouse input beyond the joystick gameport.
- Any language, grammar, diagnostic, or ABI change.

## Technical Requirements

- Sample data is chip memory, word-aligned, and an even byte length.
- Audio requests use the allocation key returned by the device.
- A busy channel returns `false` from `audioPlay`; the caller may stop it first.
- The timer uses `UNIT_VBLANK`; one tick is one vertical blank.
- The joystick decode follows the 1.3 Hardware Reference: JOY0DAT bit 1 =
  right, bit 9 = left, bit 1 XOR bit 0 = down, bit 9 XOR bit 8 = up; fire is
  CIAAPRA bit 6 (port 0) or bit 7 (port 1), active low.

## Acceptance Criteria

1. The modules compile on both hosted profiles, cross-build with
   `nix13`/`nix20`, and are rejected by host.
2. The spec tests cover audio open/close, sample storage, channel masks,
   joystick decode, and timer open/wait.
3. The A500 paddle game shows the court and prints the deterministic line.
4. `make check`, `make verify`, `make sdk-registry`, and the paddle smoke pass.
5. The hello, Boing, and sprite gates stay green.
6. Registry Bundle 3 is 6/8 done with 2 deferred; issue #4 closed.
