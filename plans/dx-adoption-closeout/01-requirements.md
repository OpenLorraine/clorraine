# Requirements: DX and Adoption Closeout

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Implements**: R1-R14
> **Decision authority**: the goal text rules the scope and the seven user
> answers recorded in the [Ambiguity Register](00-ambiguity-register.md);
> eligible technical mechanisms are delegated under auto-design and recorded
> there.

## Feature Overview

The compiler, driver, and frozen specification are complete. What remains for
adoption is the developer path around them: getting the SDK installed, editing
Clorraine with real language feedback, generating reference documentation from
source comments, and helping a C programmer translate familiar Amiga patterns
into Clorraine.

The closeout has five independently shippable slices:

1. **SDK installer** — one entry point that detects the host, Docker image,
   toolchain, NDK, and licensed media, reports exactly what is missing, and
   fetches only the public, pinned components.
2. **Language server** — a stdio JSON-RPC server that reuses the compiler front
   end and provides diagnostics, hover, goto definition, completion, and
   document symbols. It stays in the std-only Rust workspace.
3. **VS Code extension** — TypeScript packaging that launches the server,
   registers the canonical TextMate grammar, and produces a `.vsix` exercised
   by a headless test.
4. **Documentation generator** — `clorraine --doc` turns `///` comments plus
   declarations into deterministic linked Markdown without running the C
   toolchain.
5. **C-to-Clorraine guide** — side-by-side examples for the common Amiga
   patterns and the diagnostics a C programmer will meet.

Nothing here changes the language, grammar, diagnostics registry, ABI, runtime
shims, standard library, or compiler performance.

## Functional Requirements

### R1 — Installer detection

**R1.1 Entry point.** `make setup` runs a Python 3 standard-library installer
from `tools/setup/`. It works from the repository root and from any working
directory by locating the repository from its own path.

**R1.2 Components.** The installer detects and reports, per component:

| Component | Detection |
| --------- | --------- |
| Docker CLI and daemon | `docker version` succeeds |
| Docker Compose | `docker compose version` succeeds |
| SDK image | `docker image inspect <IMAGE>` succeeds |
| SDK environment file | `docker/.env` exists and names an existing media root |
| m68k toolchain | `m68k-amigaos-gcc`, `m68k-amigaos-gdb`, `vc`, `vamos` inside the image |
| NDK | the NDK header tree inside the image is present and non-empty |
| Host C compiler | `cc` or `gcc` on `PATH` (host profile) |
| Kickstart ROMs | at least one `*.rom` under the media root |
| Workbench/floppy media | `*.adf` and `*.hdf` under the media root |

**R1.3 Report.** The default output is a human table with one row per
component: name, state (`ok`, `missing`, `optional`), detail, and a recovery
hint. `--json` prints one machine-readable object with the same data and
stable keys. Detection never prints file contents, ROM bytes, or media bytes.

**R1.4 Media policy.** Licensed media is reported, never copied, moved,
uploaded, or downloaded. A missing ROM or Workbench image is `optional` for a
plain install and becomes `missing` only when the user passes
`--require-media`.

### R2 — Installer fetch, integrity, and safety

**R2.1 Fetch scope.** `--fetch` prepares the missing public components by
running the existing pinned image build (`make -C docker image`) and, when
absent, `make -C docker init`. The image build is the only fetch path: it
clones the pinned `amiga-gcc` revision and lets that build fetch its own
public submodules and NDK under its own SHA-256 checks. The installer never
downloads a licensed artifact.

**R2.2 Fail-closed integrity.** The installer performs no download of its own.
The delegated image build is the only downloader, and its pinned upstream
inputs are SHA-256-verified where upstream publishes checksums; the installer
records that boundary in its report instead of implying it verified bytes it
never saw. After a fetch, the installer verifies that:

- the image exists;
- `/opt/amiga/BUILD-PROVENANCE.txt` records exactly the `AMIGA_GCC_COMMIT`
  from `docker/versions.env`;
- the NDK header tree is present and non-empty;
- each required tool reports a version.

A provenance mismatch, missing NDK tree, missing tool, or failed build exits
non-zero with the failing step and a recovery command. The installer never
continues past a failed verification. A test proves the provenance mismatch is
rejected even when every other check passes.

**R2.3 Prompts and dry run.** `--dry-run` prints the detection report and the
exact command plan without changing anything. A real fetch asks for
confirmation on a terminal; `--yes` is required in a non-interactive run and
its absence is a usage error, never an implicit yes.

