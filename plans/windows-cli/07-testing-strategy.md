# Testing Strategy: Windows CLI and installer

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
|---|---|
| `platform.rs`, URI and path normalization, PE version parser, `#line` escaping | 90% |
| Driver call sites, installer functions, packaging, evidence validator | 80% |
| Docs, site, glue | 60% |

- Test names state behavior: `should [expected behavior] when [condition]`.
- Windows branches run on Linux through `HostOs::Windows` (AR-21). PowerShell suites run on the
  Windows host through the remote helper (AR-47, AR-48). End-to-end is the Windows acceptance
  (AR-46).

## Test Slices & Groups

`N/A — full verify only`: the project has no test map. Gate: `make check
IMAGE=clorraine-sdk:windows-cli` on Linux; the PowerShell suites on Windows; final `make verify`
plus `make acceptance-check` of the Windows record (AR-47).

## Supersession of existing expectations

Two existing specification suites pin Linux-only facts that this plan changes by explicit user
ruling. Only these expectations change; every other expectation in those suites stays:

- `tools/setup/tests/dist.spec.test.sh`: "exactly three assets" becomes five (AR-33).
- `tools/setup/tests/acceptance_spec.py`: "platform must be linux" and the constant version become
  platform-keyed with the version read from the candidate (AR-46).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived from `01-requirements.md`, the `03-XX` specs, and the register. Immutable oracle: if the
> implementation disagrees, the implementation is wrong. Test comments quote the behavior in plain
> language, never an ST or AR identifier.

