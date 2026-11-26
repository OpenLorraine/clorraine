# Driver: Windows CLI and installer

> **Document**: 03-02-driver.md
> **Parent**: [Index](00-index.md)

## Overview

Make `clorraine`, `clc`, and the verified FS-UAE runner work on Windows 11 with the smallest
contained changes, keeping every Linux behaviour except the `CLORRAINE_CC` override (AR-29).
Windows-specific decisions are pure functions that take the host OS as a parameter, so the Linux
`cargo test` in `make check` exercises the Windows branches (AR-21).

## Architecture

### Current Architecture
Unix assumptions are spread over `setup.rs`, `environment.rs`, `emulator/select.rs`,
`emulator/storage.rs`, `artifacts.rs`, `new.rs`, and `toolchain.rs` (see 02-current-state
§Code Analysis).

### Proposed Changes
A new module `compiler/src/clorraine/platform.rs` owns the host-OS decisions. Call sites pass
`HostOs::current()`; tests pass `HostOs::Windows` or `HostOs::Unix` directly.

## Implementation Details

### New Types/Interfaces

```rust
/// The host family whose naming and directory rules the driver follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostOs { Unix, Windows }

impl HostOs {
    /// The family of the running host.
    pub fn current() -> Self;
}
```

### New Functions/Methods

All in `platform.rs`, all `pub` so the integration suites can call them.

| Function | Contract | AR Ref |
|---|---|---|
| `executable_name(os, base: &str) -> String` | `base` on Unix; `base + ".exe"` on Windows unless `base` already ends in `.exe` (ASCII case-insensitive) | AR-10, AR-45 |
| `display_path(os, text: &str) -> String` | Windows: `\\?\UNC\server\share\…` → `\\server\share\…`; `\\?\X:\…` → `X:\…`; otherwise unchanged. Unix: unchanged | AR-38 |
| `config_file(os, var: impl Fn(&str) -> Option<OsString>) -> Result<PathBuf, String>` | Unix: today's `XDG_CONFIG_HOME` → `HOME/.config` rule, `…/clorraine/config`. Windows: `%APPDATA%\clorraine\config`; `HOME` and `XDG_CONFIG_HOME` are ignored; a missing or relative `APPDATA` is an error naming `APPDATA` | AR-10, AR-43 |
| `host_compilers(os, override_cc: Option<&OsStr>) -> Vec<OsString>` | With an override: exactly that value. Unix: `["cc"]`. Windows: `["gcc", "cc", "clang"]` | AR-2, AR-29 |
| `missing_compiler_hint(os) -> &'static str` | Unix: `install a C compiler (cc)` (unchanged). Windows: `install GCC (for example w64devkit or MSYS2 UCRT64) or set CLORRAINE_CC` | AR-15, AR-29 |
| `host_artifact_name(os, stem: &str, host_run: bool) -> String` | Unix: `stem` / `stem.host`. Windows: `stem.exe` / `stem.host.exe` | AR-45 |
| `pe_file_version(bytes: &[u8]) -> Option<(u64, u64, u64)>` | Requires `MZ` and a `PE\0\0` header at the `e_lfanew` offset; returns the major, minor, and patch of the first `VS_FIXEDFILEINFO` (signature `0xFEEF04BD`, structure version `0x00010000`); `None` otherwise | AR-14 |
| `shell_word(os, word: &str) -> String` | Unix: today's POSIX quoting. Windows: plain words unchanged, otherwise wrapped in double quotes | AR-32 |

### OS-parameterized entry points

The existing functions keep their signatures and call these with `HostOs::current()`:

| Function | Contract | AR Ref |
|---|---|---|
| `setup::validate_for(os, selected: &Path, mode: Validation) -> Result<Toolchain, SetupError>` | Today's validator with the OS-specific compiler name and display paths | AR-1, AR-10 |
| `emulator::select::fs_uae_version_for(os, executable: &Path) -> Result<(u64, u64, u64), String>` | Windows: the PE version of the file (never executed); Unix: `--version` | AR-14 |
| `new::launcher_for(os, invoked: &OsStr, search_path: &OsStr) -> String` | Today's launcher rule with `executable_name` for the PATH lookup | AR-32 |

### Call-site changes

| Site | Change | AR Ref |
|---|---|---|
| `setup.rs` validator | Compiler path `bin\` + `executable_name(os, "m68k-amigaos-gcc")`; the stored and printed root and compiler use `display_path`; `check_glibc` runs on Linux only | AR-1, AR-38 |
| `setup.rs` `config_file` | Delegates to `platform::config_file` | AR-43 |
| `environment.rs` PATH search | Checks `executable_name(os, tool)`; the cc hint comes from `missing_compiler_hint` | AR-29 |
| `toolchain.rs` host and freestanding run tools | The program is resolved at run time: `CLORRAINE_CC`, else the first `host_compilers` entry found on PATH; `Tool.program` becomes the placeholder the resolver replaces | AR-29 |
| `emulator/select.rs` | PATH fallback looks for `executable_name(os, "fs-uae")`; on Windows the version comes from `pe_file_version` of the executable's bytes (read at most 256 MiB (AR-58); larger is an error); Unix keeps `--version` | AR-14 |
| `emulator/mod.rs` spawn | On Windows also sets `TEMP` and `TMP` to the run's `tmp`; `XAUTHORITY` only on Unix | AR-14 |
| `emulator/storage.rs` | Retained-run and error messages use `display_path`; messages name `TMP` on Windows (read before `TEMP`) and `TMPDIR` on Unix; the child's `TEMP`/`TMP` use the plain spelling; Windows relies on the per-user ACL of `%TEMP%` | AR-38, AR-42 |
| `artifacts.rs` | Host and freestanding host-run names from `host_artifact_name`; Amiga artifacts unchanged | AR-45 |
| `new.rs` launcher and summary | PATH lookup uses `executable_name`; an invoked path is used as written (so the junction path stays); the summary quotes with `platform::shell_word` | AR-32 |

### Integration Points
- `install.ps1` invokes `…\Clorraine\current\bin\clorraine.exe` for `setup`, the hellos, and
  `new`, so the starter's tasks follow upgrades (03-04, AR-32).
- The LSP and extension use `display_path` semantics (03-03).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| `APPDATA` unset or relative on Windows | Exit 2: `clorraine: cannot determine a config directory; set APPDATA` | AR-43 |
| `CLORRAINE_CC` names nothing runnable | Exit 1: `clorraine: CLORRAINE_CC '<value>' was not found` | AR-29 |
| No host compiler on Windows | Exit 1 with the Windows hint | AR-29 |
| FS-UAE executable without a readable version resource | Exit 2: `--fs-uae: cannot read the FS-UAE version from '<path>'` | AR-14 |
| FS-UAE executable over 256 MiB (AR-58) | Exit 2: same message | AR-14 |
| FS-UAE older than 3.2.0 | Unchanged message | AR-14 |

## Testing Requirements
- ST-9 – ST-30 in [07-testing-strategy.md](07-testing-strategy.md); every Windows branch runs on
  Linux through `HostOs::Windows`.
