# Requirements: Cross-platform CLI and FS-UAE delivery

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Clorraine must be usable as a normal command-line tool on Linux, macOS and
Windows. A user installs the CLI, runs one setup command that obtains the Amiga
toolchain, then builds programs and runs them in FS-UAE. The VS Code extension
uses the same installed CLI. No Docker, no Python runtime and no self-hosted
release runner are required.

## Functional Requirements

### Must Have

| ID | Requirement | AR |
|---|---|---|
| R1 | Ship `clc`, `clorraine` and `clorraine-lsp` for Linux x64 in v1.2.0. | AR-2/27 |
| R2 | Provide `install.sh` for Linux v1.2.0. It downloads or accepts a local CLI archive, verifies its hash, installs it and invokes `<home>/bin/clorraine setup` by absolute path. macOS/Windows installers belong to the follow-up plan. | AR-4/5/7/28/31 |
| R3 | `clorraine setup` validates a user-supplied or detected upstream AmigaPorts toolchain (`--toolchain PATH`), records it, discovers FS-UAE/ROM paths, and verifies it by building an Amiga program. Clorraine does not download or install the toolchain. | AR-3/26 |
| R4 | Support `--toolchain PATH` for a user-installed toolchain, `--fs-uae PATH` and `--kickstart PATH` overrides. | AR-5/11/12 |
| R5 | Store installation state in one portable home (`~/.clorraine`, or `%LOCALAPPDATA%\Clorraine` on Windows) and write a simple config file. | AR-6/19 |
| R6 | On Windows, add the Clorraine `bin` to the user PATH automatically. On Linux/macOS print the export line and support an opt-in `--modify-path`. | AR-7 |
| R7 | Build `host`, `hosted-amiga` and `hosted-amiga-13` programs from the CLI. | R1/R7 existing |
| R8 | Run Amiga programs in FS-UAE with a user Kickstart ROM, or FS-UAE's AROS ROM with a warning when no ROM is configured. | AR-11/12 |
| R9 | Plain `clorraine run` on an Amiga profile uses `vamos` only if it is already on `PATH`; otherwise explain the FS-UAE and amitools options. | AR-18 |
| R10 | VS Code discovers the installed CLI and provides Build and Run in FS-UAE; it does not bundle a compiler or duplicate toolchain settings. | AR-13/21 |
| R11 | Release v1.2.0 through a GitHub-hosted Linux build producing the CLI archive, `SHA256SUMS`, install scripts and a VSIX. No self-hosted runner or Docker. Follow-up releases add macOS/Windows. | AR-22/27 |
| R12 | Keep version `1.2.0`; v1.2.0 is Linux x64 only. macOS arm64 and Windows x64 are a separate follow-up plan. | AR-8/17/27 |
| R13 | Remove the superseded Python installer, support-lock/support pipeline and release workflows once the new Linux path is proven. | AR-16/25 |
| R14 | Keep the existing compiler, language server, ownership model, Amiga ABI and Docker development gates unchanged. | AR-1/10 |

### Should Have

| ID | Requirement | AR |
|---|---|---|
| R15 | Linux capability check accepts x86_64, glibc >= 2.35, `curl`, `tar`; tested on Ubuntu 22.04/24.04 and Debian 12. WSL is untested/unsupported. | AR-9/32 |
| R16 | Clear, actionable failure messages for missing toolchain, FS-UAE, ROM, host compiler and unsupported platform. | AR-9/10/23 |
| R17 | Re-running the installer updates the CLI and reuses an unchanged toolchain. | AR-19 |

### Won't Have (Out of Scope)

- Docker as a user prerequisite (AR-25).
- Our own toolchain build, support-lock pipeline or binary mirror (AR-3/20).
- Bundled `vamos`/Python wheels or a Python runtime requirement (AR-18).
- WSL, Windows ARM, Linux ARM, macOS and Intel Mac support in the first
  release; macOS arm64/Windows x64 are a separate follow-up plan (AR-27/32).
- Bundled FS-UAE, Kickstart ROMs or Workbench media (AR-11/12).
- Marketplace/Open VSX publishing, code signing, Homebrew/Scoop/winget manifests
  (AR-13/15).
- Automatic launch of a Windows-side FS-UAE from a Linux/WSL host; WSL is not
  supported (AR-32).

## Technical Requirements

### Performance

- Setup installs the CLI archive; the user supplies the toolchain. No toolchain compilation.
- CLI build/run startup adds no new daemon or service.
- Downloads have finite connect/overall timeouts and bounded memory.

### Compatibility

- Rust stable, std-only workspace; no new crates.
- Linux x64 for this plan; macOS arm64/Windows x64 are follow-up scope.
- The upstream toolchain must pass the existing corpus through a separate
  upstream-toolchain leg; `make verify` remains the Docker SDK gate and does not
  by itself prove the upstream archive.
- FS-UAE user install; no emulator bundling.

### Security

- HTTPS only, redirect schemes checked, exact pinned size and SHA-256 before
  unpacking.
- Safe archive extraction: reject absolute paths, `..`, links that escape, and
  unsupported member types.
- No shell evaluation of URLs, paths or config values; literal process argv.
- Config file contains paths only; no tokens, ROMs or secrets.
- A failed/partial setup leaves no active config and no half-unpacked toolchain.
- Unsigned binaries are accepted for the first release; documents state the
  macOS/Windows warnings.

## Scope Decisions

| Decision | Options | Chosen | Rationale | AR |
|---|---|---|---|---|
| Toolchain source | build / upstream / user-provided | upstream | Least work; no NDK redistribution; prebuilt for all targets | AR-3 |
| Installer | Python installer / CLI setup / scripts | CLI setup | One binary, no Python, same behavior everywhere | AR-5 |
| Amiga run | vamos / FS-UAE / both | FS-UAE primary, optional vamos | Matches the 99% workflow | AR-11/18 |
| Platforms | Linux only / all three | Linux first, then macOS/Windows | Prove the new path before expanding | AR-2/8 |
| Extension | bundled tools / CLI client | CLI client | One installation, editor-independent | AR-13 |
| Release | self-hosted / hosted matrix | hosted matrix | No runner infrastructure | AR-22 |

## Acceptance Criteria

1. [ ] Linux clean install, build and FS-UAE run pass with real evidence.
2. [ ] macOS arm64 and Windows x64 follow-up builds are deferred to a separate
   plan; no unverified platform assets are attached to v1.2.0.
3. [ ] `clorraine setup` handles valid/invalid toolchain paths, missing
   FS-UAE/ROM, invalid config and a failed smoke.
4. [ ] VS Code Build/Run uses the installed CLI and reports missing CLI clearly.
5. [ ] The release workflow produces the Linux archive, checksums, scripts and a VSIX.
6. [ ] The superseded installer/support paths are removed.
7. [ ] Existing `make verify`, `make vscode-check` and `make site-check` pass.
8. [ ] Documentation says Linux/macOS/Windows exactly as tested, with no WSL or
   unsupported-platform claims.
