# Driver CLI: Clorraine build/run driver

> **Document**: 03-01-driver-cli.md
> **Parent**: [Index](00-index.md)

## Overview

This component owns the `clorraine` command surface: parsing, validation,
dispatch, help/version, exit codes, and the two environment checks. The build,
run, artifact, and error-reporting mechanics are owned by
[03-02-build-run.md](03-02-build-run.md).

## Architecture

### Current Architecture

`clc` parses its own arguments in `compiler/src/driver.rs` (`Options::parse`)
and exposes `pub fn run(args: Vec<String>) -> i32`. There is no second command
and no notion of "build" or "run".

### Proposed Changes

Add `compiler/src/clorraine/` with a `cli` module and a top-level entry
`clorraine::run(args) -> i32`, plus a thin binary `compiler/src/clorraine_main.rs`
that forwards `std::env::args()` (mirroring `compiler/src/main.rs`).

## Implementation Details

### New Types/Interfaces

```rust
/// The selected command.
enum Command { Run, Build, Emit }

/// A parsed, validated invocation.
struct Options {
    command: Command,
    profile: Profile,        // default Profile::Host (AR-8)
    release: bool,           // AR-19
    emit_c: bool,            // AR-9
    emulator: bool,          // AR-5, AR-6
    capture: bool,           // AR-25
    output: Option<String>,  // emit only (AR-12)
    files: Vec<String>,      // one or more (AR-18)
}

/// Parses and validates the command line, or returns a usage error.
fn parse(args: &[String]) -> Result<Options, UsageError>;

/// The process entry point; returns the process exit code.
pub fn run(args: Vec<String>) -> i32;
```

### Command surface

```text
clorraine <command> [options] FILE...

Commands:
  run     Build and run a program
  build   Build a program without running it
  emit    Print the generated C (or write it with -o)

Options:
  -p, --profile <host|hosted-amiga>   Target profile (default: host)
      --release                       Optimize (-O2) instead of debug (-O0 -g)
      --emit-c                        Print the generated C after building
      --emulator                      hosted-amiga run: use FS-UAE (host handoff)
      --capture                       Write program stdout to out.txt; print its path
  -o FILE                             emit only: write C to FILE (default: stdout)
  -h, --help                          Print this help
  -V, --version                       Print the version
```

### Dispatch and validation (per AR-3, AR-26)

| Rule | Behaviour |
| ---- | --------- |
| No command, or `-h`/`--help` | Print help to stdout, exit 0 |
| `-V`/`--version` | Print `clorraine <version>` (from the crate), exit 0 |
| Unknown command | Usage error, exit 2 |
| Unknown flag | Usage error, exit 2 |
| `--profile` value not `host`/`hosted-amiga` | Usage error, exit 2 |
| No `FILE` | Usage error, exit 2 |
| `-o` with `run`/`build` | Usage error, exit 2 (it is `emit`-only) |
| `--emulator` with `emit`/`build`, or with `host` | Usage error, exit 2 |
| `--capture` with `emit`/`build` | Usage error, exit 2 |
| `--release`/`--emit-c` with `emit` | Usage error, exit 2 (`emit` produces C, not a build) |

### Environment checks (per AR-7, AR-23, AR-24)

```rust
/// The SDK image marker; its absence means the driver is outside the container.
const SDK_MARKER: &str = "/opt/amiga/BUILD-PROVENANCE.txt";

/// True when the process runs inside the pinned SDK container.
fn sdk_present(marker: &str) -> bool;

/// The first required tool that is not on PATH, if any.
fn missing_tool(tools: &[&str]) -> Option<String>;
```

- **Outside the SDK** (`sdk_present` false): print guidance ("run inside the
  SDK: `make -C docker exec CMD=\"…\"` or `make -C docker shell`"), exit 2.
- **Missing tool**: name it and the SDK remedy, exit 1. The required tools are
  `cc` for a host build, and `m68k-amigaos-gcc` (and `vamos` for the default
  runner) for `hosted-amiga`.

Both functions take their inputs as parameters so unit tests can exercise them
without changing the host environment.

### Integration Points

`cli::parse` produces `Options`; `03-02` consumes it. `run` sequences:
environment check → parse → dispatch to emit/build/run → return the code.

## Code Examples

### Example 1: a host run

```sh
clorraine run hello.clo
# builds build/clorraine/hello/host/hello, runs it, prints:
# Hello, Clorraine!
```

### Example 2: the FS-UAE handoff

```sh
clorraine run --profile hosted-amiga --emulator hello.clo
# prints (and exits 0):
# FS-UAE must run on the host. From the repository root:
#   make -C docker run PROG=build/clorraine/hello/hosted-amiga/hello
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Unknown command/flag, bad value, missing file, inapplicable flag | Usage error message to stderr, exit 2 | AR-26 |
| Outside the SDK container | Guidance to stderr, exit 2 | AR-23 |
| Required tool missing | Named error + SDK remedy, exit 1 | AR-24 |
| Clorraine diagnostics from `clc` | Passed through unchanged; exit 1 | AR-22 |

> **Traceability:** every strategy cites the Ambiguity Register entry that
> resolved it; see `00-ambiguity-register.md`.

## Testing Requirements

- Unit tests in this module for `parse` (each usage-error rule, help/version,
  defaults) and for `sdk_present`/`missing_tool` with injected inputs.
- Spec tests (end-to-end) in `compiler/tests/clorraine_driver_spec_test.rs` per
  the ST cases in `07-testing-strategy.md`.
