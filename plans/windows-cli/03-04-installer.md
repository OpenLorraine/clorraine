# Installer: Windows CLI and installer

> **Document**: 03-04-installer.md
> **Parent**: [Index](00-index.md)

## Overview

`tools/setup/install.ps1` is the Windows counterpart of `tools/setup/install.sh`: a Windows
PowerShell 5.1 script, ASCII-only, assembled by `package.sh` with the release pins, run with a
process-scope execution-policy bypass (AR-6, AR-7). It follows the same five steps and the same
non-interactive rules as Linux.

## Architecture

### Layout (AR-10, AR-25)

| Path | Contents |
|---|---|
| `<Prefix>` | default `%LOCALAPPDATA%\Programs\Clorraine` |
| `<Prefix>\versions\<version>-<sha12>\` | `bin\clc.exe`, `bin\clorraine.exe`, `bin\clorraine-lsp.exe`, notices, `README.md` |
| `<Prefix>\current` | junction to the active version |
| `<Prefix>\examples\` | `hello.clo`, `hello-amiga.clo` |
| `<Prefix>\fs-uae\3.2.35\` | the FS-UAE zip, when the installer downloaded it (AR-13, AR-28) |
| user PATH entry | `<Prefix>\current\bin` (no shims) |
| `%APPDATA%\clorraine\config` | written by `clorraine setup` (AR-43) |
| `C:\Program Files\Clorraine\amiga-gcc-16.2-rc24` | rc24, per-machine (AR-10) |

### Parameters (AR-24)

| install.sh | install.ps1 | Notes |
|---|---|---|
| `--prefix DIR` | `-Prefix DIR` | absolute; no `;`, `"`, or control characters |
| `--modify-path` | `-ModifyPath` | AR-26 |
| `--archive F --checksums F` | `-Archive F -Checksums F` | both or neither |
| `--toolchain PATH` | `-Toolchain PATH` | existing rc24 root |
| `--no-toolchain` | `-NoToolchain` | skips steps 2–4 |
| — | `-InstallToolchain` | non-interactive consent to rc24 (AR-27) |
| — | `-ReplaceToolchain` | replace another amiga-gcc registration (AR-11, AR-53) |
| `--fs-uae PATH` | `-FsUae PATH` | |
| `--kickstart PATH` | `-Kickstart PATH` | |
| `--no-emulator` | `-NoEmulator` | |
| — | `-InstallFsUae` | non-interactive consent to the FS-UAE zip (AR-27) |
| `--no-hello` | `-NoHello` | |
| `--starter DIR` / `--no-starter` | `-Starter DIR` / `-NoStarter` | |
| `--vscode` / `--no-vscode` | `-VSCode` / `-NoVSCode` | |
| `--uninstall` / `--purge` | `-Uninstall` / `-Purge` | `-Purge` requires `-Uninstall` |
| `-h`, `--help` | `-Help` | |

Conflict rules mirror `install.sh:1385-1407`: `-Uninstall` combines only with `-Prefix` and
`-Purge`; `-Toolchain` conflicts with `-NoToolchain` and `-InstallToolchain`; `-FsUae`,
`-Kickstart`, and `-InstallFsUae` conflict with `-NoEmulator`. A conflict is a usage error, exit 2.

Interactive mode is on when stdin and stdout are not redirected
(`[Console]::IsInputRedirected`/`IsOutputRedirected` both false), as on Linux.

## Implementation Details

### Flow

1. **Host check (AR-30).** `RuntimeInformation.OSArchitecture` must be `X64`, else exit 1
   `this installer supports 64-bit x86 Windows only`. A build below 22000 prints
   `Windows 10 is untested; continuing`. Requires `curl.exe` and `tar.exe` in `System32`.
2. **Lock and repair (AR-12, AR-40).** Acquire the named mutex `Local\Clorraine-Install`
   (non-blocking; an abandoned mutex counts as acquired); otherwise exit 1
   `another Clorraine installer is running`. Then repair an interrupted switch: `current` missing
   and an owned `current.new` present → rename it to `current`; both present → remove
   `current.new`.
