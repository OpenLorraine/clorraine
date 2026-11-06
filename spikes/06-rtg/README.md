# RTG spike: Picasso96 on FS-UAE UAEGFX

This spike proved the stack before the compiler surface was written.

## Media

- Aminet `driver/video/Picasso96.lha` (Picasso96 2.0), SHA-256
  `0146d4a2aaadb82ec454dde6ca4fb33a3f646ca4c0daa46e569c025f4d9728af`
- Runtime files copied into a clean WB3.1 directory HD:
  `Libs/Picasso96API.library`, `Libs/Picasso96/rtg.library`,
  `Libs/Picasso96/emulation.library`, `Libs/Picasso96/uaegfx.card`,
  `Devs/Monitors/uaegfx` (the P96 monitor renamed so it guesses its board
  type), and `Devs/Picasso96Settings`.
- FS-UAE 3.1.66, A1200/KS3.1, `--graphics-card=uaegfx`,
  `--chip-memory=2048 --fast-memory=0 --slow-memory=0`.

## Results

A direct C probe opened `Picasso96API.library` v2, queried one board, opened
the best 640x480x32 mode, and reported:

```
probe: ok boards 1 mode 50031303 w 640 h 480 depth 32 fmt 9 bfmt 6 \
  pixel ff00 bluepx ff bluefill ff bitmap ffff00 ff ff \
  blit ffff00 ff rgb 255 255 0 0 0 255
```

- Screen format is `RGBFB_B8G8R8A8` (9); a user-private bitmap reports
  `RGBFB_A8R8G8B8` (6).
- `p96WritePixel`/`p96ReadPixel` round-trip ARGB values.
- A bounded `p96WritePixel` loop fills the user-private bitmap correctly
  (`bitmap ... ff ff`), and `BltBitMapRastPort` with minterm `0xC0` copies it
  to the screen.
- `p96ReadPixelArray` with `RGBFB_R8G8B8` returns top-down RGB bytes
  (`rgb 255 255 0 0 0 255` for yellow and blue).

## Negative control

`p96RectFill` on the same user-private bitmap did not round-trip the pure-blue
value: filling with `0x000000FF` read back as red (`ff0000`). The shipped shim
therefore fills bitmaps with the bounded pixel loop and keeps `p96RectFill` for
screen fills only.

## Screen clip

`p96ReadPixel` returned 0 for a top-left blit at (10,10) even though
`p96ReadPixelArray` read the copied yellow/blue pixels; `p96ReadPixel` follows
the screen's top-left clip. The shim reads pixels through a 1x1
`p96ReadPixelArray` and converts RGB back to ARGB, so the top-left blit and the
capture at (200,200) both read correctly. The gate checks both blits.

## Emulated Picasso IV

The same surface also runs on FS-UAE's emulated Picasso IV Zorro III card,
which exposes 24-bit `B8G8R8` only. The A4000/KS3.1 gate stages
`PicassoIV.card` plus `CirrusGD5446.chip`, attaches the supplied
`Village-31` settings with `AttachSettings PicassoIV 0`, and stages the 7.4
flash ROM as `picasso_iv_flash.rom` in the emulator's Kickstarts directory.
The demo prefers 32-bit and falls back to 24-bit; the gate reports
`rtg: ok boards 1 mode 640x480x24 red 1 green 1 blue 1 blit 1` and verifies
the same PPM pixels. The Zorro II variant (`--graphics-card=picasso-iv` on A1200) also runs the
24-bit demo after the same staging. The committed gate uses the card's native
Z3 path on an A4000, which exercises the 32-bit aperture a real Picasso IV
normally uses. The `picasso-iv-z2` spelling is not a valid FS-UAE 3.1.66 value
and is silently ignored.

## Re-run

The committed gates are `make sdk-a1200-rtg` (32-bit UAEGFX) and
`make sdk-a4000-rtg-piv` (24-bit emulated Picasso IV). Both use the shared
`docker/scripts/rtg-smoke.sh`, check the report line, and verify a 64x64 PPM
capture.
