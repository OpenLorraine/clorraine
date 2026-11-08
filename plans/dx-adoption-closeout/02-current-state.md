# Current State: DX and Adoption Closeout

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

**Compiler and driver.** `clc` is one library with two thin binaries.
`compiler/src/driver.rs:330-410` owns the complete front-end sequence:
read files, parse, profile gate, embedded-stdlib injection to a fixpoint,
load, resolve, type check, ownership check, emit, diagnostics, output. The
driver has no reusable entry point that accepts in-memory sources, so an editor
server cannot reuse it without either spawning `clc` or duplicating the
sequence.

**Loader.** `compiler/src/load/mod.rs:150-205` merges files that declare the
same dotted module and `:476-510` requires exactly one `func main(): i32`.
There is no library mode.

**Diagnostics.** `compiler/src/diag.rs` carries code, severity, message, span,
and help. `SourceMap` (`compiler/src/span.rs:69-128`) resolves byte offsets to
one-based line and character columns. There is no UTF-16 mapping and no LSP
range conversion.

**CLI.** `compiler/src/clorraine/cli.rs:112-150` treats `--explain`, `--help`,
and `--version` as early-exit modes. Every other argument goes through
`parse_options`, which requires a command word (`run`, `build`, `emit`).
`--doc` has no path.

**Grammar and demo.** `editors/clorraine.tmLanguage.json` is the canonical
TextMate grammar; `tools/demo` consumes it directly. There is no VS Code
extension and no language client.

**SDK setup.** `docker/scripts/init-env.sh` creates `docker/.env`, detects X11,
and scans `$HOME/amiga` for ROMs. `docker/Makefile` exposes `image`, `init`,
`check-image`, `exec`, `smoke`, and `provenance`. There is no single installer
that reports Docker, image, toolchain, NDK, and media state together.

**Documentation comments.** `spec/01-lexical.md:125` defines `///` as a
documentation comment attached to the next declaration. `compiler/src/lex/mod.rs:140`
skips it as trivia. Nothing reads it.

**Node tooling.** `tools/demo` is an npm-only package with TypeScript and a
`node --test` suite (`Makefile:304-306`). `make demo-check` is a host-only gate.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/driver.rs` | Front-end orchestration | Call the shared analysis entry point |
| `compiler/src/analysis.rs` | — | New shared parse/check entry point |
| `compiler/src/load/mod.rs` | Module merge and entry-point check | Add a library-mode option |
| `compiler/src/lsp/` | — | New JSON, framing, URI, text, workspace, server, features |
| `compiler/src/lsp_main.rs` | — | New `clorraine-lsp` binary |
| `compiler/Cargo.toml` | Two binaries | Add the server binary, no dependency |
| `compiler/src/clorraine/cli.rs` | Driver CLI | Add `--doc` |
| `compiler/src/clorraine/docgen.rs` | — | New Markdown generator |
| `tools/setup/install_sdk.py` | — | New installer |
| `tools/setup/tests/` | — | New unit tests |
| `tools/vscode/` | — | New extension package |
| `docs/c-to-clorraine.md`, `docs/README.md` | — | New guide and index |
| `Makefile`, `.github/workflows/check.yml` | Gates | Add setup, vscode, installer CI |
| `AGENTS.md`, `README.md` | Guidance | Document the new surfaces |

### Code Analysis

The driver's per-file parse rule is important: it keeps a parsed program only
when that file added no parse error, so later stages never cascade from a
broken file. The shared entry point must preserve this rule.

The loader's merge rule means the LSP analysis set must include every file that
declares a module in the import closure, not just the first file per module.

The driver's profile gate rejects an embedded stdlib module the profile does
not provide (`driver.rs:260-310`). The LSP must expose the profile or the
default will reject Amiga modules.

`SourceMap::location` counts characters, not UTF-16 code units. Every LSP
range must be converted explicitly; using `Location.column` directly would be
wrong for non-BMP characters.

## Gaps Identified

### Gap 1: No reusable front-end entry point

**Current Behavior:** only `driver::run(args)` exists; it reads paths from
disk and writes diagnostics to stderr.
**Required Behavior:** a library call accepts `(path, text)` pairs, a profile,
and an entry-point mode, and returns sources, diagnostics, and analysis.
**Fix Required:** `compiler/src/analysis.rs` plus a loader option; the driver
calls it and keeps its CLI behavior.

### Gap 2: No protocol layer

**Current Behavior:** no JSON value model, no framing, no URI handling, no
cancellation.
**Required Behavior:** std-only modules with unit tests and a bounded server
loop.
**Fix Required:** `compiler/src/lsp/{json,protocol,uri,text,workspace,server,features}.rs`.

### Gap 3: No installer

**Current Behavior:** `make init` writes `.env`; nothing reports the whole
toolchain or fetches the image.
**Required Behavior:** a single report and a pinned fetch path.
**Fix Required:** `tools/setup/install_sdk.py` and `make setup`.

### Gap 4: No documentation generator

**Current Behavior:** `///` is discarded.
**Required Behavior:** deterministic linked Markdown.
**Fix Required:** `docgen.rs` and a `--doc` CLI action.

### Gap 5: No editor package or guide

**Current Behavior:** canonical grammar only.
**Required Behavior:** a `.vsix` that launches the server, and a Rosetta guide.
**Fix Required:** `tools/vscode/` and `docs/c-to-clorraine.md`.

## Dependencies

### Internal Dependencies

- The parser, loader, resolver, sema, and ownership stages are the LSP and
  documentation analysis source.
- `editors/clorraine.tmLanguage.json` is the extension's only grammar.
- `docker/Makefile` and `docker/versions.env` are the installer's fetch and
  provenance source.
- The Makefile owns the gate names.

### External Dependencies

- Docker and Docker Compose for the installer's fetch path; detection degrades
  cleanly when absent.
- Node.js 22+ and npm for `tools/vscode` and `tools/demo`; host-only.
- `@vscode/vsce` as a development npm package.
- No Rust crate.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Driver refactor changes diagnostics | Medium | High | Shared entry point owns the exact sequence; existing suites and `make differential` are the oracle |
| LSP workspace model merges unrelated programs | High if naive | High | Module index plus import closure, never whole-tree analysis |
| JSON parser accepts malformed input | Medium | High | Unit cases for escapes, surrogates, depth, truncated frames, and size caps |
| Server hangs on a bad client | Medium | High | Reader thread, bounded frames, per-request test timeout, cancellation set |
| Installer touches licensed media | Low | Critical | Read-only metadata scan; no copy, move, or download; tests assert no media bytes are read |
| `.vsix` packages stale grammar | Medium | Medium | Package build copies from `editors/`; byte comparison test |
| Doc output path traversal | Low | High | Module-name allowlist and canonical output path check |
| Node not on `PATH` | High | Low | `make vscode-check` reports the missing tool clearly; `make check` stays Node-free |
