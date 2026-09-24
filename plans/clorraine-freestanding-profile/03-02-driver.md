# Driver Integration: Freestanding profile

> **Document**: 03-02-driver.md
> **Parent**: [Index](00-index.md)

## Overview

This component teaches the `clorraine` driver about `freestanding`. It owns the
profile's toolchain rows, the two-artifact build/run flow, the CLI text, and the
`make` passthrough. The compiler-side facts live in
[03-01](03-01-profile-runtime.md).

## Architecture

### Current Architecture

`clorraine/toolchain.rs` maps a `Profile` to one `Tool` (program, base flags,
libraries) and one `Runner` (`Native` or `Vamos`). The pipelines in
`clorraine/build.rs` and `clorraine/run.rs` build one artifact from the emitted
C; the command orchestration (`run_command`, `required_tools`) lives in
`clorraine/mod.rs`, and artifact paths in `clorraine/artifacts.rs`. The CLI
(`clorraine/cli.rs:172-175`) rejects any profile name outside `host` and
`hosted-amiga`, and the help text lists the same two.

### Proposed Changes

`freestanding` needs **two** artifacts: the m68k build-only artifact and the host
no-libc run artifact. The toolchain module gains a run-tool lookup separate from
the build tool, `mod.rs` routes the freestanding run to the host artifact and
requires `cc`, `artifacts.rs` owns the new host-artifact path, and the CLI
accepts the name and lists it in the help (AR-4, AR-20).

## Implementation Details

### Toolchain rows

`compiler/src/clorraine/toolchain.rs`:

```rust
/// Freestanding m68k build flags: no C runtime, its own start-up, soft floats.
const FREESTANDING_BUILD_BASE: &[&str] =
    &["-std=c99", "-ffreestanding", "-fno-builtin", "-nostdlib", "-m68000", "-fwrapv"];

/// The freestanding host-run flags: the same no-libc stance on the host.
const FREESTANDING_HOST_BASE: &[&str] =
    &["-std=c99", "-ffreestanding", "-fno-builtin", "-nostdlib"];

/// The freestanding m68k link libraries (the self-contained soft-float runtime).
const FREESTANDING_LIBS: &[&str] = &["-lg"];

/// The optimizer that links the freestanding host no-libc artifact. On an
/// architecture without a syscall layer in the shim, the caller drops
/// `-nostdlib` so the platform C runtime supplies `write` and `_exit`.
pub fn run_tool(profile: Profile, native_syscalls: bool) -> Tool { /* ... */ }
```

| Function | `host` | `hosted-amiga` | `freestanding` |
| -------- | ------ | -------------- | -------------- |
| `tool` (build) | `cc`, host base | `m68k-amigaos-gcc`, amiga base, `-lamiga -lm` | `m68k-amigaos-gcc`, freestanding build base, `-lg` |
| `run_tool` | same as build | none (the build artifact is run under `vamos`) | host `cc`, freestanding host base (minus `-nostdlib` on the fallback path) |
| `runner` | `Native` | `Vamos` | `Native` |

`native_syscalls` is true on x86_64/aarch64; the plan records it as a single
constant so the fallback is one branch, not a scatter of `cfg!` checks (AR-8).

### Artifact layout

For `build/clorraine/<stem>/freestanding/`:

| Artifact | Meaning |
| -------- | ------- |
| `<stem>` | the m68k build-only artifact (compiled and linked) |
| `<stem>.host` | the host no-libc artifact that `run` executes; its path comes from a new `artifacts::host_bin_path` helper |

`build` produces only `<stem>`; `run` produces both. `emit` is unchanged and
writes the same C the other profiles write for the same inputs (AR-20).

### Build and run flow

`compiler/src/clorraine/build.rs`:

- `emit_c` is unchanged: it calls `clc::driver::run` in process.
- `compile` builds the profile's `tool` artifact. For freestanding this is the
  m68k link, so a successful build already proves the `-nostdlib -lg` link
  (AR-27). A C-compiler failure stays an internal compiler error with the C path
  and the first error.

`compiler/src/clorraine/mod.rs`:

- `run_command` for `freestanding`: build the m68k artifact, then compile
  `<stem>.host` with `run_tool`, then execute it natively. Stream stdout and
  propagate the exit code as for `host`; `--capture` writes `out.txt` as usual
  (AR-20).
- `required_tools` for a freestanding run lists `cc` (the host artifact's
  compiler) as well as the m68k build compiler.
- For `host` and `hosted-amiga`, `run_command` is unchanged.

### CLI

`compiler/src/clorraine/cli.rs`: `Profile::parse` now accepts `freestanding`, so
the `--profile` path needs no new branch; the unknown-profile message and the
`HELP` text gain `freestanding` (AR-4). The `clc` usage message in
`compiler/src/driver.rs:231` (`--profile <host|hosted-amiga> is required`) is
updated the same way. `--emulator` stays hosted-amiga only.

### Make passthrough

The root `Makefile` must change: its `PROFILE_ARG` currently forwards only
`hosted-amiga` (`Makefile:34`), so `PROFILE=freestanding` would be dropped and
the host profile would run silently. Change it to forward any non-host profile,
for example
`PROFILE_ARG = $(if $(filter-out host,$(PROFILE)),--profile $(PROFILE),)`.
With that one line, `make run FILE=… PROFILE=freestanding` builds and runs
through the driver (AR-17, AR-20).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| `m68k-amigaos-gcc` missing | the existing missing-tool message names it and the SDK remedy | AR-20 |
| `cc` missing for the host run | the existing missing-tool message names it | AR-20 |
| Emitted C fails the m68k build | internal compiler error with the C path and the first error | AR-7 |
| Emitted C fails the host build | the same internal-compiler-error report, naming the host leg | AR-20 |
| `--emulator` with freestanding | usage error, as for any non-hosted-amiga profile | AR-20 |

## Testing Requirements

- Unit tests in `toolchain.rs` for the freestanding build and run command
  vectors and the `-lg`/`-nostdlib` placement (ST-11).
- Unit tests in `cli.rs` for `--profile freestanding` acceptance and the help
  text (ST-1).
- The end-to-end driver behaviour is covered by the freestanding run harness
  ([07-testing-strategy.md](07-testing-strategy.md)).
