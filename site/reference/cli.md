---
title: Command line
description: "The clc, clorraine, and clorraine-lsp command-line surface."
---

# Command line

The installers put three binaries on your `PATH`: in `~/.local/bin` on Linux and
in `%LOCALAPPDATA%\Programs\Clorraine\current\bin` on Windows (`.exe`), or under
the prefix you choose:

| Binary | Purpose |
|---|---|
| `clc` | Compile `.clo` sources to one C99 translation unit |
| `clorraine` | Build, run, emit, document, and explain diagnostics |
| `clorraine-lsp` | Stdio language server for editors |

## `clorraine`

```text
clorraine: build and run Clorraine programs

Usage:
  clorraine <command> [options] FILE...

Commands:
  run     Build and run a program
  build   Build a program without running it
  emit    Print the generated C (or write it with -o)
  setup   Validate and store an Amiga toolchain (and the FS-UAE and Kickstart paths)
  new     Create a starter Amiga project with VS Code tasks (default folder: amiga-starter)
```

| Option | Meaning |
|---|---|
| `-p`, `--profile <host\|hosted-amiga\|hosted-amiga-13\|freestanding>` | Target profile; defaults to `host` |
| `--release` | Optimize with `-O2`; strip and section-GC on Linux/Amiga |
| `--size` | Optimize for size with `-Os`; same release processing; cannot combine with `--release` |
| `--emit-c` | Print the generated C after building |
| `--emulator` | Amiga run: verified FS-UAE guest run with a native toolchain; the marked SDK container keeps its legacy handoff |
| `--fs-uae PATH` | FS-UAE executable for an emulator run or `setup` (then `CLORRAINE_FS_UAE`, the path stored by `setup`, then `PATH`) |
| `--kickstart PATH` | Kickstart 3.1 A1200 ROM for an emulator run or `setup` (then `CLORRAINE_KICKSTART`, then the path stored by `setup`) |
| `--toolchain PATH` | Toolchain root for `setup`, non-host `build`, and non-host `run` |
| `--capture` | Write program stdout to `out.txt` and print its path (verified guest: success only) |
| `-o FILE` | `emit` only: write C to `FILE` (default stdout) |
| `--doc [-o DIR] FILE...` | Generate Markdown from `///` comments |
| `--explain CODE` | Print the registry entry for a diagnostic code |
| `--diagnostics=json` | Print diagnostics as JSON objects on stderr |
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print the version |

Options that take a value accept both `--profile host` and `--profile=host`.
For `--diagnostics=json`, place the option after the command:
`clorraine build --diagnostics=json -p host file.clo`.

The installer runs `setup` for you. `setup --toolchain PATH` validates the
upstream rc24 root and stores its canonical path; with `--kickstart PATH` and
`--fs-uae PATH` it also checks and remembers the emulator inputs, so a later
`run --emulator` needs no extra options. A later `setup` that does not name them
keeps the remembered values. `build` and `run` resolve a toolchain in this
order: `--toolchain`, `CLORRAINE_TOOLCHAIN`, then the stored config. Host builds
do not need a toolchain. The config is `$XDG_CONFIG_HOME/clorraine/config` when
that is absolute, otherwise `~/.config/clorraine/config`.

`new [DIR]` creates a starter Amiga project in `DIR` (default `amiga-starter`):
`main.clo`, a `README.md`, and a `.vscode` folder with the Amiga profile, the
extension recommendation, and a **Run in FS-UAE** default build task. It never
writes into a folder that is not empty.

An Amiga `run` on your machine uses `--emulator`: it requires FS-UAE 3.2.0 or
newer and a 512 KB Kickstart 3.1 A1200 ROM. Without `--emulator`, a
`hosted-amiga` run uses `vamos`, which the installer does not provide. The timeout defaults to 90 seconds
(`CLORRAINE_EMULATOR_TIMEOUT_SECONDS`, 1–3600). The guest success oracle is the
compiled wrapper: rc 0 plus status `ok`. For a verified guest, `--capture`
writes `out.txt` only on success; a failed guest prints its output and retains
the run. Host, `vamos`, and freestanding runs capture stdout regardless of the
exit code.

### Examples

```sh
clorraine run hello.clo
clorraine new my-game
clorraine run hello.clo -p hosted-amiga --emulator
clorraine build hello.clo -p hosted-amiga-13 --release
clorraine setup --toolchain ~/.local/share/clorraine/toolchains/rc24 \
  --kickstart ~/amiga/roms/amiga-os-310-a1200.rom --fs-uae /usr/bin/fs-uae
clorraine emit hello.clo -p host -o build/hello.c
clorraine --explain E10020
clorraine --doc -o build/api examples/hello/hello.clo
```

### Exit codes

| Code | Meaning |
|---|---|
| `0` | Success, including help, version, a verified guest run, and the legacy FS-UAE handoff |
| `1` | Clorraine diagnostic, missing tool, failed toolchain probe, C compiler failure, or guest failure |
| `2` | Usage error, invalid toolchain, or invalid emulator, ROM, or timeout selection |

Host, `vamos`, and freestanding runs exit with the program's own code. A
verified guest failure exits 1 for every nonzero guest code, so a guest
`return 2` is not reported as 2.

## `clc`

`clc` is the compiler front end and emitter. It requires a profile and writes C:

```sh
clc --profile host -o build/hello.c hello.clo
clc --profile hosted-amiga -o build/hello.c hello.clo
```

Useful flags:

| Flag | Meaning |
|---|---|
| `--profile <name>` | Required target profile |
| `-o FILE` | Output C file |
| `--version` | Print the compiler version |
| `--diagnostics=json` | JSON diagnostic stream |
| `--explain CODE` | Print a diagnostic registry entry |
| `--max-errors N` | Cap the reported errors |
| `--deny-warnings` | Promote warnings to failure |
| `--parse-only` | Stop after parsing |

`clc` does not have a `--help` mode; `clorraine --help` documents the
user-facing driver.

## `clorraine-lsp`

The language server speaks LSP over stdio. It provides diagnostics, hover, go to
definition, completion, and document symbols. The installers install it as
`~/.local/bin/clorraine-lsp` on Linux and
`%LOCALAPPDATA%\Programs\Clorraine\current\bin\clorraine-lsp.exe` on Windows;
point an LSP client at that path. The VS Code
extension, which the installer also installs, is the reference client and finds
it automatically.

## Documentation generation

`clorraine --doc` renders `///` comments from Clorraine sources as linked
Markdown:

```sh
clorraine --doc -o build/api examples/hello/hello.clo
```

The output directory defaults to `build/doc`. This is for your sources; the
site's [standard library page](/reference/stdlib) indexes the embedded modules.