3. **Step 1, install the CLI.** Download (or copy `-Archive`) into a fresh private temp folder,
   verify (AR-36), validate the zip (every entry under `clorraine-<v>-x86_64-windows/`, no
   absolute, drive, or `..` paths, exactly the expected file set), extract with `tar.exe` into
   `versions\.tmp-<random>`, run `bin\clorraine.exe --version` and compare, rename to
   `versions\<version>-<sha12>`, then switch: create junction `current.new`, delete the old
   `current` junction (non-recursive, AR-35), rename. Write the examples. PATH per AR-26.
4. **Step 2, rc24 (AR-1, AR-11, AR-34, AR-40, AR-53).** Order: `-Toolchain`; the existing
   registration `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{8D184346-D038-45E9-98A1-43BFC542BE34}_is1`
   with `DisplayVersion` `16.2-rc24` → its `InstallLocation`; a registration of another version →
   ask (default No) or `-ReplaceToolchain`, non-interactive without the flag exits 1; none →
   interactive offer (default yes) or `-InstallToolchain`.
   The elevated install is described under §Elevated rc24 step.
5. **Step 3, emulator (AR-13, AR-28).** FS-UAE: `-FsUae`, `CLORRAINE_FS_UAE`, the config, PATH,
   `<Prefix>\fs-uae\3.2.35\FS-UAE\Windows\x86-64\fs-uae.exe`,
   `%ProgramFiles%\FS-UAE\…\fs-uae.exe`. None → offer the pinned zip (consent or
   `-InstallFsUae`), verify, validate, extract into `<Prefix>\fs-uae\3.2.35`. ROM: `-Kickstart`,
   `CLORRAINE_KICKSTART`, the config, `~\amiga\roms`, `Documents\FS-UAE\Kickstarts`,
   `%PUBLIC%\Documents\Amiga Files\Shared\rom`. Within those folders the same exact names and
   then the same patterns as `install.sh:761-803` (`amiga-os-310-a1200.rom`, `kick31a1200.rom`,
   `kick31.rom`; `*a1200*.rom`, `kick31*.rom`; case-insensitive), accepting only 524,288 or
   524,299 bytes; otherwise a prompt. Then `current\bin\clorraine.exe setup --toolchain …
   [--kickstart …] [--fs-uae …]`.
6. **Step 4, hellos (AR-2, AR-29, AR-51).** PC hello when the AR-29 compiler order finds GCC or
   `cc` that is not Clang (detected with `--version`); Clang only → the AR-51 note, skipped; none →
   skipped with the Windows hint. Amiga hello: `clorraine run -p hosted-amiga --release
   --emulator <examples>\hello-amiga.clo`.
7. **Step 5, editor and starter (AR-32).** VS Code: `code.cmd`, `codium.cmd`,
   `code-insiders.cmd` on PATH or under `%LOCALAPPDATA%\Programs\Microsoft VS Code\bin`; the VSIX
   is downloaded and verified like the archive, then `--install-extension`. Starter:
   `current\bin\clorraine.exe new <dir>` (default `amiga-starter`), optionally opened in VS Code.
8. **Summary.** Windows next steps: open a new terminal (the PATH applies to new windows), restart
   VS Code, the three everyday commands.

### Elevated rc24 step (AR-34)

- Download into the private temp folder and verify there (early failure only).
- Start `<System32>\WindowsPowerShell\v1.0\powershell.exe -NoProfile -ExecutionPolicy Bypass
  -EncodedCommand <script>` through `Process.Start` with `Verb = runas`, then wait and read the
  exit code. The script text is the staging function from `install.ps1`; the only data passed
  are the source path and the pinned bytes and SHA-256.
- An existing admin folder must be a real folder owned by Administrators, SYSTEM, or
  TrustedInstaller, with no write-type right for any other account; otherwise 103 (AR-61).
- The elevated script creates `C:\Program Files\Clorraine` if missing, creates a new
  `.staging-<random>` folder there (failing if it exists), copies the setup in, verifies size and
  SHA-256 there, runs it with `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /NOCLOSEAPPLICATIONS /SP-
  /DIR="<target>" /LOG="<staging>\setup.log"`, copies the log to
  `C:\Program Files\Clorraine\rc24-setup.log`, deletes the staging folder, and exits with the
  setup's exit code (a mismatch exits 101 without running; 102 staging name taken, 103 untrusted
  admin folder, 104 other error; AR-60). The admin folder and the target come from the system's
  Program Files known folder inside the elevated step, never from the environment.
