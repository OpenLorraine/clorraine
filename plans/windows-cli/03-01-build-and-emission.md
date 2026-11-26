# Build and Emission: Windows CLI and installer

> **Document**: 03-01-build-and-emission.md
> **Parent**: [Index](00-index.md)

## Overview

Two foundations every later phase needs: a pinned image that cross-builds and checks the Windows
target, and emitted C that is correct for Windows paths and for the freestanding host run on
Windows.

## Architecture

### Current Architecture
- `docker/Dockerfile` installs Rust `stable` and the Amiga tools; `make check` and `make dist`
  run in `IMAGE ?= clorraine-sdk:dev` (`docker/Makefile:27`).
- `#line` paths are emitted verbatim (`codegen.rs:815`, `cleanup.rs:330`).
- The freestanding shim picks its syscall layer by architecture only.

### Proposed Changes

#### Image (AR-20, AR-37)
- Append one layer after the existing ones: install `gcc-mingw-w64-x86-64` and run
  `rustup target add x86_64-pc-windows-gnu` as the `dev` user. Earlier layers stay cached.
- This branch builds and uses `IMAGE=clorraine-sdk:windows-cli`. `clorraine-sdk:dev` is rebuilt
  from the merged Dockerfile after merge.

#### `make check` (AR-20)
- Add `cargo check --workspace --target x86_64-pc-windows-gnu` inside the container, after
  `cargo test --workspace`. It needs the target's standard library only.

#### `#line` escaping (AR-16)
- One helper, `codegen::c_string_body(text: &str) -> String`, escapes `\` as `\\`, `"` as `\"`,
  and every byte below 0x20 or equal to 0x7F as a three-digit octal escape. Both directive writers
  use it.
- Plain ASCII paths without those characters are emitted byte-for-byte as before.

#### Freestanding shim (AR-23, AR-44)
- `toolchain::native_syscalls()` returns `native_syscalls_for(std::env::consts::OS,
  std::env::consts::ARCH)`; the pure `pub fn native_syscalls_for(target_os: &str, target_arch:
  &str) -> bool` is true only for `linux` on `x86_64` or `aarch64`.
- `shim_freestanding.c`: the x86_64 and aarch64 syscall branches also require `__linux__`; any
  other host takes the existing C-runtime fallback.
- `uintptr_t` becomes `unsigned long long` only under `_WIN64`; every other target keeps
  `unsigned long`.
- Compiled output on Linux and m68k is unchanged; the emitted C text changes, so the shim-text
  pins are updated for this change only.

## Implementation Details

### New Functions/Methods

```rust
/// Escapes text for the inside of a C string literal (no surrounding quotes).
pub(crate) fn c_string_body(text: &str) -> String;
```

### Integration Points
- `make dist` (03-05) uses the same image for the Windows build.
- The emission-integrity auditor compares base/head emission over the corpus (AGENTS.md routing).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| The branch image is missing | `docker/Makefile` `check-image` already fails with the build hint | AR-37 |
| A Windows-only `cfg` branch does not compile | `make check` fails at the `cargo check` step | AR-20 |
| A path contains a control character | Octal escape; the C string stays valid | AR-16 |

## Testing Requirements
- ST-1 – ST-8 in [07-testing-strategy.md](07-testing-strategy.md).
