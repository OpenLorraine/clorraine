# Design: Joystick, Timer, Paddle Game, and A500 Smoke

> **Document**: 03-02-input-timer-game.md
> **Parent**: [Index](00-index.md)
> **Covers**: R5–R8, R11–R12; AR-7–AR-10

## Joystick

`std.amiga.input.readJoystick(port)` reads `JOY0DAT` for port 0 or `JOY1DAT`
for port 1 and the CIAAPRA fire bit. The 1.3 Hardware Reference decode is:

```text
right = joy & 0x0002
left  = joy & 0x0200
down  = (joy ^ (joy >> 8)) & 0x0001 ?   // bit 1 XOR bit 0
up    = ((joy >> 8) ^ (joy >> 9)) & 0x0001
fire  = CIAAPRA bit 6 (port 0) or bit 7 (port 1) == 0
```

The result is a bitmask: `JOY_UP=1`, `JOY_DOWN=2`, `JOY_LEFT=4`,
`JOY_RIGHT=8`, `JOY_FIRE=16`. Ports other than 0/1 return 0.

## Timer

`std.amiga.timer` opens `timer.device` `UNIT_VBLANK` with a `TimeRequest`.
`timerWait(timer, ticks)` converts PAL 50 Hz ticks to
`tv_secs = ticks / 50`, `tv_micro = (ticks % 50) * 20000`, sets
`tr_node.io_Command = TR_ADDREQUEST`, and calls `DoIO`. The close calls
`CloseDevice`. NTSC waits are approximate because the conversion is PAL.

## Paddle game

| Element | Value |
|---|---|
| Screen | 320x256x4 Intuition screen |
| Court | black, top wall, side walls, paddle at y=220 |
| Paddle | 48 px wide, follows the joystick; idle fallback tracks the ball |
| Ball | 8x8, velocity 2 px/frame, bounces off walls and paddle |
| Frames | 300 timer ticks (6 s at 50 Hz) |
| Sound | 8 kHz beep on every paddle hit, four audio channels opened |
| Held frame | final court held for 800 ticks (16 s) |
| Final line | `paddle: ok hits <N> audio <mask> input 0` |

The idle fallback sets the paddle x to `ball_x - 24`, so the ball always meets
the paddle and the hit count is stable. A real joystick overrides the fallback.
The game prints the allocated audio mask (expected `15`) and the idle joystick
mask (expected `0`).

## A500 smoke

`docker/scripts/a500-paddle-smoke.sh` mirrors the other A500 gates:

1. Build a clean `boot1x` ADF with `paddle` and `DF0:paddle`.
2. Decrypt the licensed KS1.3 ROM and verify the A500 SHA-1.
3. Boot FS-UAE as an A500.
4. Poll the held display crop `640x512+160+14` against
   `docs/amiga-sdk/a500-paddle.png`.
5. Poll the `430x22+160+88` line crop against
   `docs/amiga-sdk/a500-paddle-line.png`.
6. Stop the emulator and remove the decrypted ROM.

## Errors

- A missing screen, timer, audio device, or sample prints a distinct
  `paddle: ... failed` line and exits 1.
- `audioPlay` returning `false` on a busy channel does not stop the game; the
  next hit can play.
- The smoke treats a missing ROM/tool/window as a hard failure.
