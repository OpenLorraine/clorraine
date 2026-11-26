# Packaging, Acceptance, and Docs: Windows CLI and installer

> **Document**: 03-05-packaging-acceptance-docs.md
> **Parent**: [Index](00-index.md)

## Overview

Extend the local candidate to five assets, extend the acceptance evidence to Windows, add the
non-gating remote helper, and document the Windows path. Nothing is published (AR-3).

## Architecture

### Current Architecture
- `make dist` builds Linux release binaries in the image, stages
  `clorraine-<v>-x86_64-linux/`, and calls `package.sh`, which writes the archive, the VSIX copy,
  `install.sh`, and a three-line `SHA256SUMS` (`package.sh:51-60, 191`).
- `acceptance.py` accepts Linux records only (`acceptance.py:21-23, 209-213`).

### Proposed Changes

#### Candidate (AR-33)

| Asset | Name |
|---|---|
| Linux archive | `clorraine-<v>-x86_64-linux.tar.gz` (unchanged) |
| Windows archive | `clorraine-<v>-x86_64-windows.zip`, top folder `clorraine-<v>-x86_64-windows/` with `bin\{clc,clorraine,clorraine-lsp}.exe`, `LICENSE`, `LICENSE-EXCEPTION.md`, `NOTICE`, `THIRD_PARTY_NOTICES.md`, `README.md` |
| VSIX | `clorraine-<v>.vsix` (unchanged) |
| Installers | `install.sh`, `install.ps1` |
| Checksums | one `SHA256SUMS` over the five assets above |

- `make dist` also runs `cargo build --release --workspace --target x86_64-pc-windows-gnu` in the
  image and stages the Windows tree.
- `package.sh --windows-stage DIR`: checks the exact file set; checks each `.exe` is PE32+ for
  machine `0x8664` and contains the bytes of the version string; builds the zip with Python
  `zipfile` (entries sorted, forward slashes); assembles `install.ps1` from its template with the
  ten pins (03-04 §Template); rejects a non-ASCII byte or a leftover `@…@` token; and writes the
  five-line `SHA256SUMS`. `--verify` checks all five.
- The Amiga hello is embedded in `install.ps1` as a single-quoted here-string; `package.sh`
  rejects a source line that would end it (`'@` at line start).

#### Acceptance evidence (AR-46)
- `acceptance.py` becomes platform-keyed. Shared core: `version` (read from the candidate's
  archive names, not a constant), `platform`, `architecture`, `archive`, `installer`, `vsix`,
  `toolchain`, `fs_uae`, `kickstart`, `guest`, `negative`, `corpus`.
- Linux keeps today's keys and pins, including `glibc`; existing Linux evidence keeps validating.
- Windows: `platform: windows`, `os_build` (integer, at least 22000), the rc24 setup pin
  (48,065,003 bytes, `9dd69735…5512`), `installer.name: install.ps1`, the `.zip` archive,
  `fs_uae.source` (`downloaded` with the zip pin, or `existing`), `host_hello` (`compiler`,
  `result`), `existing_registration` (`reused`, AR-50), `standard_account` (`installed`),
  `manual` (`uac_prompt_seen`, `editor_build_run`), all required.
- `tools/setup/acceptance.ps1 -Candidate DIR -Kickstart ROM -Output FILE [-UacConfirmed]
  [-EditorConfirmed]` runs on Windows in the desktop session. It installs the candidate with
  `-Archive`/`-Checksums` into an isolated prefix and isolated `APPDATA` (AR-43), runs the
  fs-uae-check set (hello, four `hosted-amiga` corpus programs, `file-copier` on
  `hosted-amiga-13`, the return-1 negative), the host hello with GCC, records hashes and results,
  and writes the JSON. ROMs are recorded by name and SHA-256 only.
- `make acceptance-check EVIDENCE=… CANDIDATE=…` validates either platform on Linux.

#### Remote helper (AR-21, W18)
- `tools/setup/windows-remote.sh push HOST DIR` copies a candidate (no ROMs) to
  `%USERPROFILE%\clorraine-remote\`; `run HOST -- COMMAND` runs a `cmd.exe` command line in the
  signed-in desktop session through `tools/setup/windows-desktop-run.ps1` (a one-shot interactive
  scheduled task) and prints its output and exit code. It is not called by any Makefile target.

#### Documentation (AR-6, W17)
- `docs/windows-cli.md`, structured like `docs/linux-cli.md`: install, requirements, what goes
  where, everyday commands, configuration, emulator runs, VS Code, uninstall, limitations.
- Install command:
  `curl.exe -fsSLo clorraine-install.ps1 <release>/install.ps1; powershell -ExecutionPolicy Bypass -File .\clorraine-install.ps1`,
  plus the verified path with `SHA256SUMS` and `Get-FileHash`.
- The execution-policy section: process-scope bypass only, the Group Policy message, the rc24 UAC
  prompt, `Unblock-File` for browser downloads; never a permanent policy change (AR-6).
- Limitations (also in `docs/known-limitations.md`): Clang debug builds (AR-15), SSH logons
  (AR-19), interrupted emulator runs (AR-39), UNC/long paths/commas in `TEMP` (AR-38, AR-42),
  Windows 10 untested (AR-30), unsigned binaries (AR-18).
- `README.md` and `site/` show the Windows command beside the Linux one; `make site-check` passes.
- `AGENTS.md` gains the Windows commands and the image note at merge time (AR-37).
  `CHANGELOG.md`, `RELEASE-NOTES.md`, and `RELEASE.md` change with the release (AR-3).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| A Windows stage file is missing or extra | `package.sh` exits 1 naming it | AR-33 |
| An `.exe` is not PE32+ x86-64 or lacks the version | `package.sh` exits 1 | AR-33 |
| `install.ps1` has a non-ASCII byte or an unreplaced pin | `package.sh` exits 1 | AR-31, AR-33 |
| Evidence for an unknown platform or with a wrong pin | `acceptance-check` exits 1 with the key | AR-46 |
| The remote host is unreachable | The helper exits nonzero; no gate depends on it | AR-21 |

## Testing Requirements
- ST-63 – ST-72 in [07-testing-strategy.md](07-testing-strategy.md).
