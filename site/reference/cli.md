---
title: Command line
description: "The clc, clorraine, and clorraine-lsp command-line surface."
---

# Command line

The workspace builds three binaries:

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
  setup   Validate and store an Amiga toolchain
```

| Option | Meaning |
|---|---|
| `-p`, `--profile <host\|hosted-amiga\|hosted-amiga-13\|freestanding>` | Target profile; defaults to `host` |
| `--release` | Optimize with `-O2`; strip and section-GC on Linux/Amiga |
| `--size` | Optimize for size with `-Os`; same release processing; cannot combine with `--release` |
| `--emit-c` | Print the generated C after building |
| `--emulator` | Amiga run: verified FS-UAE guest run with a native toolchain; the marked SDK container keeps its legacy handoff |
| `--fs-uae PATH` | FS-UAE executable for a native emulator run (`CLORRAINE_FS_UAE`, then `PATH`) |
| `--kickstart PATH` | Kickstart 3.1 A1200 ROM for a native emulator run (`CLORRAINE_KICKSTART`) |
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

`setup --toolchain PATH` validates the upstream rc24 root and stores its
canonical path. `build` and `run` resolve a toolchain in this order:
`--toolchain`, `CLORRAINE_TOOLCHAIN`, the stored config, then the marker-gated
`PATH` fallback inside the pinned SDK container. `--sdk-root` and
`CLORRAINE_SDK` are removed. Host builds do not need a toolchain.

A native `run --emulator` requires FS-UAE 3.2.0 or newer and a 512 KB Kickstart
3.1 A1200 ROM. The timeout defaults to 90 seconds
(`CLORRAINE_EMULATOR_TIMEOUT_SECONDS`, 1–3600). The guest success oracle is the
compiled wrapper: rc 0 plus status `ok`. For a verified guest, `--capture`
writes `out.txt` only on success; a failed guest prints its output and retains
the run. Host, `vamos`, and freestanding runs capture stdout regardless of the
exit code.

### Examples

```sh
clorraine run hello.clo
clorraine setup --toolchain ~/toolchains/m68k-amigaos-gcc-16.2
clorraine run hello.clo -p hosted-amiga
clorraine build hello.clo -p hosted-amiga-13 --release
clorraine run hello.clo -p hosted-amiga --emulator \
  --kickstart ~/amiga/roms/amiga-os-310-a1200.rom
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
definition, completion, and document symbols. Build it with:

```sh
cargo build --workspace
```

Then point an LSP client at `target/debug/clorraine-lsp`. The VS Code extension
in `tools/vscode/` is the reference client.

## Documentation generation

`clorraine --doc` renders `///` comments from Clorraine sources as linked
Markdown:

```sh
clorraine --doc -o build/api examples/hello/hello.clo
```

The output directory defaults to `build/doc`. This is for your sources; the
site's [standard library page](/reference/stdlib) indexes the embedded modules.
