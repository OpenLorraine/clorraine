# Current State: VS Code Marketplace Extension and the v1.3.0 Release

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

- **Extension** (`tools/vscode/`): npm package with no runtime dependencies; `@vscode/vsce`
  4.1.0 (supports `--packagePath` and `--oidc`) as a dev dependency. Activation is
  `onLanguage:clorraine` only (`package.json:16`). Tools are found by `resolveTool`
  (`src/discovery.ts:87-119`), which throws in untrusted workspaces and when nothing is found.
  `serverInfo.version` arrives from `clorraine-lsp` (`compiler/src/lsp/server.rs:531-537`,
  `CARGO_PKG_VERSION`) but is not read. `scripts/stage.mjs` stages an explicit file list and sets
  the version from `compiler/Cargo.toml`; `scripts/audit_vsix.py:18-30` checks an explicit
  required/forbidden list, not the version or listing metadata.
- **Missing-tool errors**: language-server start (`src/extension.ts:268-274`), Build/Run
  (`src/actions.ts:208` throws into `:303-307`), both plain `showErrorMessage` text.
- **Starter** (`compiler/src/clorraine/new.rs`, `compiler/templates/starter/`): `tasks.json`
  bakes the absolute launcher path (`new.rs:43`, `launcher_for` at `:95-128`, re-exported at
  `compiler/src/clorraine/mod.rs:28`); `extensions.json` already recommends
  `openlorraine.clorraine`.
- **Installers**: `install.sh` detects `code`, `codium`, `code-insiders` (`:662-673`), downloads
  the pinned VSIX with `curl --max-time 120` (`:618-660`), returns early when already installed
  (`:695-698`), and installs the file (`:722`) with no timeout. `install.ps1` mirrors this
  (`Find-VSCode` `:1498-1508`, `Install-VSCodeExtension` `:1513-1559`, `Invoke-Native`
  `:266-281` waits without a limit).
- **Packaging** (`Makefile:81-110`, `tools/setup/package.sh`): `make dist` builds the VSIX,
  writes five assets, and embeds the VSIX name/URL/size/SHA-256 into both installers;
  `dist-verify` checks those pins.
- **Acceptance** (`tools/setup/acceptance.{sh,ps1,py}`): requires the candidate VSIX and records
  it under `vsix` (`acceptance.py:47-60,296-302`); Linux runs the installer non-interactively
  without `--vscode` (`acceptance.sh:104`), Windows passes `-NoVSCode` (`acceptance.ps1:177`), so
  neither installs the extension.
- **Workflows**: `check.yml` (host checks) and `docs.yml` (site; deploys on `main` pushes under
  `site/**`, build/deploy split, pinned `checkout`/`setup-node`). No release or publish workflow.
- **Release text**: `RELEASE.md` is a superseded v1.2.0 design; `RELEASE-NOTES.md` is the v1.2.0
  candidate record; `CHANGELOG.md` `[Unreleased]` is empty; Windows docs use `vX.Y.Z` and "next
  release"; Linux one-liners pin v1.2.0.
- **External setup (done, AR-22)**: Marketplace publisher `openlorraine` with a Trusted Publishing
  policy for `publish-vscode.yml` / environment `marketplace`; GitHub environment `marketplace`
  with a `v*` tag rule and no reviewers or secrets.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `tools/vscode/package.json` | Manifest | Listing fields, command, palette hiding, capabilities (03-1) |
| `tools/vscode/src/extension.ts`, `actions.ts`, `discovery.ts` | Client, actions, discovery | Guide action, drift notice, driver command (03-1) |
| `tools/vscode/scripts/stage.mjs`, `audit_vsix.py` | Packaging and audit | Stage icon/changelog; assert listing (03-1) |
| `.github/workflows/publish-vscode.yml` | — | New (03-2) |
| `compiler/templates/starter/tasks.json`, `compiler/src/clorraine/new.rs`, `mod.rs` | Starter | Command variable; remove `launcher_for` (03-3) |
| `tools/setup/install.sh`, `install.ps1` | Installers | Marketplace install/update, states, timeout, detection (03-3) |
| `Makefile`, `tools/setup/package.sh` | Candidate producer | Four assets, no VSIX (03-4) |
| `tools/setup/acceptance.{sh,ps1,py}` | Acceptance | Extension evidence (03-4) |
| Docs, site, release text | User-facing | Sweep and runbook (03-5) |

## Gaps Identified

### Gap 1: No Marketplace presence
**Current:** VSIX only on GitHub releases. **Required:** listed extension, published from CI.
**Fix:** 03-1, 03-2.

### Gap 2: Installed extensions never update
**Current:** file installs are pinned by VS Code; `--force` keeps the pin (AR-26 evidence).
**Required:** a Marketplace install that auto-updates. **Fix:** uninstall-then-install (03-3).

### Gap 3: Starter tasks are tied to one machine
**Current:** absolute launcher path. **Required:** `${command:clorraine.driverPath}`. **Fix:** 03-1, 03-3.

### Gap 4: Release and acceptance bind a VSIX that will no longer ship
**Fix:** 03-4.

### Gap 5: Stale and placeholder release text
**Fix:** 03-5.

## Dependencies

### Internal Dependencies
- Phase 1 (workflow) must be on `main` before any dispatch (AR-40).
- The publish (Phase 7) precedes acceptance; acceptance needs the Marketplace listing (AR-17, AR-28).

### External Dependencies
- Marketplace Trusted Publishing policy (configured; proven by the first publish).
- GitHub-hosted runner `ubuntu-24.04` with Python 3.10+; Node 22 via pinned `setup-node`.
- Windows test host `winbox` (desktop session) for the PowerShell suites and Windows acceptance.
- The shared `clorraine-sdk:dev` image lacks the Windows target; gates run with
  `IMAGE=clorraine-sdk:windows-cli` until it is retagged (the crash-safety plan did the same).

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Trusted Publishing policy misconfigured | Med | Publish fails at the token exchange | Fails before upload; fix the policy, re-run (version not consumed) |
| Wrong version published | Low | Permanent | Version guard in both jobs (AR-29) |
| Dependency abuses the OIDC token | Low | Permanent publish | Build/publish split, `--ignore-scripts`, no cache (AR-30) |
| Acceptance fails after publish | Med | 1.3.0 consumed | Fix forward to v1.3.1 (AR-31) |
| Uninstall succeeds, install fails | Low | User without extension | Explicit warning with the reinstall command (AR-26) |
| Another session pushes to `main` between tag and fast-forward | Low | Fast-forward impossible | Ask the user to hold `main` during the release window (03-5 §Release procedure) |
| Marketplace serves different bytes than the CI VSIX | Med | Recorded SHA differs from download | The record names the CI-built VSIX and the run; acceptance checks the installed version (AR-28) |
