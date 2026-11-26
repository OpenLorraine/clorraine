# Windows CLI and Installer Implementation Plan

> **Feature**: Windows 11 x64 CLI, driver, editor, and a PowerShell installer equivalent to the Linux one
> **Status**: Planning Complete
> **Created**: 2026-11-26
> **Implements**: Windows CLI and installer (standalone plan; no RD — the owning requirements are [01-requirements.md](01-requirements.md))
> **Branch**: `windows-cli` (worktree `../clorraine-windows`), merged through a GitHub pull request
> **Evidence**: `spikes/08-windows-cli/README.md`
> **CodeOps Artifact Schema**: 1

## Overview

Clorraine v1.2.0 ships a Linux installer only. This plan brings the same path to Windows 11 x64:
one PowerShell command installs the CLI, sets up the pinned AmigaPorts rc24 toolchain (one UAC
prompt), finds or installs FS-UAE and the user's Kickstart ROM, runs the PC and Amiga hellos, and
installs the VS Code extension and a starter project.

The spike proved the path on the user's Windows 11 machine with a cross-built `clorraine.exe` and
four small patches, and found what blocks it: `.exe` names, the config directory, FS-UAE's
silent `--version`, verbatim `\\?\` paths, unescaped `#line` paths (a defect on every host), the
freestanding syscall layer, and the language server's Windows paths.

Everything is built on Linux in the pinned SDK image. Windows behaviour is tested on Linux through
OS-parameterized functions, by PowerShell suites on the Windows host, and by a recorded acceptance
run validated on Linux. Nothing is published by this plan.

## Minimum-Sufficient Baseline

**Original goal:** Ship a Windows installer and CLI equivalent to the Linux one, developed and
built on Linux, accepted on a real Windows 11 machine.
**Smallest viable design:** Contained driver changes behind one `platform.rs` module; escaped
`#line` paths and shim guards; LSP path normalization; `install.ps1` mirroring `install.sh`;
`package.sh`/`acceptance.py` extended rather than replaced.
**Excluded machinery:** A GitHub Actions Windows job and a `WINHOST` Makefile gate (AR-21); MSI/EXE
installer frameworks (AR-7); code signing (AR-18); a Job Object for interrupted runs (AR-39).
**Approved complexity:** AR-20 (MinGW and the Windows target in the pinned image, `cargo check`
in `make check`); AR-21 (simplified: the PowerShell suites, the evidence extension, and a
non-gating remote helper).

## Planning Coverage

> **Coverage**: 4/4 clusters · 12/12 categories · 30 candidates · 4 residual limits · Effort: high/xhigh→inherited · Independence: mixed-effort
> **Freshness**: Coverage as of 2026-11-26; artifacts 00-ambiguity-register.md (AR-1–AR-23), spikes/08-windows-cli/README.md, driver/LSP/extension/installer sources