**R2.4 No privilege or destructive action.** The installer does not run
`sudo`, does not delete an existing image, does not overwrite an existing
`docker/.env`, and does not modify media. `--force` is not part of the
installer surface.

**R2.5 Recovery errors.** Every failure names the component, the command that
failed, and one concrete recovery step. Errors go to stderr; the exit code is
`2` for usage errors and `1` for an unmet requirement or failed verification.

### R3 — Installer CLI, tests, and integration

**R3.1 Isolated roots.** The installer accepts `--root DIR` (repository root)
and `--amiga-data DIR` (media root). Tests use a temporary root and a temporary
media directory and never read or write `~/amiga` or the real `docker/.env`.

**R3.2 Tests.** Python `unittest` cases cover:

- all components present;
- each component missing, one per case;
- dry run makes no command call;
- non-interactive fetch without `--yes` is a usage error;
- a provenance mismatch fails closed;
- a failed image build surfaces the recovery message;
- media discovery never opens or copies media files;
- `--require-media` changes only the exit status and the ROM/media rows.

A fake `docker` executable on the test `PATH` and a fake command runner
injected by unit-test monkeypatching (no production environment variable or
hidden flag) exercise both the success and failure paths, so no test needs a
Docker daemon. The installer's command runner is a single function; every
external call goes through it.

**R3.3 Compatibility.** `make init`, `make -C docker init`, the Docker compose
workflow, and all existing `docker/scripts/*` continue unchanged. `make setup`
is additive.

### R4 — Language server transport and lifecycle

**R4.1 Executable.** A new std-only binary `clorraine-lsp` speaks LSP over
stdio. It never writes protocol data to stdout outside a framed response and
never writes to stderr except for a fatal startup error.

**R4.2 Framing.** Messages are UTF-8 JSON framed by `Content-Length` headers
with `\r\n` line endings. Header names are matched case-insensitively. Unknown
headers are ignored. A missing or malformed length is a protocol error, not a
crash.

**R4.3 Lifecycle.** The server implements:

| Message | Behavior |
| ------- | -------- |
| `initialize` | returns `capabilities` and `serverInfo`; negotiates position encoding |
| `initialized` | accepted, no response |
| `shutdown` | returns `null`, marks shutdown requested |
| `exit` | exits `0` after shutdown and `1` without it |
| `$/cancelRequest` | records the id for cancellation |
| unknown request | JSON-RPC error `-32601` |
| unknown notification | ignored |

**R4.4 Capabilities.** The server advertises full-document synchronization,
hover, definition, completion (with `.` and `:` triggers), and document
symbols. It advertises UTF-16 positions, the LSP default.

**R4.5 Error mapping.** Malformed JSON is `-32700` with `id: null`; an invalid
request object is `-32600`; missing or invalid parameters are `-32602`; an
internal failure is `-32603` with a short message. Every error is one response
and the loop continues.

### R5 — LSP diagnostics

**R5.1 Source.** Diagnostics come from the in-process compiler front end, not
from a subprocess and not from a re-implemented checker.

**R5.2 Publication.** Opening, changing, or saving a document publishes
`textDocument/publishDiagnostics` for that document. Closing publishes an
empty list. Each diagnostic carries its range, severity (`1` error, `2`
warning), code, message, and `source: "clorraine"`.

**R5.3 Scope.** Only diagnostics whose primary span belongs to the published
document are sent. Diagnostics in embedded standard-library buffers are never
published to a user document.

**R5.4 Empty and broken inputs.** A file with no errors publishes an empty
list. A syntactically invalid file publishes its syntax errors and still
serves document symbols from the partial tree. A missing on-disk file is
reported as empty diagnostics, not as a server failure.

### R6 — LSP language features

**R6.1 Hover.** Hovering an identifier returns a Markdown code block with the
declaration's kind and exact source spelling (signature for functions, full
declaration head for types, `let`/`const`/parameter for locals) plus its
module, or `null` when the position names nothing.

**R6.2 Goto definition.** Go-to-definition on a local, parameter, module-level
declaration, import binding, qualified module segment, or type name returns the
declaration's file URI and identifier range. Cross-file definitions work for
workspace modules. An unresolved name returns `null`.

**R6.3 Completion.** Completion returns a sorted, duplicate-free list of
keywords, built-in scalar and container types, the current module's
declarations, the current file's imports, and workspace module names. Each item
has a label, kind, and one-line detail. The list is stable across runs.

