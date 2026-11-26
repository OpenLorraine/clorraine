# Windows CLI and FS-UAE spike

This spike tested the Linux product path on Windows before the Windows plan
was written: the upstream rc24 Windows toolchain installs silently, a
cross-built `clorraine.exe` validates it and builds a program, and FS-UAE runs
the program with the user's Kickstart ROM.

Date: 2026-11-26. Host: Windows 11 Pro 24H2 (build 26100), x64, Windows
PowerShell 5.1, an administrator account signed in at the console. The
Windows binaries were cross-built on Linux; nothing was compiled on Windows.

## Pinned inputs

| Input | Value |
|---|---|
| Toolchain | AmigaPorts `m68k-amigaos-gcc` v16.2-rc24 |
| Asset | `m68k-amigaos-gcc-16.2-rc24-x86_64-w64-mingw32ucrt-setup.exe` (Inno Setup) |
| URL | `https://github.com/AmigaPorts/m68k-amigaos-gcc/releases/download/v16.2-rc24/m68k-amigaos-gcc-16.2-rc24-x86_64-w64-mingw32ucrt-setup.exe` |
| Bytes | `48,065,003` |
| SHA-256 | `9dd6973539bb2d514fc1b1e6c0a107cac6f30a9c4076cf4014cbe13013c95512` |
| Installed layout | `{bin,include,lib,libexec,m68k-amigaos,share}`, `.manifest`, `cmdline.bat`, `hello.c`, `unins000.{exe,dat}`; 270 MB; `m68k-amigaos/` contains `libnix`, `ndk-include`, and `ndk13-include` |
| FS-UAE | 3.2.35, `FS-UAE_3.2.35_Windows_x86-64.zip`, 49,123,213 bytes, SHA-256 `b402e2802d518a085c878be96c5c0a656fa3dec57dcf0019d226f3285069063e`; executable at `FS-UAE/Windows/x86-64/fs-uae.exe` |
| Kickstart used | `amiga-os-310-a1200.rom`, 524,299 bytes, SHA-256 `541f7467bce4f4b07a6eb5250d4586a4679c2da51ad30b48ba93421a901b1bb0`, with its `rom.key` in the same directory |
| Host C compilers tried | w64devkit 2.11.0 (GCC 16.2.0, SHA-256 `54c2bb99…7b2b`); LLVM-MinGW 20261117 UCRT (Clang, SHA-256 `4d002e33…a412`) |
| Cross build | `clorraine-sdk:dev` plus `gcc-mingw-w64-x86-64` and the Rust `x86_64-pc-windows-gnu` target |

Files fetched with `curl.exe` carry no Mark of the Web, so SmartScreen does
not inspect them.

## Toolchain install

```
setup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /NOCLOSEAPPLICATIONS /SP- /DIR="<root>" /LOG="<log>"
```

- Exit 0 after 44 seconds; `Need to restart Windows? No`.
- The installer is per-machine: it writes the uninstall key under
  `HKLM\...\Uninstall\{8D184346-D038-45E9-98A1-43BFC542BE34}_is1` and a
  Start-menu shortcut under `ProgramData`. It does not allow a per-user
  install, so an unelevated user needs one UAC prompt.
- The spike ran from an elevated SSH token, so no UAC prompt appeared. The
  prompt itself still needs one in-person check.
- `/DIR` installs outside Program Files. The single `AppId` means one rc24
  registration per machine.
- `bin/m68k-amigaos-gcc.exe --version` reports
  `m68k-amigaos-gcc.exe (AmigaDev 16.2-rc24) 16.2.1b`.
- From the desktop session, `-mcrt=nix20` and `-mcrt=nix13` both compile and
  link a probe; the outputs start with hunk magic `000003f3`.

### SSH sessions cannot start the compiler

Under a key-based OpenSSH logon (OpenSSH_9.5p2 for Windows), `cc1.exe` exits
with `0xC00004BC` (`STATUS_UNTRUSTED_MOUNT_POINT`) with no output. A copy of
`as.exe` launched through `cmd` fails the same way. The tree contains no
reparse points, and the same binaries run from the desktop session. Remote
spike commands therefore run as a one-shot interactive scheduled task
(`run-desktop.ps1`). This limits the test harness, not the product. A user who
runs the CLI over SSH would hit it too; treat that as unsupported until
explained.

## Driver results

All three binaries cross-build cleanly (8.4 MB each) and import only system
DLLs: `KERNEL32`, `msvcrt`, `ntdll`, `USERENV`, `WS2_32`, `bcryptprimitives`,
and `api-ms-win-core-synch`.

