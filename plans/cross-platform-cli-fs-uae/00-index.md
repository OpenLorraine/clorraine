# Linux v1.2.0 CLI and FS-UAE delivery

> **Feature**: CLI-first Linux build/run workflow: install our CLI, validate a user-downloaded Amiga toolchain, build programs and run them in FS-UAE. macOS/Windows are a separate follow-up plan.
> **Status**: Preflight blocked — scope reduction required; see the preflight report
> **Created**: 2026-11-23
> **Implements**: — (standalone user-requested feature)
> **Roadmap**: PLAN-46
> **CodeOps Artifact Schema**: 1

## Overview

Clorraine users write a program in an editor or a terminal, build it, and run it
on an Amiga emulator. The product must work from the command line and from a
Makefile with no editor and no Docker. This plan delivers the Linux x64 v1.2.0
path; macOS arm64 and Windows x64 are deferred to a follow-up plan so their
platform-specific evidence and release wiring do not block Linux.

The delivery uses the AmigaPorts prebuilt m68k toolchain. The user downloads the
upstream archive; `clorraine setup --toolchain PATH` validates it and writes the
config. The VS Code extension becomes a thin client of the installed CLI.
FS-UAE remains user-provided; the CLI discovers it and stages the program for a
real Kickstart ROM (AROS is launch-only).

This plan replaces the superseded `v1.2.0-release-installer` design (AR-50).

## Minimum-Sufficient Baseline

**Original goal:** Write a Clorraine program, compile it, and run it in FS-UAE
from the command line or VS Code on Windows, Linux or macOS, without Docker and
without asking users to build the Amiga toolchain.

**Smallest viable design:** Ship our three Rust binaries; add `clorraine setup`
to validate a user-downloaded upstream toolchain and write the config; reuse the
existing FS-UAE launcher and driver discovery; let the extension call the
installed CLI. Linux v1.2.0 first; macOS/Windows follow. Use plain GitHub-hosted
release builds.

**Excluded machinery:** Docker for users, a self-hosted runner, our own
toolchain builds/audits or toolchain downloader, a support-lock/support-release
pipeline, bundled `vamos` wheels, a package service, a custom updater daemon,
Marketplace publishing or code signing in the first release.

**Approved complexity:** AR-3 (upstream toolchain), AR-4 (system curl/tar),
AR-12 (AROS fallback), AR-22 (single hosted matrix). No larger surface is
approved.

## Document Index

| # | Document | Owns |
|---|---|---|
| AR | [Ambiguity Register](00-ambiguity-register.md) | All resolved decisions |
| PF | [Preflight Report](00-preflight-report.md) | Audit findings and resolutions |
| 00 | [Index](00-index.md) | Goal, baseline, navigation |
| 01 | [Requirements](01-requirements.md) | Product requirements and scope |
| 02 | [Current State](02-current-state.md) | Existing code and gaps |
| 03-01 | [CLI, setup and installation](03-01-cli-setup.md) | Commands, layout, config, bootstrap |
| 03-02 | [Platform port](03-02-platform-port.md) | Windows and macOS driver work |
| 03-03 | [FS-UAE and editor](03-03-fs-uae-editor.md) | Emulator launch, ROM/AROS, VSIX |
| 03-04 | [Release and migration](03-04-release-migration.md) | Hosted builds, assets, old-path removal |
| 07 | [Testing Strategy](07-testing-strategy.md) | ST cases and verification |
| 99 | [Execution Plan](99-execution-plan.md) | Phases and task checklist |

## Quick Reference

```sh
# Linux v1.2.0: download, verify, run (no pipe to shell)
curl -fLO https://github.com/OpenLorraine/clorraine/releases/download/v1.2.0/install.sh
curl -fLO https://github.com/OpenLorraine/clorraine/releases/download/v1.2.0/SHA256SUMS
sha256sum -c SHA256SUMS --ignore-missing
sh install.sh

# macOS/Windows installers follow in a later release.
# After downloading/extracting the upstream AmigaPorts toolchain:
clorraine setup --toolchain /path/to/m68k-amigaos-gcc-16.2

# Daily use (Linux v1.2.0)
clorraine build -p hosted-amiga main.clo
clorraine run   -p hosted-amiga --emulator main.clo
```

### Key Decisions

| Decision | Outcome |
|---|---|
| Product shape | CLI-first; VS Code is a client |
| Platforms | Linux x64 in v1.2.0; macOS arm64/Windows x64 follow-ups |
| Toolchain | User downloads upstream; setup validates it |
| Installer | Download/verify/run scripts + `clorraine setup` |
| Amiga run | FS-UAE with user ROM; AROS launch-only fallback |
| Terminal run | Optional `vamos` on PATH; not bundled |
| Release | One hosted Linux matrix; unsigned assets |

## Specialist Agents

| Role | Kind | Use | AR Ref |
|---|---|---|---|
| `amiga-platform-reviewer` | reviewer | Phase 1/3/5 review of FS-UAE, Kickstart/AROS and emulator path behavior | AR-11/12 |
| Generic correctness reviewer | reviewer | Every phase; CLI/API correctness and Linux platform behavior | AR-2/10 |
| Generic security reviewer | reviewer | Phase 2/4/5; installer download, config and publication safety | AR-23 |

_Detection evidence:_ the repository already routes Amiga platform changes to
`amiga-platform-reviewer` (`.opencode/agents/amiga-platform-reviewer.md`). A
macOS/Windows specialist is not configured; the follow-up plan must carry that
lens explicitly.

## Related Files

| Path | Change |
|---|---|
| `compiler/src/clorraine/setup.rs` (new) | Setup command, validation, config |
| `compiler/src/clorraine/{environment,emulator,artifacts,native_metadata,mod}.rs` | Windows/macOS port and AROS fallback |
| `compiler/src/clorraine/cli.rs` | `setup` command and flags |
| `tools/setup/install.sh`, `install.ps1` | New bootstrap scripts |
| `tools/vscode/src/discovery.ts`, `actions.ts` | CLI client behavior |
| `.github/workflows/release.yml` | Hosted release matrix |
| `tools/release/tests/` (new) | Release and bootstrap specs |
| Remove: `tools/setup/install_native.py`, `native_archive.py`, `native_state.py`, `tools/distribution/`, old workflows | Superseded paths |