**R6.4 Document symbols.** Document symbols return every top-level function,
struct, enum, handle, `ffi` block, constant, and global with its name, LSP
symbol kind, full range, and identifier range. Order is source order.

**R6.5 Exactness.** Every feature preserves source identifiers exactly,
including generics, `take`/`borrow` modes, and dotted names. No feature
evaluates code, invokes a C compiler, or requires an SDK toolchain.

**R6.6 Profile.** The client selects the analysis profile through
`initializationOptions.profile` or the extension setting `clorraine.profile`.
The server accepts the four existing profile names. The default is
`hosted-amiga`, because the project's primary target is an Amiga program;
`host` remains selectable for host-only work. An unknown profile fails the
`initialize` request with `-32602` and a message naming the accepted values.
The selected profile affects module availability and ABI-shaped type checking
only; it never invokes a compiler.

### R7 — LSP workspace model, buffers, and cancellation

**R7.1 Workspace index and analysis set.** `initialize` accepts a `rootUri`
(or `rootPath`) and indexes `*.clo` files under that tree by their declared
module name. The walk skips `.git`, `build`, `target`, `node_modules`, and
hidden directories, is bounded (2000 files), and is deterministic
(lexicographic). A file that fails to parse is still indexed by path when its
module declaration is readable; otherwise it is skipped with no user-facing
error.

A request analyzes the requested document plus the transitive closure of its
imports and qualified module references, using the index and open buffers. It
does **not** analyze every file in the workspace as one program, because a
workspace commonly holds independent example programs whose module names or
`main` functions would collide. When a referenced module is not indexed, the
compiler's normal missing-module diagnostic is published.

**R7.2 Unsaved buffers.** `didOpen` and full-text `didChange` override the
on-disk content for every later request. The override survives until
`didClose`, which restores the disk content. No temporary file is written and
no editor buffer is persisted. A new unsaved `untitled:` document is accepted
as an overlay-only document: diagnostics, symbols, hover, and completion work
on it; goto to another file works when the target is in the index.

**R7.3 Robustness.** Malformed JSON, a request for a missing document, an
invalid position, an empty document, a file without a module declaration, and
a request after shutdown are all handled without panic, hang, or process exit
(other than the specified `exit`).

**R7.4 Cancellation.** A cancelled request that has not started is answered
with `-32800`. A request already running observes cancellation at analysis
checkpoints and answers `-32800`. Cancellation of an unknown id is a no-op. The
reader keeps accepting messages while a request runs, so `shutdown` and
`cancel` cannot deadlock behind analysis.

### R8 — Compiler reuse and std-only construction

**R8.1 No new Rust dependencies.** `compiler/Cargo.toml` gains no dependency
and `Cargo.lock` gains no package. The JSON value model, JSON parser and
writer, framing, and URI handling are in-tree modules.

**R8.2 No front-end duplication.** The server and the documentation generator
call the existing lexer, parser, loader, resolver, type checker, and ownership
checker. The pipeline orchestration is a shared, documented library entry point
used by both `clc` and the server.

**R8.3 Library mode.** The shared entry point supports a library analysis that
does not require a `func main(): i32`, so a module workspace can be checked
without a false entry-point diagnostic. The executable path keeps exactly its
current behavior.

**R8.4 No semantic change.** Refactoring the orchestration must leave every
existing compiler diagnostic, emitted C byte, exit code, and CLI contract
unchanged; the existing suites are the oracle.

### R9 — VS Code extension packaging

**R9.1 Location and conventions.** The extension is TypeScript under
`tools/vscode/`, an npm-only package like `tools/demo/`. Its runtime code has
no npm dependency; `typescript`, `@types/node`, `@types/vscode`, and
`@vscode/vsce` are development dependencies.

**R9.2 Manifest.** `package.json` declares the `clorraine` language for `.clo`,
an `onLanguage:clorraine` activation event, the server launch configuration,
and a grammar contribution at `./syntaxes/clorraine.tmLanguage.json`.

**R9.3 Canonical grammar.** The packaged grammar is a byte-identical copy of
`editors/clorraine.tmLanguage.json`, produced by the package build rather than
edited in place. A test compares the bytes and fails on drift.

**R9.4 Packaging.** `make vscode-package` runs the TypeScript build and
`@vscode/vsce package`, producing `build/vscode/clorraine-<version>.vsix`. The
`.vsix` is never committed. The archive contains the compiled client, the
grammar, the manifest, and the README.

