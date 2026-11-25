# Linux CLI (v1.2.0 developer preview)

Clorraine v1.2.0 is a public **developer preview** for **Linux x86_64**. One
command installs the CLI, the Amiga toolchain, the VS Code extension, and a
starter project. Windows and macOS are on the way.

## Install

```sh
curl -fsSL https://github.com/OpenLorraine/clorraine/releases/download/v1.2.0/install.sh -o clorraine-install.sh && sh clorraine-install.sh
```

The installer walks through five steps:

1. **Install the CLI.** It verifies the release archive against its pinned
   SHA-256 and installs `clc`, `clorraine`, and `clorraine-lsp` under
   `~/.local` (or another prefix), with an optional `PATH` entry in
   `~/.profile`.
2. **Amiga toolchain.** It uses an existing AmigaPorts rc24 root or downloads
   the pinned upstream archive, checks its size and SHA-256, and extracts it.
3. **Amiga emulator.** It finds FS-UAE and a licensed Kickstart 3.1 A1200 ROM,
   then runs `clorraine setup` once to validate the toolchain and remember all
   three paths.
4. **Say hello.** On request, it runs the hello program on the PC and plays the
   demo-scene hello on an emulated Amiga 1200 in FS-UAE.
5. **Your first Amiga project.** It installs the VS Code extension from the same
   release and offers a starter project in the current folder, which it can
   open in VS Code.

To check the installer before running it, download `install.sh` and
`SHA256SUMS` from the
[v1.2.0 release](https://github.com/OpenLorraine/clorraine/releases/tag/v1.2.0)
and run `sha256sum -c SHA256SUMS --ignore-missing` first. A literal
`curl … | sh` is not supported, because the installer asks questions on the
terminal.

## Requirements

| | Required | Notes |
|---|---|---|
| Linux x86_64, glibc 2.38+ | Yes | Ubuntu 24.04 is the minimum baseline; Ubuntu 22.04 cannot run the rc24 host tools |
| A C compiler (`cc`) | Yes | For the hello program on the PC |
| FS-UAE 3.2.0+ | For Amiga runs | Ubuntu 24.04's `fs-uae` package is 3.1.66, which is too old |
| A licensed 512 KB Kickstart 3.1 A1200 ROM | For Amiga runs | Never bundled or downloaded; KS3.0 is untested |
| VS Code | Optional | The extension is installed when VS Code is detected |

The toolchain pin is AmigaPorts `m68k-amigaos-gcc` v16.2-rc24
(`m68k-amigaos-gcc-16.2-rc24-x86_64-linux.tar.xz`, 69,868,548 bytes, SHA-256
`b93b070ac47d2f01d126f57c03a8405f71fb470c8397b0bda51041217d25c100`). The
installer downloads it from AmigaPorts; Clorraine does not mirror or
redistribute it. `setup` rejects other versions.

## What goes where

| Path | Contents |
|---|---|
| `~/.local/bin/` | `clc`, `clorraine`, `clorraine-lsp` (links into the active version) |
| `~/.local/share/clorraine/versions/` | Installed versions; `current` points at the active one |
| `~/.local/share/clorraine/examples/` | `hello.clo` and the Amiga intro `hello-amiga.clo` |
| `~/.local/share/clorraine/toolchains/rc24/` | The downloaded toolchain (`$XDG_DATA_HOME` when set) |
| `~/.config/clorraine/config` | The remembered toolchain, Kickstart, and FS-UAE paths (`$XDG_CONFIG_HOME` when set) |

## Everyday commands

```sh
clorraine new my-game                              # a starter project with VS Code tasks
clorraine run hello.clo                            # build and run on the PC
clorraine run -p hosted-amiga --emulator hello.clo # build and run on the emulated A1200
clorraine build -p hosted-amiga hello.clo          # build an Amiga executable
```

`clorraine new [DIR]` writes `main.clo`, a `README.md`, a `.gitignore`, and a
`.vscode` folder (the Amiga profile, the extension recommendation, and a
**Run in FS-UAE** default build task). It never writes into a folder that is not
empty; the default folder is `amiga-starter`.

## Configuration

`setup` validates and stores the toolchain, and optionally the emulator:

```sh
clorraine setup --toolchain ~/.local/share/clorraine/toolchains/rc24 \
  --kickstart ~/amiga/roms/amiga-os-310-a1200.rom --fs-uae /usr/bin/fs-uae
```

Each value is checked before the config is replaced. A later `setup` that does
not name the Kickstart or FS-UAE keeps the remembered values. The values are
resolved in this order:

| Input | Order |
|---|---|
| Toolchain | `--toolchain`, `CLORRAINE_TOOLCHAIN`, the config |
| Kickstart ROM | `--kickstart`, `CLORRAINE_KICKSTART`, the config |
| FS-UAE | `--fs-uae`, `CLORRAINE_FS_UAE`, the config, then `PATH` |

A remembered path that no longer works is reported with the config file and the
`setup` command that changes it.

## Emulator runs

`run --emulator` boots an Amiga 1200 with Kickstart 3.1 in FS-UAE from a private
directory drive and runs the program through a compiled wrapper. The wrapper
records the return code and writes a status marker only for zero; the CLI reads
both before cleaning up. `--capture` writes a successful program's output to
`out.txt`. A failed or timed-out run prints its output and keeps its private
directory.

- A run ends after 90 seconds, Amiga boot included
  (`CLORRAINE_EMULATOR_TIMEOUT_SECONDS`, 1–3600).
- The run boots straight into the program, without Workbench.
- The A500 (`hosted-amiga-13`) profile builds run on the A1200, but that is not
  a Kickstart 1.3 test.
- FS-UAE's built-in AROS can boot manually, but it is not a supported path and
  the CLI does not launch it.

## VS Code

The installer installs the `openlorraine.clorraine` extension from the release
when VS Code (`code`, `codium`, or `code-insiders`) is found. It adds
diagnostics, hover, go to definition, completion, document symbols, and
**Run in FS-UAE** and **Run** buttons in the editor title bar for `.clo` files.
It finds the CLI and the language server in `~/.local/bin`.

## Scripted installs and removal

Without a terminal the installer asks nothing and keeps its defaults. The
options are listed by `sh clorraine-install.sh --help`, among them `--prefix`,
`--modify-path`, `--toolchain`, `--no-toolchain`, `--fs-uae`, `--kickstart`,
`--no-emulator`, `--no-hello`, `--starter DIR`, `--no-starter`, `--vscode`, and
`--no-vscode`.

`sh clorraine-install.sh --uninstall` removes the active version's links,
`current`, and the `PATH` block; `--purge` also removes the installed versions
and the CLI config.

## Limits

- Linux x86_64 only. Windows and macOS are on the way; WSL2, Remote-WSL, and
  ARM hosts are not claimed.
- The VS Code extension is not in a marketplace yet.
- The full list is in [known limitations](known-limitations.md).

## Related

- [Quick start on the website](https://openlorraine.github.io/clorraine/guide/quick-start)
- [Known limitations](known-limitations.md)
- [v1.2.0 release](https://github.com/OpenLorraine/clorraine/releases/tag/v1.2.0)
