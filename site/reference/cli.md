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
```

| Option | Meaning |
|---|---|
| `-p`, `--profile <host\|hosted-amiga\|hosted-amiga-13\|freestanding>` | Target profile; defaults to `host` |
| `--release` | Optimize with `-O2`; strip and section-GC on Linux/Amiga |
| `--size` | Optimize for size with `-Os`; same release processing; cannot combine with `--release` |
| `--emit-c` | Print the generated C after building |
| `--emulator` | Amiga run: use FS-UAE (host handoff) |
| `--capture` | Write program stdout to `out.txt` and print its path |
| `-o FILE` | `emit` only: write C to `FILE` (default stdout) |
| `--doc [-o DIR] FILE...` | Generate Markdown from `///` comments |
| `--explain CODE` | Print the registry entry for a diagnostic code |
| `--diagnostics=json` | Print diagnostics as JSON objects on stderr |
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print the version |

Options that take a value accept both `--profile host` and `--profile=host`.
For `--diagnostics=json`, place the option after the command:
`clorraine build --diagnostics=json -p host file.clo`.

### Examples

```sh
clorraine run hello.clo
clorraine run hello.clo -p hosted-amiga
clorraine build hello.clo -p hosted-amiga-13 --release
clorraine emit hello.clo -p host -o build/hello.c
clorraine --explain E10020
clorraine --doc -o build/api examples/hello/hello.clo
```

### Exit codes

| Code | Meaning |
|---|---|
| `0` | Success, including help, version, and the FS-UAE handoff |
| `1` | Clorraine diagnostic, missing tool, or C compiler failure |
| `2` | Usage error, or the driver is not inside the SDK container |

A `run` exits with the program's own code, so a program that exits 2 is
indistinguishable from a usage or environment error.

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
