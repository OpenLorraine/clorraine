# Current State: Amiga SDK Bundle 3 — Audio and Input

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing implementation

Bundles 0–2 provide the A500/1.3 profile, `std.amiga.video`, `std.math`,
`std.amiga.sprite`, the custom display and Intuition screen paths, and three
real-ROM A500 gates. The shim includes exec, graphics, intuition, DOS, and
hardware headers. It has no audio.device, timer.device, joystick, or CIAAPRA
code.

## Gaps

| Gap | Required change |
|---|---|
| No audio module | Add `stdlib/std.amiga.audio.clo` with `Audio`/`Sample` |
| No input module | Add `stdlib/std.amiga.input.clo` with `readJoystick` |
| No timer module | Add `stdlib/std.amiga.timer.clo` with `Timer` |
| No module registration | Add all three to `SOURCES`/`MODULES` and the 1.3 whitelist |
| No audio/input/timer shim | audio.device, JOY0DAT/CIAAPRA, timer.device |
| No game/smoke | `paddle.clo` and the A500 paddle smoke |

## Risks

| Risk | Mitigation |
|---|---|
| FS-UAE cannot open host audio | The emulated audio.device is independent; the game verifies the device/channel allocation and the API, and the user can hear it with host audio enabled |
| Channel allocation conflicts | The game requests all four channels; a busy channel makes the open fail, and the final line prints the allocated mask |
| Joystick decode differs on OCS | Use the documented 1.3 XOR table; the idle run expects `input 0` |
| Timer.device unit differences | Use `UNIT_VBLANK` (available on 1.3) and a `TimeRequest` |
| Deterministic hit count | The auto-play paddle tracks the ball; the game uses a fixed frame count and timer ticks |
