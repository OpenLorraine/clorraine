# Design: Sprite Demo, Collision, and A500 Smoke

> **Document**: 03-02-demo-collision.md
> **Parent**: [Index](00-index.md)
> **Covers**: R7, R8, R14; AR-9, AR-14

## Demo layout

`examples/amiga/sprite-demo.clo` uses an Intuition `Screen` for the static
background and the new sprite module for the moving objects.

| Element | Value |
|---|---|
| Screen | 320x256x4 Intuition custom screen |
| Background | blue top bar, green left half, red right half |
| Pair | channels 2/3, 16 colors from sprite registers 16..31 |
| Single | channel 4, registers 25..27 |
| Animation | channel 6, four 16x16 frames, registers 29..31 |
| Movement | 200 frames; the pair and single cross the screen |
| Collision | pair 2/3 vs single 4; bit 12 (`0x1000`) |
| Hold | the final frame is held for 800 ticks (16 s) |
| Final line | `sprite-demo: collision 0x1000` |

The animation frame is changed every frame with `animationNext`, which rebuilds
the screen list. The held frame uses animation frame 2, distinct from frame 0,
so the live golden proves frame switching.

## Deterministic collision

The demo finishes the animation, then:

1. Parks the pair's even and odd sprites at `(100, 80)`.
2. Parks the single sprite (channel 4) at `(100, 80)`.
3. Parks the animation at `(200, 150)` on frame 2.
4. Clears the accumulated `CLXDAT` by reading it once.
5. Waits two frames and reads `CLXDAT & 0x7E00`.
6. Holds the same frame and prints `sprite-demo: collision 0x1000`.

The background is static, the sprite pixels are fixed, and the positions are
exact, so the collision bit is stable across runs.

## Lifetime

The demo uses both supported orders:

- The pair and single call `spriteHide` + `spriteDetach` before the screen
  closes; `freeSprite` runs after the screen is gone.
- The animation is hidden but left attached; the screen close clears its
  back-pointer through the channel registry, and `freeAnimation` then runs
  safely after the screen is gone.

The A500 run therefore exercises the explicit detach path and the automatic
screen-close detach path.

## A500 smoke

`docker/scripts/a500-sprite-smoke.sh` mirrors the Boing smoke:

1. Build a clean `boot1x` ADF with `sprite-demo` and `DF0:sprite-demo`.
2. Decrypt the licensed KS1.3 ROM and verify the A500 SHA-1.
3. Boot FS-UAE as an A500.
4. Poll the held display crop `640x512+160+14` against
   `docs/amiga-sdk/a500-sprites.png`.
5. Poll the `430x22+160+88` line crop against
   `docs/amiga-sdk/a500-sprites-line.png`.
6. Stop the emulator and remove the decrypted ROM.

The Intuition screen is displayed at 2x (640x512) at +160+14 in the 960x540
FS-UAE window. The smoke fails if either the held frame or the line never
matches, and it requires at least one live frame with the sprite palette.

## Error behavior

- A missing screen or sprite allocation prints a distinct
  `sprite-demo: ... failed` line and exits 1.
- A collision read of zero prints `sprite-demo: collision 0x0000`, which the
  smoke rejects.
- The harness treats a missing ROM/tool/window as a hard failure.
