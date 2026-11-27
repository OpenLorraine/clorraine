# VS Code Marketplace Extension and the v1.3.0 Release — Implementation Plan

> **Feature**: Publish `openlorraine.clorraine` to the VS Code Marketplace, install it from there, and release Clorraine v1.3.0
> **Status**: Planning Complete
> **Created**: 2026-11-27 14:47
> **Implements**: — (task plan T-05; source: GitHub issue #38, revised by the Ambiguity Register)
> **Mode**: `--auto-design` (root invocation `ad-20261127-vsmkt`); execution with `--auto-commit`
> **CodeOps Artifact Schema**: 1

## Overview

The VS Code extension is a thin client that finds an installed `clorraine-lsp` and `clorraine`.
Today it ships only as a VSIX attached to each GitHub release: both installers download it, check
a pinned size and SHA-256, and install the file. VS Code pins file-installed extensions, so users
never receive updates, and editors without the installer have no clean path to the extension.

This plan publishes the extension to the Microsoft VS Code Marketplace through a GitHub Actions
workflow that uses Trusted Publishing, switches both installers to install and update it by ID,
makes the starter project's tasks portable, and then releases v1.3.0. That release also carries the
already-merged Windows CLI and installer and the Amiga crash-safety fixes.

Work happens on the `release-1.3.0` branch (AR-40). Only the publish workflow reaches `main` before
the release, so dry runs can be dispatched; nothing user-visible on the site or README changes
before the GitHub release exists.

## Minimum-Sufficient Baseline

**Original goal:** Install and update the VS Code extension from the Marketplace instead of a
pinned release file, then ship v1.3.0 with the extension, the Windows installer, and the compiler
fixes.
**Smallest viable design:** One universal VSIX, version taken from `compiler/Cargo.toml`
(existing `stage.mjs` rule), built from the release tag by one two-job workflow and published with
`vsce publish --oidc`; installers call the editor CLI by extension ID; the extension contributes
one command for the starter tasks plus two small messages; the release drops the VSIX asset.
**Excluded machinery:** Open VSX publishing; platform-targeted VSIXs; bundling the language
server; a local publish target; a required-reviewer approval step.
**Approved complexity:** None — the workflow is the user's chosen publication mechanism (AR-5),
and its two-job split reuses the `docs.yml` build/deploy pattern (AR-30).

## Planning Coverage

> **Coverage**: 4/4 clusters · 12/12 categories · 34 candidates (15 material after merge) · 4 residual limits · Effort: high/high/high/xhigh→inherited (no provider variant) · Independence: uniform-effort
> **Freshness**: Coverage as of 2026-11-27; artifacts: register AR-1..AR-24, issue #38, tools/vscode, tools/setup installers/packaging/acceptance, compiler starter and LSP serverInfo, docs/ and site/, workflows

| Cluster | Area | Examined | Not examined / limit | Candidates | Notes / limit | Confidence |
|---------|------|----------|----------------------|------------|---------------|------------|
| C1 Product and scope | extension, starter, installers, acceptance, docs/site, issue #38 | full read of named files; v1.2.0 starter via git | live VS Code behaviour; install.ps1 summary rendering | 9 → AR-25, 27, 28, 32, 34, 35, 36, 37, 38, 39 | `general` agent substituted for `preflight-auditor` | high |
| C2 Behavior and edges | `${command:}`, `--force`/pinning, acceptance, drift, re-runs | upstream VS Code source (pinned metadata, auto-update) | Pester suites; live Marketplace | 7 → AR-25, 26, 28, 29, 32; re-run folded into AR-30; one optional omitted | `general` substituted | high |
| C3 Technical and data | vsce 4.1.0 internals, workflows, packaging, acceptance schema, version inventory | vsce source; host rasterizers | runner image; docker image ImageMagick | 10 → AR-25, 27, 28, 29, 30, 32, 34, 36, 38; re-run folded into AR-30 | `general` substituted | high |
| C4 Risk and stakeholders | OIDC exposure, integrity, recovery, timeouts, provenance, listing | lockfile, workflows, installers | Marketplace byte identity after signing; vsce-sign need; `--force` vs newer installed | 8 → AR-26, 28, 29, 30, 31, 33, 38; one optional omitted | `general` substituted; external doc check | medium-high |

**Source note:** run-wide CV-001..CV-015 map to AR-25..AR-39 as recorded in the register's
Coverage Map. No Discovery keys.

## Document Index

| #    | Document                                              | Description                                              |
| ---- | ----------------------------------------------------- | -------------------------------------------------------- |
| AR   | [Ambiguity Register](00-ambiguity-register.md)        | Zero-Ambiguity Gate decisions (AR-1..AR-45)              |
| 00   | [Index](00-index.md)                                  | This document                                            |
| 01   | [Requirements](01-requirements.md)                    | Requirements, scope, acceptance criteria                 |
| 02   | [Current State](02-current-state.md)                  | What exists today and the gaps                           |
| 03-1 | [Extension](03-01-extension.md)                       | Manifest, listing, `clorraine.driverPath`, guide action, drift notice, audit |
| 03-2 | [Publish Workflow](03-02-publish-workflow.md)         | `publish-vscode.yml`, version guard, dry runs            |
| 03-3 | [Starter and Installers](03-03-starter-installers.md) | Portable starter tasks, Marketplace install/update in both installers |
| 03-4 | [Packaging and Acceptance](03-04-packaging-acceptance.md) | Four-asset candidate, acceptance evidence for the extension |
| 03-5 | [Docs and Release](03-05-docs-release.md)             | Docs sweep, runbook, release notes, release procedure    |
| 07   | [Testing Strategy](07-testing-strategy.md)            | ST-cases and verification                                |
| 99   | [Execution Plan](99-execution-plan.md)                | Phases and task checklist                                |

## Quick Reference

### Usage Examples

```sh
# Installer (Linux): install or update the extension from the Marketplace without asking
sh install.sh --vscode
# Equivalent editor command the installers run
code --install-extension openlorraine.clorraine
```

```json
// Starter .vscode/tasks.json after this plan
{ "label": "Clorraine: Run in FS-UAE", "type": "process",
  "command": "${command:clorraine.driverPath}",
  "args": ["run", "-p", "hosted-amiga", "--emulator", "main.clo"] }
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Registry | Microsoft Marketplace only (AR-2) |
| Publication | GitHub Actions + Trusted Publishing, build/publish split, tag-version guard (AR-5, AR-29, AR-30) |
| Release assets | Four assets + `SHA256SUMS`; no VSIX (AR-6) |
| Installers | Uninstall-then-install by ID when present; consent unchanged when absent; 300 s bound (AR-26, AR-27, AR-33) |
| Release | v1.3.0 GitHub pre-release; publish before acceptance; fix forward (AR-10, AR-17, AR-31) |

## Specialist Agents

**None** — the capability-gap check ran against this repository's evidence and found no gap a
standing specialist would close.

_Detection evidence: the change surface is a TypeScript extension (`tools/vscode/src/`), shell and
PowerShell installers (`tools/setup/install.{sh,ps1}`), packaging scripts, one GitHub workflow, and
docs. The catalog `correctness-reviewer` and `security-auditor` with dynamic packets cover them;
`amiga-platform-reviewer` is not triggered (no `std.amiga.*`, shim, FS-UAE gate, or registry
change). The single CI-publication concern is one-off, not recurring._

## Related Files

- `tools/vscode/package.json`, `src/{extension,discovery,actions,paths}.ts`, new `src/guide.ts`,
  `scripts/{stage.mjs,audit_vsix.py}`, new `scripts/release-version.mjs`, `README.md`, new
  `CHANGELOG.md`, new `DEVELOPMENT.md`, new `images/icon.png`, `test/*`
- `.github/workflows/publish-vscode.yml` (new)
- `compiler/templates/starter/tasks.json`, `compiler/src/clorraine/{new.rs,mod.rs}`,
  `compiler/tests/{first_run_spec_test.rs,windows_cli_spec_test.rs,windows_cli_impl_test.rs}`
- `tools/setup/{install.sh,install.ps1,package.sh,acceptance.py,acceptance.sh,acceptance.ps1}`,
  `tools/setup/tests/*`, `Makefile`
- `compiler/Cargo.toml`, `Cargo.lock`, `tools/vscode/package{,-lock}.json` (version)
- `README.md`, `AGENTS.md`, `CHANGELOG.md`, `RELEASE.md`, `RELEASE-NOTES.md`, `docs/*.md`,
  `site/**`, `plans/windows-cli/08-merge-notes.md`, `plans/00-roadmap.md`
