# Release Flags: Build-Flag and Binary-Size Closeout (PLAN-26)

> **Document**: 03-01-release-flags.md
> **Parent**: [Index](00-index.md)

## Overview

The driver's release command becomes per-profile. Hosted-amiga,
hosted-amiga-13, and Linux host get section garbage collection and release
stripping; freestanding (m68k and host-run) and non-Linux hosts keep `-O2`.
Debug builds are untouched. The frozen compile matrix's two GCC release rows
change to match, under the user's ruling (AR-19).

## Architecture

### Current Architecture

`Tool` carries `program`, `base`, and `libs`; `command()` appends one global
`RELEASE_FLAGS = ["-O2"]` or `DEBUG_FLAGS`. `tool(Profile::Host)` and
`run_tool(Profile::Freestanding, ..)` cannot select different release flags,
and the host choice cannot depend on the linker family without changing the
shape.

### Proposed Changes

Add a `release: &'static [&'static str]` field to `Tool`. Define four release
slices and one pure selector:

```rust
const HOST_RELEASE_GNU: &[&str] = &[
    "-O2",
    "-ffunction-sections",
    "-fdata-sections",
    "-Wl,--gc-sections",
    "-s",
];
const HOST_RELEASE_FALLBACK: &[&str] = &["-O2"];
const M68K_RELEASE_FLAGS: &[&str] = &[
    "-O2",
    "-ffunction-sections",
    "-fdata-sections",
    "-Wl,--gc-sections",
    "-s",
];
const FREESTANDING_RELEASE_FLAGS: &[&str] = &["-O2"];

/// Selects the host release flags by linker family.
fn host_release_flags(gnu_linker: bool) -> &'static [&'static str] {
    if gnu_linker { HOST_RELEASE_GNU } else { HOST_RELEASE_FALLBACK }
}

/// True on the Linux host, whose pinned linker accepts `--gc-sections`.
///
/// The name says Linux, not "GNU", because a non-GNU linker on another
/// platform is not probed. The pure `host_release_flags(bool)` seam is what
/// the tests exercise.
fn host_is_linux() -> bool {
    cfg!(target_os = "linux")
}
```

`tool(Profile::Host)` uses `host_release_flags(host_is_linux())`.
`tool(Profile::HostedAmiga)` and `tool(Profile::HostedAmiga13)` use
`M68K_RELEASE_FLAGS`. `tool(Profile::Freestanding)` and
`run_tool(Profile::Freestanding, ..)` use `FREESTANDING_RELEASE_FLAGS`.
`run_tool(Profile::Host, ..)` returns `tool(Profile::Host)`. `command()` uses
`tool.release` instead of the removed global `RELEASE_FLAGS`; the argument
order stays `<program> <base> <release|debug> [warnings] -o <bin> <c> <libs>`.

`Tool` is also constructed by two test doubles in
`compiler/src/clorraine/build.rs:159-185`; both gain `release: &["-O2"]` so
`cargo test -p clc` still compiles. This is part of the same change, not a
separate behavior.

## Implementation Details

### Exact Command Vectors

| Profile / mode | Vector after the change |
|---|---|
| host Linux release | `cc -std=c99 -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o prog out.c` |
| host non-Linux release | `cc -std=c99 -O2 -o prog out.c` |
| host debug | `cc -std=c99 -O0 -g -Wall -Wextra -Werror -o prog out.c` |
| hosted-amiga release | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o prog out.c -lamiga -lm` |
| hosted-amiga-13 release | the same with `-mcrt=nix13` |
| hosted-amiga debug | unchanged `-O0 -g -Wall -Wextra -Werror` line |
| freestanding m68k release | `m68k-amigaos-gcc -std=c99 -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv -O2 -o prog out.c -lg` |
| freestanding host-run release (native syscalls) | `cc -std=c99 -ffreestanding -fno-builtin -nostdlib -O2 -o prog out.c` |
| freestanding host-run release (fallback) | `cc -std=c99 -ffreestanding -fno-builtin -O2 -o prog out.c` |

### Specification Matrix

`spec/17-codegen.md` §13 changes exactly two rows:

```
| GCC release | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o demo demo.c -lamiga -lm` |
| GCC 1.3 release | `m68k-amigaos-gcc -std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o demo demo.c -lamiga -lm` |
```

The debug rows, vbcc row, freestanding rows, and every other chapter stay
byte-identical.

`spec/check.sh` checks chapter structure, not the command strings, so a
`compiler/tests/clorraine_driver_spec_test.rs` case reads
`spec/17-codegen.md`, extracts the compile-matrix table, and asserts the two
release rows exactly plus the six unchanged rows. That is ST-8; it fails if a
row drifts.

One pre-existing discrepancy is recorded, not edited: the freestanding
release row says "the same line with `-O2` in place of `-O0 -g`", which would
retain `-Wall -Wextra -Werror`, while the driver's freestanding release line
omits warnings. Only the two GCC release rows are authorized by AR-19; the
freestanding row stays as it is, and the findings note the mismatch so it is
not mistaken for this feature's work.

### CLI Text

The `--release` help line becomes:

```
      --release                       Optimize (-O2); strip + section-GC on Linux/Amiga; freestanding/non-Linux and debug keep symbols
```

The `Options.release` doc says the same in one sentence. The text names the
two exclusions explicitly so it does not imply that every release artifact is
stripped; the full behavior lives in the spec/limitations docs.

## Error Handling

| Error case | Handling | AR Ref |
|---|---|---|
| Non-Linux host linker | Keep `-std=c99 -O2`; no GNU-only flag | AR-06 |
| Freestanding release | Keep `-O2` for m68k and host-run | AR-07 |
| A needed constructor is collected | Stop the landing; add a targeted `KEEP`/`-u`, re-audit | AR-13 |
| A command vector drifts | Rust tests fail; harness cross-check fails | AR-22/AR-25 |

## Testing Requirements

- Exact debug and release vector tests for `host`, `hosted-amiga`,
  `hosted-amiga-13`, and `freestanding`.
- Both `host_release_flags(true)` and `host_release_flags(false)` tested on
  the Linux host.
- Both `run_tool(Profile::Freestanding, true/false)` release vectors tested.
- A spec/text assertion or review check that only the two release rows
  changed.