### Build and emission (03-01)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-1 | In `clorraine-sdk:windows-cli`: `rustup target list --installed`; `x86_64-w64-mingw32-gcc --version` | Lists `x86_64-pc-windows-gnu`; exits 0 | AR-20 |
| ST-2 | `make check IMAGE=clorraine-sdk:windows-cli` | Runs `cargo check --workspace --target x86_64-pc-windows-gnu` and passes | AR-20, AR-37 |
| ST-3 | `clc` emits a program whose source file is named `back\slash.clo` (a Linux file name containing `\`) | The C contains `#line 1 "<dir>/back\\slash.clo"` and builds with `cc -std=c99 -Wall -Wextra -Werror` | AR-16 |
| ST-4 | Source file name `quo"te.clo` | `#line … "<dir>/quo\"te.clo"`; builds | AR-16 |
| ST-5 | Source file name containing a tab | The tab appears as `\011`; builds | AR-16 |
| ST-6 | `examples/hello/hello.clo` | Every `#line` directive is byte-identical to the base emission | AR-16 |
| ST-7 | Emit a freestanding program | The shim's syscall branches require `__linux__`; `uintptr_t` is `unsigned long long` only under `_WIN64`, otherwise `unsigned long` | AR-44 |
| ST-8 | `toolchain::native_syscalls_for(target_os, target_arch)` | `("linux", "x86_64")`, `("linux", "aarch64")` → true; `("windows", "x86_64")`, `("macos", "aarch64")`, `("linux", "m68k")` → false | AR-23 |

### Driver (03-02)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-9 | `executable_name(Windows, "m68k-amigaos-gcc")`; `(Windows, "x.EXE")`; `(Unix, "cc")` | `m68k-amigaos-gcc.exe`; `x.EXE`; `cc` | AR-10, AR-45 |
| ST-10 | `display_path(Windows, r"\\?\C:\Users\a b\rc24")` | `C:\Users\a b\rc24` | AR-38 |
| ST-11 | `display_path(Windows, r"\\?\UNC\srv\share\x")`; `display_path(Unix, r"\\?\C:\x")` | `\\srv\share\x`; unchanged | AR-38 |
| ST-12 | `config_file(Windows)` with `APPDATA=C:\Users\u\AppData\Roaming`, `HOME=/c/Users/u`, `XDG_CONFIG_HOME=/x` | `C:\Users\u\AppData\Roaming\clorraine\config` | AR-43 |
| ST-13 | `config_file(Windows)` with `APPDATA` unset; with `APPDATA=rel\dir` | Error text contains `set APPDATA` (both) | AR-43 |
| ST-14 | `config_file(Unix)` with absolute `XDG_CONFIG_HOME=/x`; with only `HOME=/h` | `/x/clorraine/config`; `/h/.config/clorraine/config` | AR-43 |
| ST-15 | `host_compilers(Windows, None)`; `(Unix, None)`; `(Windows, Some("C:\gcc\bin\gcc.exe"))` | `[gcc, cc, clang]`; `[cc]`; `[C:\gcc\bin\gcc.exe]` | AR-29 |
| ST-16 | `missing_compiler_hint(Windows)`; `(Unix)` | `install GCC (for example w64devkit or MSYS2 UCRT64) or set CLORRAINE_CC`; `install a C compiler (cc)` | AR-29 |
| ST-17 | Linux: `CLORRAINE_CC=<fake compiler that records argv and writes the output>`; `clorraine build hello.clo` | The fake is invoked with the host flags; exit 0 | AR-29 |
| ST-18 | Linux: `CLORRAINE_CC=/no/such/cc clorraine build hello.clo` | Exit 1; stderr `clorraine: CLORRAINE_CC '/no/such/cc' was not found` | AR-29 |
| ST-19 | `host_artifact_name(Windows, "hello", false/true)`; `(Unix, …)` | `hello.exe` / `hello.host.exe`; `hello` / `hello.host` | AR-45 |
| ST-20 | `pe_file_version(<minimal PE32+ fixture whose VS_FIXEDFILEINFO is 3.2.35.0>)` | `Some((3, 2, 35))` | AR-14 |
| ST-21 | `pe_file_version` of an ELF file; of a PE without the signature; of a 100-byte prefix of the fixture | `None` (each) | AR-14 |
| ST-22 | `select::fs_uae_version_for(Windows, <path of the ST-20 fixture>)` (file not executable) | `Ok((3, 2, 35))` | AR-14 |
| ST-23 | `fs_uae_version_for(Windows, …)` for a 3.1.66 fixture; for a file over 256 MiB | Error `older than the required 3.2.0`; error `cannot read the FS-UAE version from '<path>'` | AR-14, AR-58 |
| ST-24 | `setup::validate_for(Windows, <root with bin/m68k-amigaos-gcc.exe, m68k-amigaos/libnix, m68k-amigaos/ndk-include>, Selection)`; root without the `.exe` | Compiler path ends `m68k-amigaos-gcc.exe`; error contains `has no bin/m68k-amigaos-gcc.exe` | AR-1, AR-10 |
| ST-25 | `shell_word(Windows, "my app")`; `(Windows, "my-app")`; `(Unix, "it's")` | `"my app"`; `my-app`; `'it'\''s'` | AR-32 |
| ST-26 | `new::launcher_for(Windows, invoked = r"C:\P\current\bin\clorraine.exe")`; `(Windows, invoked = "clorraine", PATH = <dir containing clorraine.exe>)` | Exactly the invoked path; `<dir>/clorraine.exe` | AR-32 |
| ST-27 | Windows host: `clorraine setup --toolchain <rc24 root>` | Exit 0; `%APPDATA%\clorraine\config` holds `toolchain=C:\…` without `\\?\` | AR-38, AR-43 |
| ST-28 | Windows host with w64devkit GCC on PATH: `clorraine run hello.clo` | Prints `Hello, Amiga, from Clorraine!`; `build\clorraine\hello\host\hello.exe` exists | AR-29, AR-45 |
| ST-29 | Windows host, no compiler on PATH: `clorraine run hello.clo` | Exit 1 with the ST-16 Windows hint | AR-29 |
| ST-30 | Windows host: `clorraine run -p hosted-amiga --emulator --fs-uae <fs-uae.exe> --kickstart <rom> check\hello.clo` (backslash path) | Prints the hello line; exit 0 | AR-14, AR-16 |

### Language server and editor (03-03)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-31 | `uri::parse_for(Windows, "file:///c%3A/Users/a%20b/x.clo")` | `C:\Users\a b\x.clo` | AR-38 |
| ST-32 | `parse_for(Windows, "file:///C:/x/y.clo")` | `C:\x\y.clo` | AR-38 |
| ST-33 | `parse_for(Windows, "file://server/share/x.clo")` | Error `network-share paths are not supported` | AR-38 |
| ST-34 | `uri::from_path_for(Windows, r"C:\a b\x.clo")` | `file:///c%3A/a%20b/x.clo` | AR-38 |
| ST-35 | `normalize_for(Windows, r"\\?\c:\x\y.clo")`; `normalize_for(Unix, "/x/y.clo")` | `C:\x\y.clo`; `/x/y.clo` | AR-38 |
| ST-36 | `samePath("c:\\A\\b.clo", "C:\\a\\B.clo", "win32")`; same pair with `"linux"` | `true`; `false` | AR-38 |
| ST-37 | `installedToolPath("clorraine", "win32", {LOCALAPPDATA: "C:\\Users\\u\\AppData\\Local"}, "C:\\Users\\u")`; `("clorraine", "linux", {}, "/home/u")` | `C:\Users\u\AppData\Local\Programs\Clorraine\current\bin\clorraine.exe`; `/home/u/.local/bin/clorraine` | AR-38 |
| ST-38 | Windows host, VS Code with the installed extension, starter project: Build, then Run in FS-UAE | Both succeed (manual confirmation) | AR-38, AR-46 |

### Installer (03-04) — `install.spec.test.ps1`, on Windows

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-39 | `-Purge` alone; `-Toolchain x -NoToolchain`; `-FsUae x -NoEmulator`; `-Archive a` without `-Checksums` | Exit 2 each, with the usage text | AR-24 |
| ST-40 | Host architecture stubbed `Arm64`; OS build stubbed 19045 | Exit 1 `this installer supports 64-bit x86 Windows only`; line `Windows 10 is untested; continuing` and the run continues | AR-30 |
| ST-41 | Non-interactive `-Archive <fixture zip> -Checksums <SHA256SUMS> -Prefix <tmp> -NoToolchain` | Exit 0; `<tmp>\versions\<v>-<sha12>\bin\clorraine.exe` exists; `<tmp>\current` is a junction to it; examples written; user PATH unchanged | AR-10, AR-24, AR-26 |
| ST-42 | Same with a checksum file whose digest differs | Exit 1 naming the archive; no `versions\<v>-*`; no `current` | AR-36 |
| ST-43 | Fixture zip with an entry `../evil.txt` (checksum valid) | Exit 1 before extraction; nothing outside `<tmp>\versions\.tmp-*` written | AR-36 |
| ST-44 | Install version A, then version B | Both version folders exist; `current` targets B; A's files unchanged | AR-12 |
| ST-45 | Pre-state: `current` absent, `current.new` a junction into `versions` | After any run, `current` exists and targets that version; `current.new` absent | AR-12 |
| ST-46 | Another process holds mutex `Local\Clorraine-Install` | Exit 1 `another Clorraine installer is running` | AR-40 |
| ST-47 | Stubbed failure after extraction while A is active | `current` still targets A; no new version folder remains | AR-40 |
| ST-48 | Scratch key `Path` = ExpandString `%USERPROFILE%\bin;C:\y`; run with `-ModifyPath` twice; then `-Uninstall` | After install: kind ExpandString, `%USERPROFILE%\bin` unexpanded, `<tmp>\current\bin` present once; after uninstall the value equals the original | AR-26 |
| ST-49 | Scratch key containing `<TMP>\CURRENT\BIN\` (case and trailing `\` differ); `-Uninstall` | That entry is removed; others kept | AR-26 |
| ST-50 | Registration stub `DisplayVersion=16.2-rc24`, `InstallLocation=<root>`; non-interactive | No download, no elevation; `clorraine setup --toolchain <root>` invoked | AR-11, AR-50 |
| ST-51 | Registration stub `DisplayVersion=16.2-rc23`; non-interactive without/with `-ReplaceToolchain` | Exit 1 naming `16.2-rc23` and `-ReplaceToolchain`; with the flag the elevation stub is called | AR-11, AR-53 |
| ST-52 | No registration; non-interactive without/with `-InstallToolchain` | No download, the `-Toolchain` next step printed; with the flag the download stub receives the rc24 URL, 48065003 bytes, the pinned SHA-256, and target `C:\Program Files\Clorraine\amiga-gcc-16.2-rc24` | AR-1, AR-27 |
| ST-53 | Elevation stub throws Win32 error 1223 | The CLI stays installed; the `-Toolchain` next step is printed; exit 1 | AR-40 |
| ST-54 | The elevated staging function with a test admin root: staged copy whose SHA-256 differs; then a matching copy | Exit 101 and the setup stub never runs; then the setup stub receives `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /NOCLOSEAPPLICATIONS /SP- /DIR="<target>" /LOG="<staging>\setup.log"`, and the staging folder is gone afterwards | AR-34, AR-60 |
| ST-55 | FS-UAE candidates stubbed present at several places | Chosen in order: `-FsUae`, `CLORRAINE_FS_UAE`, config, PATH, `<Prefix>\fs-uae\3.2.35\…`, `%ProgramFiles%\FS-UAE\…` | AR-28 |
| ST-56 | No FS-UAE; non-interactive without/with `-InstallFsUae` | No download; with the flag the download stub receives the FS-UAE URL, 49123213 bytes, the pinned SHA-256, and the zip is extracted to `<Prefix>\fs-uae\3.2.35` | AR-13, AR-27 |
| ST-57 | Fake home with `amiga\roms\KICK31A1200.ROM` (524,299 bytes) and `amiga\roms\kick13.rom` (262,144 bytes) | The first is chosen; the second never | AR-28 |
| ST-58 | Compiler probe stubbed: Clang only; none | The `Clang found; the PC hello needs GCC (w64devkit or MSYS2), or set CLORRAINE_CC` note, step skipped; the ST-16 Windows hint, step skipped; neither fails the run | AR-51, AR-29 |
| ST-59 | `-Uninstall` with an owned `current`; with `current` targeting a folder outside `versions` | PATH entry and `current` removed, the version tree intact; exit 1 and nothing deleted | AR-35 |
| ST-60 | `-Uninstall -Purge` where `versions\x\link` is a junction to an outside folder holding `keep.txt`, and `fs-uae\3.2.35` exists | `versions`, `examples`, `fs-uae\3.2.35`, and the config removed; `keep.txt` still exists; registration stub untouched | AR-35, AR-52 |
| ST-61 | `-Uninstall -Purge` while a process runs from `<tmp>\versions\…\bin` | Exit 1 `close VS Code and running Clorraine builds, then retry`; nothing deleted | AR-41 |
| ST-62 | The unassembled template without `-Archive`; the assembled installer's bytes | Exit 1 `this installer template is not assembled`; every byte is ASCII | AR-31, AR-33 |
| ST-63 | Style stubs: no virtual terminal; VT with code page 437; VT with 65001 and `NO_COLOR=1` | `[ok]`/`[!!]` and no escape bytes; `[ok]` and colour; glyphs and no colour | AR-31 |

### Packaging and acceptance (03-05)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-64 | `package.sh` with Linux and Windows fixture stages | Five assets; `SHA256SUMS` has exactly five lines matching the files | AR-33 |
| ST-65 | The Windows zip | Exactly the 03-05 file set under `clorraine-<v>-x86_64-windows/`, forward slashes, sorted | AR-33 |
| ST-66 | Windows stage missing `bin/clc.exe`; stage with an extra file | Exit 1 naming the file (each) | AR-33 |
| ST-67 | An ELF file named `clorraine.exe`; a PE32+ without the version bytes | Exit 1 (each) | AR-33 |
| ST-68 | Assembled `install.ps1`; Amiga hello source with a line starting `'@` | No `@…@` token, ASCII only, `$Version = '<v>'`; exit 1 for the hello | AR-31, AR-33 |
| ST-69 | `package.sh --verify` on the candidate; after changing one zip byte | Passes; fails naming the zip | AR-33 |
| ST-70 | `acceptance.py` with a valid Windows record; with the committed Linux fixture record | Pass; pass | AR-46 |
| ST-71 | Windows record with the Linux rc24 pin; `os_build=19045`; missing `manual.uac_prompt_seen`; `platform=macos` | Fail naming `toolchain`, `os_build`, `manual`, `platform` | AR-46 |
| ST-72 | Record `version` differs from the candidate's archive version | Fail naming `version` | AR-46 |
| ST-73 | `acceptance.ps1` on the user's Windows 11 machine, standard account, then `make acceptance-check` | The record passes | AR-46, AR-49 |
| ST-74 | `make site-check`; `docs_guide_spec_test` | Pass | W17 |

## Test Categories

### Specification Tests

| Test File | ST Cases Covered | Component |
|---|---|---|
| `compiler/tests/windows_cli_spec_test.rs` | ST-3 – ST-26, ST-31 – ST-35 | 03-01, 03-02, 03-03 server |
| `tools/vscode/test/windows.spec.test.ts` | ST-36, ST-37 | 03-03 extension |
| `tools/setup/tests/install.spec.test.ps1` | ST-39 – ST-63 | 03-04 |
| `tools/setup/tests/windows-dist.spec.test.sh` | ST-64 – ST-69 | 03-05 packaging |
| `tools/setup/tests/acceptance_windows_spec.py` | ST-70 – ST-72 | 03-05 evidence |
| Command checks in the phase gates | ST-1, ST-2, ST-74 | 03-01, 03-05 |
| Windows driver smoke (helper) | ST-27 – ST-30 | 03-02 |
| Windows acceptance | ST-38, ST-73 | all |

### Implementation Tests

| Test File | Description | Priority |
|---|---|---|
| `compiler/tests/windows_cli_impl_test.rs` | Escaping of every byte class, PE parser bounds, verbatim forms, compiler resolution errors, launcher fallbacks | High |
| `tools/vscode/test/windows.impl.test.ts` | Drive-letter and separator variants, discovery fallbacks | Med |
| `tools/setup/tests/install.impl.test.ps1` | Zip validation internals, reparse walker, mutex abandonment, PATH normalization, encoded elevated command | High |
| `tools/setup/tests/windows-dist.impl.test.sh` | Zip builder ordering, PE header checks, here-string guard | Med |
| `tools/setup/tests/acceptance_windows_impl.py` | Per-platform pin tables, version extraction from archive names | Med |
| Existing pinned-shim impl tests | Updated only for the AR-44 shim text | High |

### Integration Tests

| Test | Components | Description |
|---|---|---|
| ST-17, ST-18 | driver + toolchain | `CLORRAINE_CC` end to end on Linux |
| ST-30 | driver + emulator + emission | Backslash path through `#line` and FS-UAE on Windows |

### End-to-End Tests

| Scenario | Steps | Expected Result |
|---|---|---|
| Existing registration | Spike rc24 present; acceptance install | Reused (ST-50 on the real host) |
| Clean install, standard account | Uninstall the spike rc24; `Clorraine Test` runs the installer; admin approves UAC | ST-73 passes |
| Editor | Starter project Build and Run in FS-UAE | ST-38 |

## Test Data

### Fixtures Needed
- A minimal PE32+ file with a `VS_FIXEDFILEINFO` (3.2.35 and 3.1.66 variants), generated by the
  test, plus an ELF file.
- Fixture Linux and Windows stage trees; fixture zips (valid, `..` entry); `SHA256SUMS` files.
- A Windows evidence record and the existing Linux record.
- Fake ROM files of 524,299 and 262,144 bytes (zeros; never real ROMs).

### Mock Requirements
- PowerShell: dot-source with `CLORRAINE_INSTALL_LIB=1` and replace the download, elevation,
  registration, and compiler-probe functions; a scratch registry key for PATH.
- Rust: none; pure functions take the OS and environment as parameters.

## Verification Checklist
- [ ] All ST cases defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement or AR entry
- [ ] Specification tests written before implementation
- [ ] Specification tests fail before implementation (red phase)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written
- [ ] `make check IMAGE=clorraine-sdk:windows-cli`, the PowerShell suites, `make verify`, and the
      Windows acceptance pass
- [ ] No Linux regression
