---
title: Boing graphics
description: "The Clorraine Boing demo: chip-memory bitmaps, a copper list, a masked blit, double buffering, and a frame checksum on A500."
---

# Boing graphics

The Boing demo is the native-video acceptance program. It owns a 320×256×4
custom display built from two chip-memory bitmaps, draws a checkerboard and a
shaded ball, and flips one view per frame.

<DocFigure
  src="/screenshots/a500-boing.webp"
  alt="A chequered Amiga screen with a shaded sphere moving across it."
  caption="Native capture from the A500 / Kickstart 1.3 / OCS gate."
/>

## What it exercises

| Capability | Use in the demo |
|---|---|
| Chip memory | Two frame bitmaps and a ball/mask pair |
| Bitmap drawing | Checkerboard fill and a shaded sphere |
| Copper | Three background colours change every sixteen scanlines |
| Blitter | Masked copy of the ball into the visible frame |
| Double buffering | One frame buffer shown while the other is drawn |
| Frame checksum | A deterministic final-frame oracle |
| Display lifecycle | Open a custom screen and restore the system view |

## A source excerpt

```clorraine
func drawBackground(borrow bitmap: Bitmap): void {
    bitmapClear(bitmap, 0);
    let y: i32 = 0;
    while (y < HEIGHT) {
        let x: i32 = 0;
        while (x < WIDTH) {
            let pen: u8 = u8(((x / TILE) + (y / TILE)) % 3 + 1);
            bitmapFill(bitmap, x, y, x + TILE - 1, y + TILE - 1, pen);
            x = x + TILE;
        }
        y = y + TILE;
    }
}
```

```clorraine
func drawBall(borrow ball: Bitmap, borrow mask: Bitmap): void {
    bitmapClear(ball, 0);
    bitmapClear(mask, 0);
    let radius: i32 = BALL / 2;
    let y: i32 = 0;
    while (y < BALL) {
        let x: i32 = 0;
        while (x < BALL) {
            let dx: i32 = x - radius;
            let dy: i32 = y - radius;
            let distance: i32 = dx * dx + dy * dy;
            if (distance <= radius * radius) {
                let light: i32 = (dx + 10) * (dx + 10) + (dy + 10) * (dy + 10);
                let shade: i32 = 15 - light / 96;
                if (shade < 4) {
                    shade = 4;
                }
                bitmapPlot(ball, x, y, u8(shade));
                bitmapSetPlaneBit(mask, 0, x, y, true);
            }
            x = x + 1;
        }
        y = y + 1;
    }
}
```

The complete program is
[`examples/amiga/boing.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/amiga/boing.clo).

## Run it

The simplest Amiga path uses `vamos`, but Boing needs a display, so use FS-UAE:

```sh
make sdk-a500-boing
```

That builds for `hosted-amiga-13`, boots FS-UAE as an A500 with licensed
Kickstart 1.3, shows the live custom display, and compares the final frame
checksum line and the native capture.

For an interactive window:

```sh
make run FILE=examples/amiga/boing.clo PROFILE=hosted-amiga-13 EMULATOR=1
```

## The gate line

The program prints the checksum of the visible frame:

```text
boing: checksum 0xF58C7129
```

The A500 gate compares that line and the captured frame. The checksum is
deterministic because the ball position is fixed-point and the frame count is
fixed. It is a correctness oracle, not a benchmark.

## What to inspect

- The ball is blitted through a single-plane mask, so the corners are
  transparent even though the ball bitmap is square.
- The copper list changes only the background colours, not the ball.
- The display restores the system view before the program reports.
- The A500 path uses `-mcrt=nix13`, 68000, and OCS; there is no AGA dependency.

## Learn more

- [SDK coverage](/amiga/sdk-coverage)
- [Amiga profiles](/amiga/profiles)
- [Emulator workflow](/amiga/emulator)
- [`std.amiga.video`](https://github.com/OpenLorraine/clorraine/blob/main/stdlib/std.amiga.video.clo)
