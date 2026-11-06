# Gate, Demo, and Media: A1200/UAEGFX

> **Document**: 03-03-gate-media.md
> **Parent**: [Index](00-index.md)
> **Second gate**: [Picasso IV](04-picasso-iv-gate.md)

## Media Contract

The gate expects the user-approved Aminet package at:

```
${AMIGA_DATA}/picasso96/Picasso96.lha
```

`P96_LHA` may override the path for a local checkout; the gate still verifies
the same SHA-256. For this repository the archive is installed at the media
path before the gate is run.

It verifies SHA-256
`0146d4a2aaadb82ec454dde6ca4fb33a3f646ca4c0daa46e569c025f4d9728af` before
extracting. The archive is Picasso96 2.0; the library reports 2.300.

`AMIGA_DATA` is read from `docker/.env`, the same variable the other licensed
media gates use. The script extracts the archive with
`/opt/amiga/bin/lha` inside the SDK container into:

```
build/amiga-sdk/a1200-rtg/p96/Picasso96Install/
```

Required runtime files:

```
Libs/Picasso96API.library
Libs/Picasso96/rtg.library
Libs/Picasso96/emulation.library
Libs/Picasso96/uaegfx.card
Devs/Monitors/Picasso96   -> copied as Devs/Monitors/uaegfx
Devs/Picasso96Settings.31 -> copied as Devs/Picasso96Settings
```

The P96 monitor program reads its board type from its icon tooltypes. When the
icon is missing it guesses the board type from the program name, so copying the
monitor as `uaegfx` is enough for the UAEGFX card. The WB3.1 startup sequence
executes every file in `DEVS:Monitors` after `BindDrivers`; the gate writes a
small startup sequence that runs `DEVS:Monitors/uaegfx` before the demo.

The archive is never committed. No decrypted ROM is committed; the gate
decrypts the licensed A1200 ROM into `build/` and removes it on exit, exactly
like the existing A1200 gates.

## Demo

`examples/amiga/optional-rtg.clo`:

1. Opens a 640x480x32 RTG screen and checks `rtgBoards()`, width, height,
   depth, and format.
2. Writes red at (200,200), green at (201,200), and blue at (202,200), and
   reads them back.
3. Writes and fills the top-left at (20,20)-(30,30), verifies (25,25), and
   fills a blue rectangle at (300,300)-(310,310) outside the capture.
4. Allocates a 32x32x32 bitmap, fills it blue, writes yellow at (0,0), and
   blits it to (210,210) for the capture and to (10,10) to prove the full
   display clip; both transfers are read back. The screen is opened with the
   title suppressed.
5. Reads a 64x64 RGB region from (200,200) into a `Buffer`, writes a P6 PPM to
   `CNEXT:rtg.ppm`, and writes the report to `CNEXT:rtg.txt`.
6. Prints `rtg: ok boards 1 mode 640x480x32 red 1 green 1 blue 1 blit 1`.

The demo uses `std.mem.bufferNew` for the 12,288-byte capture buffer, so the
Amiga stack is not asked to hold the image. The PPM header is
`P6\n64 64\n255\n`.

## Gate Steps

1. Build the demo for `hosted-amiga` with `--release` (the same release path
   used by the asset gates).
2. Stage Workbench 3.1 `Libs`, `Devs`, `C`, and `Classes` using
   `workbench-media.sh`.
3. Copy P96 libraries and the `uaegfx` monitor; write assigns for `LIBS:`,
   `DEVS:`, `FONTS:`, `ENV:`, `ENVARC:`, and `CNEXT:`.
4. Decrypt `amiga-os-310-a1200.rom` through `amitools`; verify SHA-1
   `e21545723fe8374e91342617604f1b3d703094f1`.
5. Start a private Xvfb display; no host window is shown.
6. Run FS-UAE with:
   - `--amiga-model=A1200`
   - `--chip-memory=2048 --fast-memory=0 --slow-memory=0` (the proven spike)
   - `--graphics-card=uaegfx`
   - the staged directory HD as `CNEXT:`
7. Poll for `rtg.txt` and `rtg.ppm` with a 90-second bound; stop the emulator.
8. Assert `rtg.txt` equals the expected report line.
9. Parse `rtg.ppm` with Python:
   - magic `P6`, 64x64, max value 255
   - exactly 12,288 payload bytes
   - red at (0,0), green at (1,0), blue at (2,0)
   - yellow at relative (10,10) and blue at relative (11,11) from the blitted
     bitmap, plus a blue pixel at relative (20,20) from the bitmap fill
10. Convert the PPM to `docs/amiga-sdk/a1200-rtg.png` for human inspection.
11. Remove the decrypted ROM on exit; the staged P96 extraction stays in the ignored `build/` tree for inspection.

## Failure Modes

| Condition | Gate result |
|---|---|
| P96 archive missing | exit 2 with the expected path and the Aminet URL |
| WB3.1 media missing | exit 2 with the licensed-media instruction |
| P96 library/board not found | report starts `rtg: err`, gate fails |
| mode open fails | report `rtg: err`, gate fails |
| wrong pixel or short PPM | Python check fails with the coordinate/value |
| emulator timeout | gate fails and prints the tail of the FS-UAE log |

## Baseline

- A1200/KS3.1, 68020, 2 MB chip, 0 fast memory (the proven spike).
- FS-UAE 3.1.66 with `uaegfx`.
- Picasso96 2.0 from Aminet, `Picasso96API.library` v2.
- Headless Xvfb display; no host input and no host window.
