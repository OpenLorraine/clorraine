# Testing Strategy: VS Code Marketplace Extension and the v1.3.0 Release

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Version guard, guide helpers, acceptance validation | 90% |
| Installer VS Code flow (both platforms) | every row of 03-3 §Flow |
| Workflow file | static checks below; live dry runs |
| Docs | sweep grep + `make site-check` |

- Test names state behaviour (`should … when …`, or the suite's existing naming).
- Live checks that cannot be automated (Marketplace publish, Windows desktop session, the editor
  confirmation) are release steps in 03-5, not tests.

## Test Slices & Groups

| Slice | Tier | Test Files | Sources | Gate |
| ----- | ---- | ---------- | ------- | ---- |
| extension | task | `tools/vscode/test/*.test.ts` | `tools/vscode/**`, workflow | `make vscode-check` |
| starter | task | `compiler/tests/first_run_spec_test.rs`, `windows_cli_*_test.rs` | `compiler/src/clorraine/`, `compiler/templates/starter/` | `make check` |
| installers / packaging / acceptance (bash, Python) | task | `tools/setup/tests/*.sh`, `*.py` | `tools/setup/**`, `Makefile` | `make dist-check` |
| installers (PowerShell) | task | `tools/setup/tests/install.{spec,impl}.test.ps1` | `tools/setup/install.ps1` | Windows host run (AGENTS.md Testing) |
| docs | task | — | `README.md`, `docs/`, `site/` | `make site-check` + sweep grep |
| all | release | everything | everything | `make verify` + both acceptance runs |

Gate commands come from AGENTS.md (AR-20). Gates run with `IMAGE=clorraine-sdk:windows-cli` while
the shared `dev` image lacks the Windows target.

## 🚨 Specification Test Cases

> Derived from 01, 03-1..03-5, and the register. **Immutable oracle:** if the implementation does
> not match, the implementation is wrong. In-code comments quote the behaviour in plain language,
> never these IDs.

### Publish workflow (03-2)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-1 | Read `.github/workflows/publish-vscode.yml` | The only trigger is `workflow_dispatch`; one input `dry_run`, `type: boolean`, `default: true` | AR-13, AR-41 |
| ST-2 | Same file | Top-level `permissions` is exactly `contents: read`; job `build` has no `id-token`; job `publish` has `environment: marketplace` and `permissions` `contents: read` + `id-token: write` | AR-12, AR-30 |
| ST-3 | Every `uses:` line | Pinned to a 40-hex SHA followed by a version comment | AR-12 |
| ST-4 | Same file | No `cache:` key and no `actions/cache` | AR-30 |
| ST-5 | Job `publish` | `needs: build`; `if` requires `!inputs.dry_run` and a `refs/tags/v` ref; steps contain, in order, a SHA-256 comparison against `needs.build.outputs.sha256`, `npm ci --ignore-scripts`, and `vsce publish --packagePath … --oidc`; no `--skip-duplicate` | AR-29, AR-30 |
| ST-6 | Every `run:` block | Contains no `${{` expression | AR-29 |
| ST-7 | `release-version.mjs --ref refs/tags/v1.3.0`, Cargo `1.3.0`, manifest `1.3.0` | Exit 0; stdout `release-version: 1.3.0` | AR-29, AR-42 |
| ST-8 | `--ref refs/tags/v1.3.0`, Cargo `1.2.0`, manifest `1.2.0` | Exit 1; stderr `release-version: tag v1.3.0 does not match version 1.2.0` | AR-29, AR-42 |
| ST-9 | `--ref refs/tags/v1.3.0-r1`, Cargo and manifest `1.3.0` | Exit 1; stderr `release-version: tag v1.3.0-r1 does not match version 1.3.0` | AR-29 |
| ST-10 | `--ref refs/heads/main`, both `1.3.0`: without `--publish`; with `--publish`; Cargo `1.3.0` vs manifest `1.2.0` | Exit 0; exit 1 `release-version: publishing requires a release tag, got refs/heads/main`; exit 1 `release-version: compiler version 1.3.0 does not match extension manifest 1.2.0` | AR-13, AR-42 |

### Extension (03-1)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-11 | `tools/vscode/package.json` | `icon` = `images/icon.png`; `homepage`, `bugs.url` are https; `keywords` ≤ 30; `galleryBanner` set; `preview` true; `extensionKind` = `["workspace"]`; `capabilities.untrustedWorkspaces.supported` = `"limited"` with a description; `clorraine.driverPath` in `contributes.commands` and in `menus.commandPalette` with `when: "false"`; `activationEvents` = `["onLanguage:clorraine"]` | AR-18, AR-32, AR-38 |
| ST-12 | `make vscode-package`, then list the VSIX | Contains `extension/CHANGELOG.md` and `extension/images/icon.png` (PNG, width and height ≥ 128); does not contain `extension/DEVELOPMENT.md`; staged `package.json` version = Cargo version | AR-18, AR-36, AR-38 |
| ST-13 | `audit_vsix.py` on a VSIX missing `CHANGELOG.md`; missing the icon; with an SVG icon; with `preview` false | Each exits non-zero naming the problem | AR-18 |
| ST-14 | `installGuideUrl("linux")`, `("win32")`, `("darwin")` | `https://openlorraine.github.io/clorraine/guide/quick-start#install-on-linux`; `…#install-on-windows`; `https://openlorraine.github.io/clorraine/guide/quick-start` | AR-35 |
| ST-15 | `serverOlderThan` on (`1.2.0`,`1.3.0`), (`1.3.0`,`1.3.0`), (`1.4.0`,`1.3.0`), (`undefined`,`1.3.0`), (`dev`,`1.3.0`), (`1.9.0`,`1.10.0`), (`1.10.0`,`1.9.0`) | true, false, false, false, false, true, false | AR-35 |
| ST-16 | Extension host with a server reporting `1.2.0`, extension `1.3.0`; the server restarts once | Exactly one information message `Clorraine CLI 1.2.0 is older than the extension 1.3.0. Run the installer to update.` with action `Open install guide` | AR-16, AR-35 |
| ST-17 | Execute `clorraine.driverPath` with a resolvable driver | Returns the absolute driver path | AR-8, AR-32 |
| ST-18 | Execute `clorraine.driverPath` with no driver anywhere | Returns `undefined`; one error message starting `Clorraine: No clorraine found` with action `Open install guide`; choosing it opens the platform URL | AR-32, AR-35 |
| ST-19 | Start with no language server; Build with no driver | Each shows its error with `Open install guide` | AR-9, AR-35 |
| ST-20 | Listing `README.md` | Contains `Linux x86_64 and Windows 11 x64; macOS is not supported yet`, the trademark line, links to both install guides; every link is absolute `https`; no `make vscode-package` text | AR-38 |

### Starter (03-3)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-21 | `clorraine new demo` on Linux and on Windows | Both tasks in `.vscode/tasks.json` have `"command": "${command:clorraine.driverPath}"`; the file contains no absolute path and no `@CLORRAINE@`; `extensions.json` recommends `openlorraine.clorraine` | AR-8, AR-25 |

### Installers (03-3) — each row on Linux (`install.spec.test.sh`) and Windows (`install.spec.test.ps1`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-22 | Only `codium` on PATH, `--vscode` | No editor command runs; state `unavailable`; warning names `code --install-extension openlorraine.clorraine` | AR-14 |
| ST-23 | `code` present, extension absent, non-interactive, no flag | No install call; summary `VS Code detected; rerun with --vscode to install the extension` | AR-27 |
| ST-24 | Extension absent, `--vscode` | Exactly one editor call `--install-extension openlorraine.clorraine` (an ID, not a file); summary `extension installed` | AR-7, AR-27 |
| ST-25 | Extension present, non-interactive, no flag | Calls `--uninstall-extension openlorraine.clorraine` then `--install-extension openlorraine.clorraine`; summary `extension updated from the Marketplace` | AR-7, AR-26, AR-27 |
| ST-26 | Extension present, `--no-vscode` | No editor call; summary `skipped` | AR-7 |
| ST-27 | Extension present; uninstall succeeds, install exits 1 | Warning `the Clorraine extension was removed but could not be reinstalled from the Marketplace; run: code --install-extension openlorraine.clorraine`; summary failed; installer exit status 0 | AR-15, AR-26 |
| ST-28 | `CLORRAINE_EDITOR_TIMEOUT=1`, fake editor sleeps 30 s on install | Editor killed within a few seconds; state failed with the manual command; installer completes with exit 0 | AR-33, AR-43 |
| ST-29 | Assembled installers from `make dist` / fixtures | No `VSIX_`/`Vsix` pin, no `.vsix` URL, no VSIX download | AR-6 |

### Packaging and acceptance (03-4)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-30 | `package.sh` with fixture stages | Output has exactly the Linux archive, Windows zip, `install.sh`, `install.ps1`, `SHA256SUMS`; `SHA256SUMS` has 4 lines | AR-6 |
| ST-31 | `package.sh --vsix x.vsix …` | Exits non-zero as an unknown option | AR-6, AR-34 |
| ST-32 | `package.sh --verify` with an extra `clorraine-1.3.0.vsix` in the output | Fails naming the file | AR-6 |
| ST-33 | Linux and Windows records with `extension` {id `openlorraine.clorraine`, version = candidate, numeric run, 64-hex SHA} and `manual.editor_build_run` true | `acceptance-check` passes | AR-28 |
| ST-34 | Same with `extension.version` `1.2.0`, candidate `1.3.0` | Fails `extension version 1.2.0 does not match candidate 1.3.0` | AR-28 |
| ST-35 | Record with a `vsix` key; record without `extension` | Each fails naming the key | AR-44 |
| ST-36 | Linux record without `manual.editor_build_run` true | Fails `manual.editor_build_run must be true` | AR-28 |

## Test Categories

### Specification Tests

| Test File | ST Cases | Component |
| --------- | -------- | --------- |
| `tools/vscode/test/publish-workflow.spec.test.ts` | ST-1..ST-6 | Workflow |
| `tools/vscode/test/release-version.spec.test.ts` | ST-7..ST-10 | Version guard |
| `tools/vscode/test/marketplace.spec.test.ts` | ST-11..ST-13, ST-20 | Manifest, package, audit, README |
| `tools/vscode/test/guide.spec.test.ts` | ST-14, ST-15 | Guide helpers |
| `tools/vscode/test/driver-command.spec.test.ts` | ST-16..ST-19 | Extension host behaviour (existing fake-host fixtures) |
| `compiler/tests/first_run_spec_test.rs` (rewritten assertion) | ST-21 | Starter |
| `tools/setup/tests/install.spec.test.sh`, `install.spec.test.ps1` (rewritten VS Code cases) | ST-22..ST-29 | Installers |
| `tools/setup/tests/dist.spec.test.sh`, `windows-dist.spec.test.sh` | ST-29..ST-32 | Packaging |
| `tools/setup/tests/acceptance_spec.py`, `acceptance_windows_spec.py` | ST-33..ST-36 | Acceptance |

Rewrites of existing spec tests are authorised by AR-25; the Windows `launcher_for` spec test is
deleted under the same ruling.

### Implementation Tests

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `tools/vscode/test/release-version.impl.test.ts` | Missing files, malformed versions, `GITHUB_OUTPUT` | High |
| `tools/vscode/test/guide.impl.test.ts` | Version edge cases (leading zeros, extra parts) | Med |
| `tools/setup/tests/install.impl.test.sh`, `install.impl.test.ps1` | Timeout override parsing, call order, summary text | High |
| `tools/setup/tests/windows-dist.impl.test.sh`, `acceptance_windows_impl.py` | Fixture and parser internals | Med |

### Integration / End-to-End

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Workflow dry run (Phase 1) | Dispatch from `main` with `dry_run=true` | Build succeeds, audit passes, publish job skipped |
| Release | 03-5 §Release procedure | All steps pass; acceptance records pass `acceptance-check` |

## Test Data

- Fake `code` launchers in `install_fixtures.sh` (Linux) and the PowerShell fixtures (Windows) that
  log their arguments, emulate an installed list, fail on demand, and sleep on demand.
- A minimal generated VSIX for audit negatives (built in the test from the staged tree).

## Verification Checklist

- [ ] All ST-cases defined with concrete input/output pairs
- [ ] Every ST case traces to an AR entry
- [ ] Spec tests written before implementation and seen failing
- [ ] Spec tests pass after implementation
- [ ] Implementation tests written
- [ ] Gates pass; no regressions
