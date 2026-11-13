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
| FS-UAE | Graphics, audio, Intuition, timers | Slower | AROS by default; original ROMs optional |

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

## Media policy

- **No ROM is required for `vamos`.**
- **FS-UAE defaults to its built-in AROS replacement ROM.**
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
