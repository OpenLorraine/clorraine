# Gate: Emulated Picasso IV (24-bit)

> **Document**: 04-picasso-iv-gate.md
> **Parent**: [Index](00-index.md)
> **Scope**: second RTG runtime gate on FS-UAE's emulated Picasso IV Zorro III
> card

## Why the card, why 24-bit

The UAEGFX gate proves the 32-bit true-alpha path. It cannot prove that the
surface works with a real-card P96 driver, because UAEGFX is an emulator-only
virtual card. FS-UAE also emulates the Village Tronic Picasso IV, and the
Picasso96 2.0 archive ships `PicassoIV.card` plus the Cirrus GD5446 chip
driver for it.

The real Picasso IV exposes 24-bit `B8G8R8` truecolor. It does not advertise
any 32-bit true-alpha format. The module therefore accepts depth 24 as well
as 32: 24-bit requests use `RGBFF_TRUECOLOR` and `B8G8R8`, 32-bit requests
keep `RGBFF_TRUEALPHA` and `A8R8G8B8`.

## Media

| Item | Requirement |
|---|---|
| Kickstart | A4000 3.1 ROM, decrypted with `rom.key`; SHA-1 `5fe04842d04a489720f0f4bb0e46948199406f49` |
| Workbench | `amiga-os-310-workbench.adf` |
| Picasso96 | Aminet `Picasso96.lha`, SHA-256 `0146d4a2aaadb82ec454dde6ca4fb33a3f646ca4c0daa46e569c025f4d9728af` |
| Picasso IV ROM | 7.4 flash image, SHA-1 `cafafb916f16b9f3ec9b49aa4b40eb4eeceb5b5b`, staged as `picasso_iv_flash.rom` |
| Monitor | `Devs/Monitors/Picasso96` copied as `PicassoIV`, with `Picasso96.info` next to it |
| Drivers | `Libs/Picasso96/PicassoIV.card` and `CirrusGD5446.chip` |
| Settings | `Devs/Picasso96Settings.31`, attached with `Picasso96/AttachSettings PicassoIV 0` |

The P96 archive and ROM are user-provided and read-only. The gate copies them
into a disposable `build/` directory and never commits them.

## Emulator configuration

- `--amiga-model=A4000`, 2 MB chip, 8 MB fast, 68040.
- `--graphics-card=picasso-iv-z3` (Zorro III; the A1200's 24-bit address space
  is not compatible with the Z3 RTG aperture).
- `--kickstarts-dir=<work>/Kickstarts` so FS-UAE's ROM scanner finds
  `picasso_iv_flash.rom`.
- Private Xvfb display; no host emulator window.
- `--governor-warning=0` to suppress the host CPU warning dialog.

The Zorro II variant (`--graphics-card=picasso-iv` on A1200) also runs the
24-bit demo once `PicassoIV.card`, `CirrusGD5446.chip`, the monitor icon, and
the settings attachment are staged. The committed gate still uses the native
Z3 path on an A4000: it exercises the card's 32-bit aperture and the
configuration a real Picasso IV normally uses. The `picasso-iv-z2` spelling is
not a valid FS-UAE 3.1.66 value and is silently ignored; use `picasso-iv` for
the auto/Z2 path.

## Startup sequence

```text
CNEXT:AttachSettings PicassoIV 0 Devs:Picasso96Settings >CNEXT:attach.txt
DEVS:Monitors/PicassoIV
CNEXT:C/Wait 2
CNEXT:optional-rtg >CNEXT:rtg.txt
```

`AttachSettings` attaches the supplied `Village-31` monitor modes to the
PicassoIV board. Without it `rtgMode` returns 0 for every depth even though
the card is visible to `CheckBoards`.

## Expected evidence

Report:

```text
rtg: accessors boards 1 mode 640x480x24 screen 640 480 24 3 bmp 32 32 24 3 96 fill 1 bmpread 1
rtg: ok boards 1 mode 640x480x24 red 1 green 1 blue 1 blit 1
```

The 64x64 PPM at (200,200) is checked for the same red/green/blue probe
pixels and the yellow/blue blit as the UAEGFX gate. The artifact is
`docs/amiga-sdk/a4000-rtg-piv.png`; the PPM stays under
`build/amiga-sdk/a4000-rtg-piv/`.

## Shared gate script

`docker/scripts/rtg-smoke.sh` is the single implementation. It selects the
model, ROM, memory, card, P96 files, monitor, expected report, work directory,
and artifact from a card name. `a1200-rtg-smoke.sh` and
`a4000-rtg-piv-smoke.sh` are thin entry points. Both gates use the same PPM
parser and `convert -strip` artifact step.

## Failure modes

| Symptom | Cause | Fix |
|---|---|---|
| `rtg: err 1`, `rtgBoards()` 1 but mode 0 | settings not attached | run `AttachSettings PicassoIV 0` before the monitor |
| Monitor prints "Could not create graphics board context" | `CirrusGD5446.chip` not staged | copy both `PicassoIV.card` and the chip driver |
| FS-UAE "Z3 RTG and 24-bit address space" | A1200 model with `picasso-iv-z3` | use `picasso-iv` for the Z2 path or A4000 for Z3; the gate uses A4000 |
| No flash ROM loaded | ROM not in the Kickstarts dir | stage `picasso_iv_flash.rom` and pass `--kickstarts-dir` |