| Step | Unmodified driver | With spike patches |
|---|---|---|
| `--version`, `new` | pass | pass |
| `setup --toolchain` | fails: looks for `bin/m68k-amigaos-gcc` | pass; probes both runtimes |
| config file | no `HOME`: no config directory | `%APPDATA%\clorraine\config` |
| `build -p hosted-amiga` | — | pass |
| `run --emulator` | fails: FS-UAE `--version` prints nothing | pass |
| host profile | `cc` never found on `PATH` | pass with GCC; debug fails with Clang |

The spike patches were reverted and are not part of the tree. They added
`.exe` to the compiler name, an `%APPDATA%` config directory, `Version.txt` as
a fallback FS-UAE version source, and `EXE_SUFFIX` in the `PATH` search.

### FS-UAE check equivalent

The same set as `make fs-uae-check` on A1200/KS3.1:

| Program | Profile | Result |
|---|---|---|
| `examples/hello/hello.clo` | hosted-amiga | pass |
| corpus `file-copier`, `string-parser`, `buffer-container`, `volatile-mmio` | hosted-amiga | pass, 7–9 s each including the build |
| corpus `file-copier` | hosted-amiga-13 | pass |
| return-1 negative | hosted-amiga | exit 1, `exited with code 1`, run retained |
| `--capture` | hosted-amiga | pass |

FS-UAE wrote nothing under `Documents`, `%APPDATA%`, or `%LOCALAPPDATA%`;
`FS_UAE_BASE_DIR` and the path options keep it inside the private run. No
private run directory was left behind after a successful run.

### Host profile

| Compiler | Debug (`-Wall -Wextra -Werror`) | Release |
|---|---|---|
| w64devkit GCC 16.2.0 | hello and three corpus programs pass | pass |
| LLVM-MinGW Clang (`cc.exe`) | fails: `-Wunused-function` (`cl_isnan`), `-Wparentheses-equality` | pass, writes `hello.exe` |

The Clang failures come from the emitted C and the strict flags, not from
Windows; a Linux host whose `cc` is Clang should behave the same.

## Defects found

| Defect | Where | Scope |
|---|---|---|
| `#line` paths are not escaped as C string literals. `check\hello.clo` becomes `"check\hello.clo"`: `\h` is an unknown escape and `-Werror` fails the build. `check\file-copier.clo` passes silently, because `\f` is a form feed, so the debug file name is wrong. | `codegen.rs` `emit_line_directive`, `codegen/cleanup.rs` `emit_close_line` | All hosts: a Linux path containing `\` or `"` breaks the same way. Emission change. |
| `canonicalize` returns verbatim `\\?\C:\...` paths, which reach the saved config and user messages. | `setup.rs` `validate`, emulator run paths | Windows |
| The FS-UAE version check reads stdout only; the Windows build is a GUI-subsystem program and prints nothing to a pipe. `Version.txt` and the PE version resource both say `3.2.35`. | `emulator/select.rs` `check_version` | Windows |
| The compiler name, `PATH` search, and FS-UAE lookup use bare names without `.exe`. | `setup.rs`, `environment.rs`, `emulator/select.rs` | Windows |
| The config directory needs `HOME`/`XDG_CONFIG_HOME`. | `setup.rs` `config_file` | Windows |
| `native_syscalls()` is true on any x86_64, so a freestanding host run on Windows would select the raw Linux syscall layer. | `toolchain.rs` | Windows; found by reading, not run |
| LSP `file:` URIs: `file:///c%3A/...` parses to `/c:/...`, and `from_path` encodes backslashes. | `lsp/uri.rs` | Windows; found by reading, not run |
| The editor finds the CLI only at `~/.local/bin`. | `tools/vscode/src/discovery.ts` | Windows; found by reading |

## Active-version switch

- A directory junction needs no privilege and keeps working while a program
  from the old version is running.
- `MoveFileEx(MOVEFILE_REPLACE_EXISTING)` cannot replace a junction (error 5),
  and `Directory.Move` refuses an existing target. The switch is therefore
  delete-then-rename: the old version tree stays intact, but there is a short
  window with no `current`. The installer must finish an interrupted switch
  (`current.new` present, `current` missing) on its next run, or keep the
  active pointer in the per-user `PATH` value instead.

## Execution policy

The test machine has `LocalMachine = AllSigned` and `CurrentUser =
RemoteSigned`. A user without the CurrentUser override cannot run an unsigned
`install.ps1` unless the command passes `-ExecutionPolicy Bypass`, which sets
the process scope and overrides both. A Group Policy setting
(`MachinePolicy`/`UserPolicy`) cannot be bypassed this way.

## Not covered

- The UAC prompt as an unelevated user sees it.
- A standard (non-administrator) account.
- The VS Code extension and the language server on Windows.
- The freestanding profile on Windows.
- Windows 10.