| Cluster | Area | Examined | Not examined / limit | Candidates | Notes / limit | Confidence |
|---------|------|----------|----------------------|------------|---------------|------------|
| C1 Product and scope | Installer parity, starter, messages, assets | install.sh, package.sh, new.rs, starter templates, environment.rs, setup.rs, toolchain.rs, docs/linux-cli.md, README/site greps | acceptance internals, emulator, LSP (other clusters) | 8 → AR-24, 25, 27, 28, 29, 30, 31, 32, 33 | general-agent fallback for the auditor role | high |
| C2 Behavior and edges | Emulator lifecycle, LSP/extension round trip, installer state | emulator/*, emulator_runner.c, setup.rs, new.rs, artifacts.rs, lsp/*, tools/vscode/src/*, install.sh | no Windows runtime checks; client.ts/extension.ts partly | 8 → AR-26, 34, 38, 39, 40, 41, 42, 43 | general-agent fallback | medium-high |
| C3 Technical and data | cfg sites, config, artifacts, packaging, evidence | every cfg/target site, shim_freestanding.c, codegen casts, lsp keys, package.sh, acceptance.{sh,py}, Dockerfile | observe.rs, lsp/server.rs, tests, site | 8 → AR-25, 26, 29, 33, 38, 43, 44, 45, 46 | general-agent fallback | high |
| C4 Risk and stakeholders | Privilege boundary, deletion, PATH, transport, shared state | install.sh, package.sh, acceptance.{sh,py}, select.rs, storage.rs, docker/Makefile, worktrees, roadmap policy | no Windows runtime checks | 6 → AR-25, 26, 34, 35, 36, 37 | refuted: reproducible zip, timeout, licensing (upstream-fetch policy), signing (AR-18) | high |

**Source note:** C1 CV-001→AR-24/25, CV-002→AR-27, CV-003→AR-28, CV-004→AR-29, CV-005→AR-30, CV-006→AR-31, CV-007→AR-32, CV-008→AR-33; C2 CV-001→AR-38, CV-002→AR-39, CV-003→AR-43, CV-004→AR-26, CV-005→AR-40, CV-006→AR-41, CV-007→AR-34, CV-008→AR-42; C3 CV-001→AR-29, CV-002→AR-43, CV-003→AR-44, CV-004→AR-45, CV-005→AR-25/26, CV-006→AR-38, CV-007→AR-33/31, CV-008→AR-46; C4 CV-001→AR-34, CV-002→AR-35, CV-003→AR-26, CV-004→AR-36, CV-005→AR-25, CV-006→AR-37. Residual limits: no auditor ran anything on Windows; claims about PowerShell 5.1, .NET PATH writes, Node path spelling, and toolchain code pages are unverified until execution.

## Document Index

| # | Document | Description |
|---|---|---|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions AR-1 – AR-53 |
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | W1 – W18, scope, acceptance criteria |
| 02 | [Current State](02-current-state.md) | Code facts and gaps |
| 03-01 | [Build and Emission](03-01-build-and-emission.md) | Image, check step, `#line`, freestanding shim |
| 03-02 | [Driver](03-02-driver.md) | `platform.rs` and the Windows call sites |
| 03-03 | [LSP and Editor](03-03-lsp-editor.md) | Path normalization, extension discovery |
| 03-04 | [Installer](03-04-installer.md) | `install.ps1` |
| 03-05 | [Packaging, Acceptance, Docs](03-05-packaging-acceptance-docs.md) | Five-asset candidate, Windows evidence, helper, docs |
| 07 | [Testing Strategy](07-testing-strategy.md) | ST-1 – ST-74 |
| 99 | [Execution Plan](99-execution-plan.md) | 9 phases, 75 tasks |

## Quick Reference

### Usage Examples

```powershell
curl.exe -fsSLo clorraine-install.ps1 https://github.com/OpenLorraine/clorraine/releases/download/<tag>/install.ps1
powershell -ExecutionPolicy Bypass -File .\clorraine-install.ps1
clorraine run -p hosted-amiga --emulator hello.clo
```

### Key Decisions

| Decision | Outcome |
|---|---|
| Installer | `install.ps1`, PowerShell 5.1, ASCII (AR-7, AR-31) |
| Toolchain | Pinned rc24 setup, verified in an admin-only folder, one UAC prompt (AR-1, AR-34) |
| Layout | `%LOCALAPPDATA%\Programs\Clorraine`, `current` junction on PATH, config in `%APPDATA%` (AR-10, AR-25, AR-43) |
| Host compiler | `CLORRAINE_CC`; Windows `gcc`, `cc`, `clang`; Clang debug is a known limitation (AR-29, AR-15) |
| Build | Cross-built in the pinned image, branch tag `clorraine-sdk:windows-cli` (AR-20, AR-37) |
| Tests | OS-parameterized Rust tests on Linux; PowerShell suites and acceptance on Windows (AR-21, AR-48) |
| Release | Version chosen at release time; publication out of scope (AR-3) |

## Review Routing

Per AGENTS.md phase-review dispatch; every packet includes the blast-radius and test-integrity
steps.

| Phase | Reviewers |
|---|---|
| 1 | correctness reviewer |
| 2 | correctness reviewer, memory-safety auditor, emission-integrity auditor |
| 3 | correctness reviewer, memory-safety auditor, `amiga-platform-reviewer` (FS-UAE selection and spawn) |
| 4 | correctness reviewer |
| 5, 6 | correctness reviewer, a security review (general agent with a security brief: privilege boundary, deletion, PATH, downloads) |
| 7 | correctness reviewer, security review |
| 8 | correctness reviewer |
| 9 | correctness reviewer, `amiga-platform-reviewer` (FS-UAE acceptance) |

## Specialist Agents

**None** — the capability-gap check found no gap a standing specialist would close. Windows
installer and PowerShell security review is needed in two phases of one plan; a general agent with
a security brief covers it, as the Linux installer plan did with its generic security reviewer.
Revisit if Windows work recurs across plans.

_Detection evidence: `.opencode/agents/` holds `amiga-platform-reviewer`, `correctness-reviewer`,
`emission-integrity-auditor`, `memory-safety-auditor`, `semantics-reviewer`; `codeops/codeops.json`
is absent; `plans/linux-cli-v1.2.0/00-index.md` §Review routing used a generic security reviewer._

## Related Files

Created: `compiler/src/clorraine/platform.rs`, `compiler/tests/windows_cli_{spec,impl}_test.rs`,
`tools/setup/install.ps1`, `tools/setup/acceptance.ps1`, `tools/setup/windows-remote.sh`,
`tools/setup/windows-desktop-run.ps1`, `tools/setup/tests/install.{spec,impl}.test.ps1`,
`tools/setup/tests/windows-dist.{spec,impl}.test.sh`,
`tools/setup/tests/acceptance_windows_{spec,impl}.py`, `tools/vscode/test/windows.{spec,impl}.test.ts`,
`docs/windows-cli.md`.

Modified: `docker/Dockerfile`, `Makefile`, `compiler/src/codegen.rs`, `compiler/src/codegen/cleanup.rs`,
`compiler/runtime/shim_freestanding.c`, `compiler/src/clorraine/{setup,environment,toolchain,mod,artifacts,new}.rs`,
`compiler/src/clorraine/emulator/{mod,select,storage}.rs`, `compiler/src/lsp/{uri,workspace,source_files}.rs`,
`tools/vscode/src/{actions,discovery}.ts`, `tools/vscode/package.json`, `tools/setup/package.sh`,
`tools/setup/acceptance.py`, `tools/setup/tests/dist.spec.test.sh`, `tools/setup/tests/acceptance_spec.py`,
`docs/README.md`, `docs/known-limitations.md`, `README.md`, `site/`; at merge: `AGENTS.md`,
`plans/00-roadmap.md` (AR-37).