**R9.5 Launch contract.** The client starts `clorraine-lsp` over stdio. The
server path resolves from the `clorraine.lsp.path` setting, then the
`CLORRAINE_LSP` environment variable, then the workspace `target/debug` and
`target/release` locations, then `PATH`. The client sends the configured
`clorraine.profile` and completes the `initialize`/`initialized` handshake. It
forwards diagnostics to the VS Code diagnostic collection and opens the
document on definition.

### R10 — VS Code client and headless smoke

**R10.1 Client.** The in-repo client implements framing, request/response
correlation, notifications, server shutdown, and restart-on-crash. It exposes
typed callbacks for diagnostics and a small request API for the extension.

**R10.2 Headless test.** `node --test` runs without a desktop session and
proves: the packaged grammar equals the canonical file; the manifest points at
every shipped file; the packaged client starts the real `clorraine-lsp` from
`CLORRAINE_LSP`, initializes, receives diagnostics for a fixture with an error,
and receives a hover result.

**R10.3 Command.** `make vscode-check` builds the LSP binary, runs type-check,
tests, packaging, and a post-package archive check. It requires Node.js 22+
and network access only for the first `npm ci`; the archive check itself is
offline.

### R11 — `--doc` extraction and Markdown contract

**R11.1 Surface.** `clorraine --doc [-o DIR] FILE...` is an early-exit mode like
`--explain`. Without `-o` it writes the documentation tree under `build/doc`
and prints that path. It needs no `--profile`, no SDK, and no container.

**R11.2 Input.** It parses the listed files with the compiler parser and reads
`///` comment runs from the source text. A run attaches to the next top-level
declaration with no blank line between the run and the declaration. A `///`
run before the `module` declaration documents the module. A run not followed by
a declaration is ignored.

**R11.3 Output.** The output directory contains `index.md` and one
`<dotted.module>.md` file per module. Each module page has:

- an H1 with the module name;
- the module documentation;
- one section per declaration in source order with the exact identifier as
  the heading anchor;
- a fenced `clorraine` code block containing the signature exactly as written
  in the source, with surrounding whitespace trimmed and the body omitted;
- the documentation paragraphs with their original text, normalized only for
  leading `///` and one optional space.

`index.md` lists modules in lexicographic order with their first documentation
line.

**R11.4 Links.** A type or function declared in an input module is linked to
its anchor. A dotted module name in prose is plain code, not a link. No link
points outside the generated directory. Identifiers and signatures are never
rewritten, shortened, or re-spelled.

**R11.5 Guardrails.** The generator never evaluates code, never reads outside
the listed files, never writes outside `-o`, and never requires the SDK or a C
compiler. A source with syntax errors writes nothing and exits non-zero with
the normal diagnostics.

### R12 — `--doc` determinism, fixtures, and tests

**R12.1 Determinism.** Two runs over the same inputs produce byte-identical
files. Output contains no timestamps, absolute paths, random ordering, or
environment-dependent text. Files use LF line endings.

**R12.2 Fixture.** A fixture workspace has at least two modules: a library
module with generics, a struct, an enum, a handle, a constant, and a function
with a multi-line signature and `///` documentation; and a consumer module that
imports and uses the library.

**R12.3 Golden.** A committed golden directory holds the expected Markdown.
The specification test runs the built `clorraine --doc` over a copy of the
fixture in a temporary directory and compares every file byte-for-byte. An
implementation test covers the comment-attachment rule, blank-line separation,
and the ignored orphan comment.

### R13 — C-to-Clorraine Rosetta guide and wrapper cookbook

**R13.1 Location.** The guide is `docs/c-to-clorraine.md`, linked from
`README.md` and `docs/README.md`. It does not restate the language
specification; each section links the relevant chapter.

**R13.2 Coverage.** Side-by-side C and Clorraine examples cover:

| Pattern | Required content |
| ------- | ---------------- |
| Allocation | `AllocMem`/`FreeMem`, ownership, `take`, `freedBy`, `Box<T>` |
| Files | DOS `Open`/`Read`/`Write`/`Close` versus `std.amiga.dos` and `Buffer` |
| Strings | C NUL strings, views `str`/`cstring`, `String`, conversion rules |
| MMIO | volatile registers, `raw` bases, the per-chip wrappers, `unsafe` |
| Interrupts | server/client setup, handler signature, ownership at the boundary |
| Hooks | `Hook` callbacks, synchronous C hooks, capture rules |
| Ownership modes | value, `borrow`, `take`, view, handle, and when to choose each |
| Error handling | status structs, nullable returns, traps, and partial I/O |

