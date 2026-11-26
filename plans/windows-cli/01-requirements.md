# Requirements: Windows CLI and installer

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Evidence**: `spikes/08-windows-cli/README.md`

## Feature Overview

A Windows 11 x64 user installs the Clorraine CLI with one PowerShell installer, without Docker or
a Windows build environment. The installer follows the same five steps as the Linux installer:
install the CLI, set up the pinned rc24 toolchain, find or install FS-UAE and the Kickstart ROM,
run the PC and Amiga hellos, and install the VS Code extension and a starter project. The driver,
the language server, and the editor work with Windows paths. Every Windows binary is cross-built
on Linux in the pinned SDK image, and acceptance is recorded on a real Windows 11 machine.

## Functional Requirements

### Must Have

| ID | Requirement | AR Ref |
|---|---|---|
| W1 | `install.ps1` installs the versioned CLI zip per-user, without administrator rights, Docker, or a build environment, into the AR-10 layout, with the AR-24 options and the AR-25 PATH entry (no shims). | AR-7, AR-10, AR-24, AR-25 |
| W2 | The installer refuses non-x64 hosts, warns and continues on Windows 10, and runs under Windows PowerShell 5.1 (PowerShell 7 untested). | AR-9, AR-30 |
| W3 | Downloads use `curl.exe` with HTTPS-only redirects, a size cap, and timeouts, and verify the pinned size and SHA-256 before use; a mismatch deletes the file and stops. `-Archive`/`-Checksums` installs local bytes as on Linux. | AR-33, AR-36 |
| W4 | The rc24 step installs the pinned upstream setup silently behind one UAC prompt, verifying it inside the elevated step from an admin-only folder; an existing rc24 registration is reused; another version is never replaced without an explicit yes. A declined prompt keeps the CLI and exits 1. | AR-1, AR-11, AR-34, AR-40 |
| W5 | The installer detects FS-UAE and the ROM from the AR-28 candidate lists, and offers the pinned FS-UAE 3.2.35 zip with consent. Non-interactive downloads need `-InstallToolchain`/`-InstallFsUae`. | AR-13, AR-27, AR-28 |
| W6 | The installer runs the PC hello when a GCC-compatible compiler is found (skipped otherwise) and the Amiga hello in FS-UAE, installs the extension, and offers a starter project. | AR-2, AR-29, AR-32 |
| W7 | Updates install side-by-side and switch the `current` junction by delete-then-rename; the next run finishes an interrupted switch. A named mutex serializes runs; an interrupted run rolls back. | AR-12, AR-40 |
| W8 | `-Uninstall` removes the `current` junction and the PATH entry; `-Purge` also removes the versions and the config; neither removes rc24 or FS-UAE; deletion never follows a reparse point; purge stops while Clorraine programs run from the tree. | AR-17, AR-25, AR-35, AR-41 |
| W9 | The user PATH is edited in the registry exactly and reversibly, with consent. | AR-26 |
| W10 | `install.ps1` is ASCII-only, with the AR-31 glyph and colour fallback. | AR-31 |
| W11 | The driver works on Windows: `.exe` lookups, `%APPDATA%` config, display paths without `\\?\`, the FS-UAE version from the PE resource, `.exe` host artifacts, the AR-29 host-compiler order, and the `clorraine new` launcher. Windows-specific logic is testable on Linux. | AR-10, AR-14, AR-21, AR-29, AR-32, AR-43, AR-45 |
| W12 | `#line` paths are escaped as C string literals on every host. | AR-16 |
| W13 | The freestanding profile runs on Windows through the C-runtime fallback, with the shim guards. | AR-23, AR-44 |
| W14 | The language server and the extension agree on Windows paths; the extension finds the installed CLI on Windows. | AR-38 |
| W15 | The pinned SDK image cross-builds the Windows binaries; `make check` compiles the Windows target; `make dist` adds the Windows zip and `install.ps1` under one `SHA256SUMS`. | AR-20, AR-33, AR-37 |
| W16 | Windows acceptance is recorded by `tools/setup/acceptance.ps1` and validated on Linux by `make acceptance-check`. | AR-46, AR-49, AR-50 |
| W17 | The docs, site, and README document the Windows install, the execution-policy bypass, the UAC prompt, and the Windows limitations. | AR-6, AR-15, AR-19, AR-39, AR-42 |

### Should Have

| ID | Requirement | AR Ref |
|---|---|---|
| W18 | A non-gating helper copies a candidate to a Windows host and runs commands in its desktop session. | AR-21 |

### Won't Have (Out of Scope)

- Publication, tagging, or upload; a separate approval (AR-3).
- MSVC, Windows on ARM, WSL, Windows 10 support claims (AR-9, scope confirmation).
- Code signing (AR-18).
- Running the CLI from an SSH logon (AR-19).
- Clang debug builds on Windows; a follow-up issue (AR-15).
- A GitHub Actions Windows job or a `WINHOST` Makefile gate (AR-21).
- UNC or network-share workspaces, paths over 260 characters, commas in `TEMP` (AR-38, AR-42).
- Killing FS-UAE when a run is interrupted (AR-39).

## Technical Requirements

### Performance
- An emulator run keeps the 90-second default timeout; the spike measured 7–9 s per run.

### Compatibility
- Linux behaviour, assets, and evidence keep working unchanged, except the `#line` escaping
  (AR-16) and the `CLORRAINE_CC` override (AR-29).
- Windows 11 x64, Windows PowerShell 5.1, the pinned rc24 setup and FS-UAE 3.2.35 (AR-1, AR-13).

### Security
- The rc24 setup is verified in an admin-only folder by the elevated step (AR-34).
- No download is used before its size and SHA-256 match (AR-36).
- Deletion never follows a reparse point (AR-35); PATH edits are exact and reversible (AR-26).
- Licensed ROMs are never copied off the machine, bundled, or downloaded.

## Scope Decisions

The register owns every decision; see AR-1 – AR-50 in
[00-ambiguity-register.md](00-ambiguity-register.md).

## Acceptance Criteria

1. [ ] The ST cases in [07-testing-strategy.md](07-testing-strategy.md) pass: Rust and Linux suites
       in `make check IMAGE=clorraine-sdk:windows-cli`, the PowerShell suites on Windows (AR-47).
2. [ ] `make verify` passes on Linux.
3. [ ] A Windows acceptance record from the user's Windows 11 machine passes
       `make acceptance-check` (AR-46).
4. [ ] The user confirms the UAC prompt and an editor Build/Run in person (AR-46).
5. [ ] The docs, site, and README describe the Windows path (W17).
