---
title: Emulator workflow
description: "Run Clorraine programs on an emulated Amiga 1200 in FS-UAE, and understand the ROM and media requirements."
---

# Emulator workflow

Clorraine targets real 68K Amiga systems. On your Linux PC, the
[installer](/guide/quick-start) sets up an emulated **Amiga 1200 with
Kickstart 3.1** in FS-UAE, and `clorraine run --emulator` boots it with your
program.

| Tool | Best for | ROM |
|---|---|---|
| FS-UAE (A1200, Kickstart 3.1) | Graphics, audio, Intuition, timers, console output | Your licensed Kickstart 3.1 A1200 ROM |
| The browser player | The showcase demos on this site | None; it uses AROS |

## Run a program in FS-UAE

The installer finds FS-UAE and your Kickstart ROM and remembers them, so a run
needs only the profile and `--emulator`:

```sh
clorraine run -p hosted-amiga --emulator hello.clo
```

An Amiga 1200 boots in an FS-UAE window and runs the program. When the program
ends, the window closes and its output is printed in your terminal.

To remember a different ROM or FS-UAE later, run `setup` again:

```sh
clorraine setup --toolchain ~/.local/share/clorraine/toolchains/rc24 \
  --kickstart ~/amiga/roms/amiga-os-310-a1200.rom --fs-uae /usr/bin/fs-uae
```

`--kickstart` and `--fs-uae` on a `run`, or `CLORRAINE_KICKSTART` and
`CLORRAINE_FS_UAE`, override the remembered values for one run.

## How a run is checked

The CLI validates FS-UAE 3.2.0 or newer and a 512 KB Kickstart 3.1 A1200 ROM,
stages a private directory drive, and runs the program through a compiled
wrapper. The wrapper records the return code and writes `ok` only for zero; the
CLI reads both markers before cleaning up. `--capture` writes a successful
program's output to `out.txt`. A failed or timed-out run prints its output,
keeps its private directory, and prints the path.

A run ends after 90 seconds, Amiga boot included
(`CLORRAINE_EMULATOR_TIMEOUT_SECONDS`, 1–3600). The FS-UAE run boots straight
into your program; it does not start Workbench.

Without `--emulator`, a `hosted-amiga` run uses `vamos`, a host-side AmigaOS
runner for console programs. The installer does not provide it.

## Media policy

- **Original Kickstart and Workbench media is user-supplied.** It is never
  downloaded, copied, or committed by the project.
- **FS-UAE's built-in AROS replacement ROM** can be used for manual
  experiments; `clorraine run --emulator` requires a licensed Kickstart 3.1
  A1200 ROM.
- **Kickstart 3.0 is untested.** The A500 (`hosted-amiga-13`) profile builds run
  on the A1200, but that is not a Kickstart 1.3 test.

## Requirements

| Requirement | Why |
|---|---|
| FS-UAE 3.2.0 or newer | Ubuntu 24.04's 3.1.66 package is too old |
| A desktop session | FS-UAE opens a window |
| A licensed Kickstart 3.1 A1200 ROM | The emulated Amiga 1200 boots from it |

## Learn more

- [Quick start](/guide/quick-start)
- [Amiga profiles](/amiga/profiles)
- [Known limitations](/guide/known-limitations)
