# Platform port: Windows and macOS follow-up

> **Document**: 03-02-platform-port.md
> **Parent**: [Index](00-index.md)
> **AR**: AR-2/9/10/27/32

## Overview

v1.2.0 ships Linux x64. The same binaries must build and run on macOS arm64 and
Windows x64 in follow-up releases. The compiler is portable Rust; the driver's
SDK resolver and a few file/process details are not.

## Linux v1.2.0

The existing Docker SDK remains the development/test environment. The native
driver must accept the upstream toolchain root (03-01 §Driver toolchain
resolution) while preserving `--sdk-root`/`CLORRAINE_SDK` compatibility and the
old installed layout. `make verify` continues to use the Docker toolchain; a
separate upstream-toolchain corpus leg is a Phase 1/2 evidence task.

## Windows x64 follow-up

- Use `Path`/`PathBuf`; executable names get `.exe`.
- Home: `%LOCALAPPDATA%\Clorraine`; config stores absolute Windows paths.
- Activation: copy the versioned `.exe` files into `bin` through a temporary
  directory/rename; keep the old `bin` until the new CLI passes. No symlink or
  junction privilege is required.
- `std::fs::rename` cannot replace an existing file reliably; use a fresh target
  or backup/rename, and fail without corrupting the old bin.
- Omit Unix permission calls; use ordinary directories and private temp names.
- Use `Command` with literal argv; no shell.
- FS-UAE discovery checks `--fs-uae`, `CLORRAINE_FS_UAE`, config, PATH
  (`fs-uae.exe`), and `Program Files`/`Program Files (x86)` defaults.
- Isolate FS-UAE `APPDATA`/`LOCALAPPDATA`/base/cache dirs in the child.
- Non-UTF-8 process arguments: the binaries must use `args_os()` and report a
  usage error, not panic.

## macOS arm64 follow-up

- Home `~/.clorraine`; `shasum -a 256` for installer verification.
- FS-UAE discovery: `--fs-uae`, env, config, PATH, then
  `/Applications/FS-UAE.app/Contents/MacOS/fs-uae` and
  `~/Applications/FS-UAE.app/...`. Do not search Documents.
- macOS >= 14 arm64; Intel Mac unsupported (AR-32).
- Unix permissions remain valid; guard Linux-only nonblocking flags.

## Corrected scope of code changes

| File | Actual change |
|---|---|
| `compiler/src/clorraine/environment.rs` | Remove the Linux-only native-SDK gate; accept upstream `bin/` root; precedence; platform executable check |
| `compiler/src/clorraine/emulator.rs` | Optional ROM/AROS; config/env/`.exe`/macOS defaults; Windows env isolation |
| `compiler/src/clorraine/artifacts.rs` | Windows-safe directory creation (already `cfg`-guarded) |
| `compiler/src/clorraine/mod.rs` | Native `vamos` PATH check; per-OS missing-tool text; `args_os()` entry |
| `compiler/src/clorraine/cli.rs` | `setup` command; `--toolchain`; document `--fs-uae`/`--kickstart` only on setup/run --emulator |
| `compiler/src/clorraine_main.rs`, `main.rs` | `args_os()` conversion |
| `native_metadata.rs` | Only real platform gaps; existing non-Unix branch remains |

`mod.rs:522` and `artifacts.rs:112-115` are test/non-Unix blocks; do not list them
as production blockers. The real Linux-only gate is `environment.rs:78`.

## Test command per platform

`cargo test --workspace` is not portable as-is: host tests hardcode `cc`, shell
fixtures and `/` paths. The follow-up plan adds a `cc` shim/Windows batch fixture
and normalizes path assertions, or restricts the Windows/macOS job to
`cargo test --workspace --no-run` plus platform/setup tests. The command is
recorded in that plan's platform task and CI matrix.

## Error handling

| Case | Behavior | AR |
|---|---|---|
| Unsupported arch | Exit 1 with supported list | AR-2/32 |
| Windows HOME unset | Use LOCALAPPDATA; fail if neither usable | AR-6 |
| Explicit bad path | Exit 1; no fallback | AR-26 |
| macOS Intel/WSL | Unsupported; no false claim | AR-32 |
| Windows rename conflict | Fresh target or fail without corrupting old bin | AR-30 |

## Testing requirements

- Platform-neutral helper tests for home/config/path/prefix.
- Windows/macOS hosted-runner tests and a `cc` fixture or build-only gate.
- Real-machine evidence before any follow-up release.