**R13.3 Diagnostics.** A dedicated section lists the diagnostics a C
programmer is most likely to meet (`E10010`, `E10012`, `E10020`, `E10026`,
`E10034`, `E10105`, `E10201`, `E10207`, `E10306`, `E10400`, `E11200`,
`E11215`), the C habit that triggers each, and the fix.

**R13.4 No toolchain dependency.** Every example is a source excerpt that the
prose can explain; the guide is Markdown only and adds no build target.

### R14 — Integration, documentation, and verification

**R14.1 Commands.** `Makefile` gains `setup`, `setup-check`, `vscode-check`,
and `vscode-package`; `make check` runs the installer tests, the LSP suites,
and the `--doc` suites; `make demo-check` remains green. The host-only CI
workflow gains the installer unit tests so they cannot rot without a Docker
daemon. `AGENTS.md` documents the new commands, `tools/setup/`,
`tools/vscode/`, the LSP binary, and `--doc`.

**R14.2 Documentation.** `README.md` gains a short "Editor and tooling"
section linking the language server, the extension, `--doc`, and the Rosetta
guide. `docs/README.md` is the documentation index.

**R14.3 Roadmap.** One row `PLAN-21` tracks this feature through RD Drafted,
Plan Created, Executing, and Done. The header progress count is updated at
every transition.

**R14.4 Gates.** Because the compiler library's orchestration is refactored,
acceptance includes `make check`, `make asan`, `make ubsan`,
`make corpus-amiga`, `make differential`, `make verify`, and `make demo-check`
plus `make vscode-check`. `make verify` must print `verify: all legs passed`.

**R14.5 Hygiene.** No licensed media, ROM, NDK content, generated `.vsix`,
`node_modules`, `build/`, `target/`, generated C, or scratch file is
committed. Temporary test roots live under `$CODEOPS_TMPDIR` or `/tmp/opencode`
and are removed.

## Non-Functional Requirements

### N1 — Determinism and reproducibility

Installer reports, LSP responses for the same inputs, completion lists,
documentation output, and VS Code package contents are deterministic. The
installer's fetch path is the existing pinned Docker build; no floating tag or
unpinned download is introduced.

### N2 — Security

All external input is untrusted. The LSP validates JSON shapes, message
lengths, URIs, and positions; it never executes a method named by the client,
never reads a path outside the initialized root for workspace scans, and
accepts only `file:` URIs and overlay-only `untitled:` documents. The
documentation generator derives output names from validated module names and
rejects a name that escapes the output directory. The installer treats
downloaded content as opaque, verifies before use, and never prints media
bytes.

### N3 — Performance bounds

The workspace scan is capped at 2000 files. A request analyzes at most the
known workspace once; the fixture workspace completes initialize plus five
feature requests in under five seconds on the pinned container. The installer
detection path completes in under ten seconds without a fetch.

### N4 — Compatibility

The LSP targets protocol 3.17 and advertises UTF-16 positions. A client that
does not request a position encoding gets UTF-16. The extension declares a VS
Code engine floor and does not use proposed APIs. Existing compiler and driver
contracts are unchanged.

## Out of Scope

- Language, grammar, diagnostic, ABI, runtime-shim, and standard-library
  changes.
- Compiler performance work and generated-C optimization.
- Amiga wrapper expansion, v2 language work, package manager, project
  manifest, debugger integration, watch mode, and code formatting.
- New Rust crates.
- Redistribution or download of Kickstart ROMs, Workbench media, NDK content,
  or any generated binary.

## Traceability

| Requirement | Acceptance evidence |
| ----------- | ------------------- |
| R1-R3 | `tools/setup/tests/`, `make setup`, installer report fixtures |
| R4-R8 | `compiler/tests/lsp_spec_test.rs`, `compiler/tests/lsp_impl_test.rs`, `make check` |
| R9-R10 | `tools/vscode/test/`, `make vscode-check`, packaged archive audit |
| R11-R12 | `compiler/tests/docgen_spec_test.rs`, fixture plus golden, `make check` |
| R13 | `docs/c-to-clorraine.md`, link check in `make check` |
| R14 | `make check`, `make asan`, `make ubsan`, `make corpus-amiga`, `make differential`, `make verify`, `make demo-check`, `make vscode-check` |