- A cancelled UAC prompt (Win32 error 1223) or a nonzero exit keeps the installed CLI, prints the
  `-Toolchain` next step, and exits 1 (AR-40).

### PATH (AR-26)

- Read `HKCU\Environment` `Path` with `DoNotExpandEnvironmentNames` and keep its value kind
  (`ExpandString` when it was, or when it contains `%`).
- Entries are compared after trimming a trailing `\` and ignoring case. Add `<Prefix>\current\bin`
  once; on uninstall remove exactly the entries equal to it. Never `setx`.
- Broadcast `WM_SETTINGCHANGE` ("Environment") with a 5-second timeout.
- Interactive: ask (default yes). Non-interactive: only with `-ModifyPath`.
- The registry key is a function parameter so tests use a scratch key.

### Uninstall and purge (AR-17, AR-35, AR-41, AR-52)

- `-Uninstall`: remove the PATH entry; remove `current` (and `current.new`) only when it is a
  junction whose target resolves inside `<Prefix>\versions`, non-recursively; print how to remove
  rc24 (its Windows uninstaller) and any FS-UAE not installed by Clorraine.
- `-Purge`: first stop if any process image lies under `<Prefix>` (`close VS Code and running
  Clorraine builds, then retry`); then delete `versions`, `examples`, `fs-uae\3.2.35`, and
  `%APPDATA%\clorraine\config` (and its folder when empty), with a walker that removes reparse
  points without following them.

### Presentation (AR-31)

- The file is ASCII. Glyphs (`✔ ✘ ▸ ▶`) are built from `[char]` codes and used only when the
  console supports virtual terminal sequences and `[Console]::OutputEncoding.CodePage` is 65001;
  colour only with virtual terminal support and no `NO_COLOR`; otherwise `[ok]`, `[!!]`, `>`.

### Template and test seam

- Pins `@VERSION@`, `@ARCHIVE_NAME@`, `@ARCHIVE_URL@`, `@ARCHIVE_BYTES@`, `@ARCHIVE_SHA256@`,
  `@VSIX_NAME@`, `@VSIX_URL@`, `@VSIX_BYTES@`, `@VSIX_SHA256@`, and `@HELLO_AMIGA_SOURCE@` as in
  `install.sh`; rc24 and FS-UAE pins are constants (AR-1, AR-13). The unassembled template's
  network paths refuse to run; `-Archive` works.
- `$env:CLORRAINE_INSTALL_LIB = '1'` before dot-sourcing defines the functions without running
  `Main`, so tests replace `Invoke-Download`, `Invoke-ElevatedSetup`, `Get-Rc24Registration`, and
  the registry key (AR-48).

### Exit codes
`0` success; `1` failure (verification, toolchain, setup, lock, refused host); `2` usage error.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| Size or SHA-256 mismatch | Delete the file; exit 1 naming the asset | AR-36 |
| Zip entry escapes the version folder | Exit 1 before extraction | AR-36 |
| UAC cancelled / setup fails | Keep the CLI, print the `-Toolchain` step, exit 1 | AR-40 |
| Other amiga-gcc registered, non-interactive | Exit 1 naming the version and `-ReplaceToolchain` | AR-11, AR-53 |
| Installer already running | Exit 1 | AR-40 |
| Interrupted mid-install | `finally` restores the previous `current` and removes the partial version | AR-40 |
| Purge while programs run | Exit 1 before deleting anything | AR-41 |
| `current` is not an owned junction | Refuse to delete it; exit 1 | AR-35 |
| Group Policy blocks scripts | Not reachable by the script; documented (03-05) | AR-6 |

## Testing Requirements
- ST-39 – ST-62 in [07-testing-strategy.md](07-testing-strategy.md):
  `tools/setup/tests/install.spec.test.ps1` and `install.impl.test.ps1` (AR-48).
