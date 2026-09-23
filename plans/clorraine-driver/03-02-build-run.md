# Build and Run: Clorraine build/run driver

> **Document**: 03-02-build-run.md
> **Parent**: [Index](00-index.md)

## Overview

This component owns everything after argument parsing: emitting the C in
process, the profile→toolchain/runner table, the artifact layout, the build and
run pipelines, output capture, the FS-UAE handoff, and the internal-compiler-error
report. The command surface and exit codes are owned by
[03-01-driver-cli.md](03-01-driver-cli.md).

## Architecture

### Current Architecture

`clc::driver::run(args) -> i32` writes one C file to `-o` and returns an exit
code. The §13 C-builder commands exist only in docs and shell legs. There is no
artifact convention.

### Proposed Changes

Add a `toolchain` module (the command table), a `build` module (emit +
compile), and a `run` module (execute + capture), all under
`compiler/src/clorraine/`. `clorraine::run` sequences them.

## Implementation Details

### The toolchain table (one source of truth, per AR-16)

The C-builder commands are the normative lines of `spec/17-codegen.md` §13. The
host flags mirror the existing host harness
(`compiler/tests/codegen_run_spec_test.rs:124-134`), which is the project's
established host build.

| Profile | Tool | Base flags | Debug | Release | Libraries |
| ------- | ---- | ---------- | ----- | ------- | --------- |
| `host` | `cc` | `-std=c99` | `-O0 -g` | `-O2` | — |
| `hosted-amiga` | `m68k-amigaos-gcc` | `-std=c99 -mcrt=nix20 -m68000 -fwrapv` | `-O0 -g` | `-O2` | `-lamiga -lm` |

A debug build also appends `-Wall -Wextra -Werror`; a release build omits them,
matching the §13 release line. The command is assembled as an argument array;
no shell is involved (AR-16, AR-22).

Runners:

| Profile | Default runner | Opt-in |
| ------- | -------------- | ------ |
| `host` | execute the built binary | — |
| `hosted-amiga` | `vamos <binary>` | `--emulator` → FS-UAE handoff |

### Artifact layout (per AR-9)

```text
build/clorraine/<stem>/<profile>/
├── <stem>.c     # the emitted C (kept, gitignored)
├── <stem>       # the built program (host ELF or m68k hunk executable)
└── out.txt      # captured stdout, only with --capture
```

- `<stem>` is the first source file's file stem (AR-18).
- `<profile>` is `host` or `hosted-amiga`.
- The program directory is removed and recreated at the start of each build, so
  repeated builds are deterministic (AR-9).

### New Functions

```rust
/// Emits the program's C to `c_path`; returns `clc`'s exit code (0 on success).
fn emit_c(profile: Profile, files: &[String], c_path: &Path) -> i32;

/// Builds `c_path` into `bin_path`; `Err` carries the compiler's first error.
fn compile(tool: &Tool, release: bool, c_path: &Path, bin_path: &Path) -> Result<(), IceError>;

/// Runs the built program, streaming or capturing stdout; returns its code and
/// whether a requested capture was written.
fn run_program(runner: &Runner, bin: &Path, capture: Option<&Path>) -> RunResult;

/// The exact host command that boots FS-UAE, without executing it.
fn fs_uae_handoff(bin: &Path, capture: bool) -> String;
```

### Pipelines

- **emit** — call `emit_c` into a private scratch file under the gitignored
  build tree, then print the C to stdout (`--emit-c` semantics) or write it to
  `-o`; the scratch file is removed afterwards. Exit `clc`'s code (AR-12).
- **build** — `emit_c` → `compile`. Print `built <bin path>`. Exit 0 or the
  step's failure code (AR-3, AR-10).
- **run** — `build`, then `run_program`; exit the program's code (AR-11).
  `--emit-c` additionally prints the C to stdout; for `run` the program output
  follows the C (a debugging mode).
- **handoff** — `run --profile hosted-amiga --emulator` prints
  `fs_uae_handoff(...)` and exits 0; `--capture` adds `RUN_CAPTURE=1` (AR-6,
  AR-25).

### The FS-UAE handoff (per AR-6)

The `dev` container cannot launch a sibling container, so the driver prints:

```text
FS-UAE must run on the host. From the repository root:
  make -C docker run PROG='build/clorraine/<stem>/hosted-amiga/<stem>'
```

The path is single-quoted (and control bytes stripped) because the `<stem>` is
derived from the source file's name, so a pasted line cannot have a byte in the
name interpreted by the shell. With `--capture`, the printed command is prefixed
`RUN_CAPTURE=1 `. The driver exits 0: the handoff is the successful outcome.

### Capture and output (per AR-11, AR-25)

- Without `--capture`, the program's stdout is streamed to the terminal; for
  `vamos`, a `\r` is stripped so the output is terminal-clean.
- With `--capture`, the program's stdout is written (CR-stripped) to
  `<artifact-dir>/out.txt` and the path is printed.
- The program's exit code is propagated unchanged.

### Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| `clc` reports diagnostics | Passed through unchanged; no C build attempted; exit 1 | AR-22 |
| Emitted C fails to compile | Internal compiler error: message, C path, target compiler's first error; exit 1 | AR-10 |
| Runner exits nonzero | Exit with the program's code; the output is still shown | AR-11 |
| Missing tool | Named error + SDK remedy; exit 1 | AR-24 |
| FS-UAE requested | Print the host command; exit 0; never attempt to launch | AR-6 |

The ICE report is one block:

```text
error: internal compiler error: the emitted C failed to compile
  note: emitted C kept at build/clorraine/<stem>/<profile>/<stem>.c
  first error: <target compiler's first error line>
```

## Code Examples

### Example 1: build then run on the host

```sh
$ clorraine run hello.clo
Hello, Clorraine!
```

### Example 2: capture the target run

```sh
$ clorraine run --profile hosted-amiga --capture hello.clo
captured build/clorraine/hello/hosted-amiga/out.txt
```

## Testing Requirements

- Unit tests for `compile` (an injected failing tool yields the ICE shape) and
  `fs_uae_handoff` (exact text, with and without `--capture`).
- Spec tests (end-to-end) for host build/run, `vamos` run, `emit`, and the
  handoff, per the ST cases in `07-testing-strategy.md`.
