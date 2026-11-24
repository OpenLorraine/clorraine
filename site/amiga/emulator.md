---
title: Emulator workflow
description: "Run Clorraine programs with vamos or FS-UAE, and understand the ROM and media requirements."
---

# Emulator workflow

Clorraine targets real 68K Amiga systems. The development loop runs on a modern
host with two emulators:

| Tool | Best for | Speed | ROM |
|---|---|---|---|
| `vamos` | Console programs, output checks, CI | Fast | None |
| FS-UAE | Graphics, audio, Intuition, timers | Slower | AROS manual-only; a licensed Kickstart 3.1 A1200 ROM for verified Linux CLI runs |

## `vamos`

`vamos` runs AmigaOS CLI binaries on the host. It is the default path for
console programs and for the host-versus-m68k differential checks.

Inside the repository:

```sh
make run FILE=examples/hello/hello.clo PROFILE=hosted-amiga
```

The command builds the program for `hosted-amiga` and runs it under `vamos`.
The output is the same text the program prints on an Amiga.

Use `CAPTURE=1` to write the output to a file instead of streaming it:

```sh
make run FILE=examples/hello/hello.clo PROFILE=hosted-amiga CAPTURE=1
```

## FS-UAE

FS-UAE is used for graphics, audio, Intuition, and the hardware gates. The SDK
provides a wrapper that builds the program and boots the emulator:

```sh
make run FILE=examples/amiga/boing.clo PROFILE=hosted-amiga-13 EMULATOR=1
```

The A500 profile needs a licensed Kickstart 1.3 ROM in `~/amiga/roms`. The
repository's `make init` creates the data folders and suggests values for
detected ROMs.

For a headless capture:

```sh
make run FILE=examples/amiga/boing.clo PROFILE=hosted-amiga-13 EMULATOR=1 CAPTURE=1
```

## Linux CLI verified run

With a native toolchain selected (see `clorraine setup --toolchain PATH`), the
Linux CLI can run a built program as an A1200/KS3.1 guest without Docker:

```sh
clorraine run hello.clo -p hosted-amiga --emulator \
  --toolchain ~/toolchains/m68k-amigaos-gcc-16.2 \
  --kickstart ~/amiga/roms/amiga-os-310-a1200.rom
```

The CLI validates FS-UAE 3.2.0 or newer and a 512 KB Kickstart 3.1 A1200 ROM,
stages a private directory drive, and runs the program through a compiled
wrapper. The wrapper records the return code and writes `ok` only for zero; the
host reads both markers before cleanup. `--capture` writes a successful guest's
output to `out.txt`. A failed or timed-out run prints its output, keeps its
private directory, and prints the path; no `out.txt` is written.

Run the whole local matrix with:

```sh
make fs-uae-check KICKSTART=~/amiga/roms/amiga-os-310-a1200.rom \
  TOOLCHAIN=~/toolchains/m68k-amigaos-gcc-16.2
```

## Media policy

- **No ROM is required for `vamos`.**
- **FS-UAE can use its built-in AROS replacement ROM for manual experiments;** the verified Linux CLI runner requires a licensed Kickstart 3.1 A1200 ROM.
- **Original Kickstart and Workbench media is user-supplied.** It is never
  downloaded, copied, or committed by the project.
- **The advanced Workbench gates** need licensed media and are not part of a
  bare checkout's fast path.

## Requirements

| Requirement | Why |
|---|---|
| Docker | The pinned SDK image and toolchain |
| X display | FS-UAE needs a display; the SDK wraps it with Xvfb for headless gates |
| `~/amiga/roms` | Only for FS-UAE with original ROMs |
| `~/amiga/adf`, `~/amiga/hdf` | Only for licensed Workbench media |

If the emulator cannot open a display, see the X11 troubleshooting section in
[docker/README.md](https://github.com/OpenLorraine/clorraine/blob/main/docker/README.md#x11-troubleshooting).

## The full acceptance gate

`make verify` runs the host checks plus the Amiga legs: SDK builds, emulator
runs, native captures, differential checks, and runtime-fault programs. It needs
the SDK X display and the licensed media for the advanced gates.

For a quick check without the Amiga legs, use `make check`.

## Learn more

- [Quick start](/guide/quick-start)
- [Amiga profiles](/amiga/profiles)
- [Verification](/project/verification)
- [Docker SDK README](https://github.com/OpenLorraine/clorraine/blob/main/docker/README.md)
