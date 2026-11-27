## Ambiguity Register: VS Code Marketplace extension and the v1.3.0 release

> **Status**: ✅ GATE PASSED — all 46 items resolved (40 user decisions, 6 delegated under --auto-design)
> **Last Updated**: 2026-11-27 14:47
> **Source**: GitHub issue #38 (Publish the VS Code extension), revised by the decisions below

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| 1 | Scope | What does this plan cover? | Marketplace + installers + v1.3.0 release / Marketplace + installers only | Marketplace + installers + release: listing readiness, publishing, installer and starter changes, then the v1.3.0 release (version bump, Windows docs, both acceptance runs, publication, issue closures) | ✅ Resolved |
| 2 | Scope | Which registries? | Microsoft Marketplace + Open VSX / Microsoft Marketplace only | Microsoft Marketplace only; Open VSX is out of scope | ✅ Resolved |
| 3 | Technical | One universal VSIX or platform-targeted VSIXs? | Universal / linux-x64 + win32-x64 targeted | Universal package (no `--target`) | ✅ Resolved |
| 4 | Data & state | How is the extension version set? | Coupled to `compiler/Cargo.toml` / independent semver | Coupled: the VSIX version stays the compiler version (today's `stage.mjs` rule); an extension-only fix needs a patch release | ✅ Resolved |
| 5 | Technical | How is a publish performed? | Local make target / GitHub Actions workflow | GitHub Actions workflow `.github/workflows/publish-vscode.yml` using Marketplace Trusted Publishing (`vsce publish --oidc`, no stored secret). Evidence: Azure DevOps global PATs retire 2026-12-01 (VS Code publishing docs, updated 2026-11-25); `@vscode/vsce` 4.1.0 supports `--oidc` | ✅ Resolved |
| 6 | Scope | Does the GitHub release keep the VSIX once installers use the Marketplace? | Keep it / drop it | Drop it: the release carries the Linux archive, the Windows zip, `install.sh`, `install.ps1`, and `SHA256SUMS` | ✅ Resolved |
| 7 | Behavioral | What does the installer do when the extension is already installed? | Leave it / update it | Always update: `--install-extension openlorraine.clorraine --force` unless `--no-vscode` / `-NoVSCode` is given | ✅ Resolved |
| 8 | Behavioral | How does the starter's `.vscode/tasks.json` find the driver? | Ask the extension / keep the absolute path / drop tasks.json | Ask the extension: it contributes `clorraine.driverPath`; tasks use `${command:clorraine.driverPath}`; `extensions.json` is unchanged | ✅ Resolved |
| 9 | UX & presentation | What does the missing-tool error offer a Marketplace user without the CLI? | Install-guide button / today's text | Add an "Open install guide" action that opens the site's install guide for the host platform | ✅ Resolved |
| 10 | Scope | GitHub release channel for v1.3.0, given open critical compiler issues (#55, #56, #61–#64)? | Pre-release / Latest | Pre-release ("developer preview"), like v1.2.0; the listing sets `"preview": true` | ✅ Resolved |
| 11 | Technical | With no VSIX in the release, what does the workflow publish? | Build from the tag in CI / upload a locally built VSIX | Build from the tag in CI: `npm ci`, build, stage, `vsce package`, `audit_vsix.py`, then `vsce publish --packagePath <vsix> --oidc`; the run summary records the VSIX SHA-256 and keeps the VSIX as a run artifact | ✅ Resolved |
| 12 | Security & compliance | How is a publish run approved? | Protected environment with required reviewer / dispatch only | Environment `marketplace` restricted to tags matching `v*`, **no** required reviewers (sole maintainer; GitHub allows self-approval, so a reviewer adds no second person). Starting the workflow from a tag is the approval; the agent asks the user before every publish dispatch. Read-only default permissions; only the publish job gets `id-token: write`; actions pinned by SHA like `check.yml` | ✅ Resolved |
| 13 | Technical | Does the workflow have a dry-run mode, and which ref does it publish? | dry_run input / none; tag input / dispatched ref | `dry_run` input. Refinement of the tag input: the workflow builds the ref it was dispatched from; dry runs run from `main` and never reach the publish job; the publish job refuses a non-tag ref (the environment's `v*` tag rule enforces the same) | ✅ Resolved |
| 14 | Scope | VSCodium has no install path (Open VSX out, VSIX dropped). What do installers detect? | code + code-insiders only / keep codium with a note | Detect `code` and `code-insiders` only; docs name VS Code (Microsoft build) as the supported editor | ✅ Resolved |
| 15 | Behavioral | What happens when the Marketplace install fails (offline, proxy, not listed)? | Warn and continue / fail the install | Warn and continue: the CLI install still succeeds; print the exact manual command `code --install-extension openlorraine.clorraine` | ✅ Resolved |
| 16 | UX & presentation | Version drift: VS Code auto-updates the extension, not the CLI | Non-blocking notice / none | Non-blocking notice once per session when the server's `serverInfo.version` is older than the extension's version, with an "Open install guide" action; features keep working through the existing capability checks | ✅ Resolved |
| 17 | Integration points | Release order | Tag → publish → accept → release / other | Release commit (1.3.0, docs) → push tag `v1.3.0` → publish workflow from the tag (user approves dispatch) → extension 1.3.0 live → `make dist` (four assets) → Linux and Windows acceptance installing from the Marketplace → user approves → `gh release create --prerelease` → fast-forward `main`. Linux v1.2.0 users receive extension 1.3.0 early; it works with their CLI | ✅ Resolved |
| 18 | Scope | Include issue #38's listing items? | All / icon and README only | All: PNG icon from `site/public/logo.svg`, user-facing listing README (developer notes moved out), `CHANGELOG.md` in the package, `homepage`/`bugs`/`keywords`/`galleryBanner`, `"preview": true`, `extensionKind: ["workspace"]`, explicit `capabilities.untrustedWorkspaces`; `audit_vsix.py` asserts them | ✅ Resolved |
| 19 | Integration points | Who recreates the Windows standard test account for acceptance? | Agent over `ssh winbox` / user | Agent over `ssh winbox`, with the user's approval at that step; removed again after acceptance | ✅ Resolved |
| 20 | Technical | Verify commands | Per phase + full at release / skip `make verify` | Per task/phase: `make check`, `make vscode-check`, `make dist-check`, plus `make site-check` for docs; release gate: `make verify` on the release commit, then Linux and Windows acceptance with `make acceptance-check` | ✅ Resolved |
| 21 | Scope | Out of scope | — | Open VSX, VSCodium, Cursor and other editors; platform-targeted VSIXs; bundling `clorraine-lsp`; macOS; verified-publisher domain; the Marketplace pre-release channel (`--pre-release`); the Windows merge-note follow-ups (multi-file starter tasks and the `E11102` message, the Linux encrypted-ROM check, the Clang issue); the open critical compiler issues | ✅ Resolved |
| 22 | Integration points | Owner-only setup | — | Done by the user 2026-11-27: Marketplace publisher `openlorraine` (display name OpenLorraine, publicly visible), Trusted Publishing policy (owner `OpenLorraine`, repo `clorraine`, workflow `publish-vscode.yml`, environment `marketplace`). Agent verified via the GitHub API: environment `marketplace`, one tag policy `v*`, no reviewers, no secrets or variables. The policy itself is proven by the first real publish | ✅ Resolved |
| 23 | Scope | Version number | — | v1.3.0 (user: "release 1.3.0 containing both the new vscode thing, the compiler fixes, and a windows installer") | ✅ Resolved |
| 24 | Naming & terminology | Plan folder and roadmap row | — | `plans/vscode-marketplace-v1.3.0/`, roadmap row T-05 (user: "go ahead"). The Discord invitation was done separately as T-04 and is not part of this plan | ✅ Resolved |
| 25 | Scope | AR-6/7/8 contradict immutable spec tests; may the plan rewrite them, and does `launcher_for` survive? (CV-001) | Authorise rewrite to the new oracles + retire `launcher_for` / rewrite but keep `launcher_for` / narrow the decisions | Rewrite + retire `launcher_for`: each listed spec test (starter tasks assertion, installer no-reinstall/VSIX download, dist five-asset and VSIX pins, acceptance `vsix` key, PowerShell VSIX pins) is rewritten to the new oracle spec-first (written, red, then green); `launcher_for` and its Windows spec test are deleted under the no-dead-code rule | ✅ Resolved |
| 26 | Integration points | v1.2.0 installed the VSIX, so VS Code pins it (no auto-update) and `--force` keeps the pin (CV-002) | Uninstall then install by ID / `--force` + docs / accept | Uninstall, then install by ID: `--uninstall-extension openlorraine.clorraine` then `--install-extension openlorraine.clorraine`; the fresh Marketplace install is not pinned, so auto-update works afterwards. If the install fails after the uninstall, warn that the extension was removed and print the reinstall command. Corrects AR-17's note: v1.2.0 users do not receive 1.3.0 automatically; they get it by rerunning an installer | ✅ Resolved |
| 27 | Behavioral | Installer consent and state model under AR-7 (CV-003) | Literal AR-7 with new `updated` state / update only with consent | Literal AR-7 + new states: not installed → today's consent (interactive prompt; non-interactive needs `--vscode`/`-VSCode`, else `available`); installed → update in every mode unless `--no-vscode`/`-NoVSCode`. States: installed, updated, available, skipped, unavailable, failed; a failed update says the extension may be missing and prints the command. Help, summary, and next-steps text updated; `--vscode` keeps meaning "install without asking" | ✅ Resolved |
| 28 | Integration points | What acceptance installs and records once the VSIX is gone (CV-004) | Install with `--vscode`, record id+version+run+SHA, manual LSP/Build-Run check / version only / nothing | Install + record + manual check: acceptance runs the installer with `--vscode`/`-VSCode` in the isolated account; the evidence records `extension {id, version}` from `code --list-extensions --show-versions` plus the publish run ID and VSIX SHA-256 from the run summary; `acceptance-check` requires version == candidate version. Issue #38's "LSP starts, Build/Run works" is a recorded manual confirmation. Precondition: the Marketplace serves the release version | ✅ Resolved |
| 29 | Security & compliance | Nothing ties the published version to the tag (CV-005) | Hard tag = Cargo = manifest check before publish / rely on checklist | Hard check in both jobs: tag == `v` + `compiler/Cargo.toml` version == staged `package.json` version, read through `env:` (never interpolated into `run:`); fail before packaging and again before `vsce publish`; `-rN` tags are refused | ✅ Resolved |
| 30 | Security & compliance | OIDC token exposure to build steps and dependencies (CV-006) | Split build/publish jobs, no cache, `--ignore-scripts` in publish / single job | Build job + publish job: build (no `id-token`): `npm ci`, package, audit, version check, upload artifact, output SHA-256. Publish (environment `marketplace`, `id-token: write`): download, re-verify SHA-256, `npm ci --ignore-scripts`, `vsce publish --packagePath <vsix> --oidc`. No npm cache; reuse `docs.yml`'s pinned checkout/setup-node; pin upload/download-artifact by SHA. A re-run on an already-published version fails (no `--skip-duplicate`); recovery is a manual listing check | ✅ Resolved |
| 31 | Non-functional | Recovery after publish but before release; announcement timing (CV-007) | Never move a published tag, fix forward to v1.3.1 / reorder acceptance before publish | Fix forward: never move a published tag; if acceptance fails after publication, fix, bump to v1.3.1, tag, publish, and rerun the full gate. The community announcement goes out only after `gh release create` and the site deploy | ✅ Resolved |
| 32 | Behavioral | `clorraine.driverPath` activation and failure result (CV-008) | Contributed + hidden, show install-guide error and cancel the task / explicit activation event / throw | Contributed, hidden, cancel on failure: declared in `contributes.commands` (implicit activation on VS Code ≥1.85), hidden from the Command Palette; with no driver it shows the AR-9 install-guide error and returns `undefined`, so the task is cancelled without a second error | ✅ Resolved |
| 33 | Non-functional | Marketplace install has no timeout in either installer (CV-009) | Bound it and report failure / accept a hang | Bound at 300 s: Linux `timeout`; Windows `WaitForExit(ms)` plus killing the process tree; on expiry the state is `failed`, the manual command is printed, and the install continues (AR-15) | ✅ Resolved |
| 34 | Scope | Docs/version sweep wider than the merge notes; does `make dist` drop the VSIX build? (CV-010) | Sweep everything listed + `dist` stops building the VSIX / merge-notes list only | Sweep all + `dist` drops the VSIX: every current-release claim moves to v1.3.0 and the Marketplace (Linux one-liners, codium mentions, "from the release" wording, the extension README platform claim, AGENTS.md, merge notes); historical text stays (CHANGELOG 1.2.0, superseded RELEASE.md sections, test fixtures). `make dist`/`package.sh` stop building and passing the VSIX; `make vscode-check` still packages and audits it | ✅ Resolved |
| 35 | UX & presentation | Install-guide URLs, error sites, drift-notice wording and comparison (CV-011) | Proposed defaults / other | Proposed defaults: the "Open install guide" action appears at all three missing-tool sites (language-server start, Build/Run, `clorraine.driverPath`). URLs: Linux → `https://openlorraine.github.io/clorraine/guide/quick-start#install-on-linux`, Windows → `…#install-on-windows`, other hosts → `https://openlorraine.github.io/clorraine/guide/quick-start`. Drift notice text: "Clorraine CLI <server> is older than the extension <ext>. Run the installer to update." with "Open install guide"; once per extension-host activation; numeric x.y.z comparison; a missing or unparsable server version, or a newer server, shows nothing | ✅ Resolved |
| 36 | Naming & terminology | Packaged CHANGELOG source (CV-012) | New extension-only `tools/vscode/CHANGELOG.md` / root CHANGELOG | Extension-only changelog: new `tools/vscode/CHANGELOG.md` with editor-facing entries (1.2.0, 1.3.0), staged by `stage.mjs` and asserted by `audit_vsix.py`; each release adds an entry | ✅ Resolved |
| 37 | Feature gaps | Where the release + publishing runbook lives; RELEASE-NOTES.md for v1.3.0 (CV-013) | Rewrite RELEASE.md as the current runbook / new docs page | Rewrite RELEASE.md: it becomes the current runbook (version bump, gates, tag, publish workflow, `make dist`, Linux and Windows acceptance, `gh release`, fix-forward rule); git history keeps the superseded v1.2.0 design. RELEASE-NOTES.md becomes the v1.3.0 notes and the `gh release` body | ✅ Resolved |
| 38 | UX & presentation | Listing README content: platforms, disclaimer, screenshot, icon, links, untrustedWorkspaces value, dev notes (CV-014) | Proposed defaults / other | Proposed defaults: platforms "Linux x86_64 and Windows 11 x64; macOS is not supported yet"; requires the Clorraine CLI (install-guide links); the project's standard trademark and independence line; no screenshot; absolute https links only; icon 256 px PNG at `tools/vscode/images/icon.png`, rendered once from `site/public/logo.svg` and committed; `capabilities.untrustedWorkspaces` = `{"supported": "limited", "description": …}`; developer notes move to `tools/vscode/DEVELOPMENT.md` (not packaged) | ✅ Resolved |
| 39 | Scope | Closing #38 while its Open VSX half is out of scope (CV-015) | Close with a note + follow-up issue / close with a note only / keep open | Close + follow-up: close #38 with a note linking the release and the decisions; open "Publish to Open VSX for VSCodium and other editors" (enhancement, deferred) | ✅ Resolved |
| 40 | Technical | Where does execution happen, given `--auto-commit` pushes after each task while `docs.yml` deploys `site/**` from `main` and the README is public, and `workflow_dispatch` only runs a workflow that exists on the default branch? | Branch `release-1.3.0` with an early workflow-only fast-forward / commit straight to `main` / long-lived PR | Authority: AI — delegated by --auto-design (see AR-40 note) | ✅ Resolved |
| 41 | Technical | Default value of the workflow's `dry_run` input | `true` / `false` | Authority: AI — delegated by --auto-design (see AR-41 note) | ✅ Resolved |
| 42 | Technical | Where the tag/Cargo/manifest version check (AR-29) lives so it can be tested | Node script in `tools/vscode/scripts/` with `node --test` coverage / inline shell in the workflow | Authority: AI — delegated by --auto-design (see AR-42 note) | ✅ Resolved |
| 43 | Technical | How the installer tests exercise the 300 s editor timeout (AR-33) without waiting 300 s | Environment override read only by the installer (`CLORRAINE_EDITOR_TIMEOUT`, seconds) / no test | Authority: AI — delegated by --auto-design (see AR-43 note) | ✅ Resolved |
| 44 | Data & state | What `acceptance-check` does with a record that still carries the old `vsix` key | Reject as an unknown key / ignore it | Authority: AI — delegated by --auto-design (see AR-44 note) | ✅ Resolved |
| 45 | Technical | Small mechanisms surfaced while writing the specs: `virtualWorkspaces`, the guide action on an invalid explicit setting, the Linux manual-confirmation key and rerun flag, stale-VSIX rejection in `dist-verify`, the uninstall-failure message | As recorded in 03-01, 03-03, 03-04 / alternatives in the note | Authority: AI — delegated by --auto-design (see AR-45 note) | ✅ Resolved |
| 46 | UX & presentation (runtime) | How the Windows install command is documented | Today: `curl.exe -fsSLo … ; powershell -ExecutionPolicy Bypass -File …` / a one-liner like Codex's `powershell -ExecutionPolicy ByPass -c "irm <url> \| iex"` | User: "we should do something like what codex does … the URL would be from our repo". Primary command everywhere (README, docs/windows-cli.md, site quick start, release notes): `powershell -ExecutionPolicy ByPass -c "irm https://github.com/OpenLorraine/clorraine/releases/download/v1.3.0/install.ps1 \| iex"`. Delegated details (see AR-46 note): the versioned release URL (v1.3.0 is a pre-release, so `releases/latest` would not reach it); options and scripted installs keep the download-verify-`-File` form, which is unchanged | ✅ Resolved |

## Coverage Map

> **Coverage**: 4/4 clusters · 12/12 categories · 34 candidates (15 material after merge) · 4 residual limits · Effort: high/high/high/xhigh→inherited (no provider variant) · Independence: uniform-effort
> **Freshness**: Coverage as of 2026-11-27; artifacts: this register (AR-1..AR-24), issue #38, tools/vscode, tools/setup installers/packaging/acceptance, compiler starter and LSP serverInfo, docs/ and site/, workflows

| Cluster | Area | Examined | Not examined / limit | Candidates | Notes / limit | Confidence |
|---------|------|----------|----------------------|------------|---------------|------------|
| C1 Product and scope | extension, starter, installers, acceptance, docs/site, issue #38 | full read of named files; v1.2.0 starter via git | live VS Code behaviour; install.ps1 summary rendering | 9 → AR-25, 27, 28, 32, 34, 35, 36, 37, 38, 39 | `general` agent substituted for `preflight-auditor` | high |
| C2 Behavior and edges | `${command:}`, `--force`/pinning, acceptance, drift, re-runs | upstream VS Code source (pinned metadata, auto-update) | Pester suites; live Marketplace | 7 → AR-25, 26, 28, 29, 32; CV-006 re-run folded into AR-30; CV-007 (code+code-insiders) optional, omitted (strict) | `general` substituted | high |
| C3 Technical and data | vsce 4.1.0 internals, workflows, packaging, acceptance schema, version inventory | vsce source; host rasterizers | runner image; docker image ImageMagick | 10 → AR-25, 27, 28, 29, 30, 32, 34, 36, 38; CV-010 re-run folded into AR-30 | `general` substituted | high |
| C4 Risk and stakeholders | OIDC exposure, integrity, recovery, timeouts, provenance, listing | lockfile, workflows, installers | Marketplace byte identity after signing; vsce-sign need; `--force` vs newer installed | 8 → AR-26, 28, 29, 30, 31, 33, 38; CV-008 (offline doc) optional, omitted (strict) | `general` substituted; external doc check | medium-high |

**Source note:** run-wide CV-001 = C1-CV1, C2-CV3, C3-CV2 · CV-002 = C2-CV1, C4-CV4 · CV-003 = C1-CV2, C3-CV4 · CV-004 = C1-CV3, C2-CV2, C3-CV3, C4-CV6 · CV-005 = C2-CV5, C3-CV1, C4-CV2 · CV-006 = C4-CV1, C3-CV6, C2-CV6, C3-CV10 · CV-007 = C4-CV3 · CV-008 = C2-CV4, C3-CV5 · CV-009 = C4-CV5 · CV-010 = C1-CV4, C3-CV9 · CV-011 = C1-CV8 · CV-012 = C1-CV6, C3-CV8 · CV-013 = C1-CV5 · CV-014 = C1-CV7, C3-CV7, C4-CV7 · CV-015 = C1-CV9. No Discovery keys.

### Resolution Notes

**AR-5:** The VS Code publishing documentation states that global Personal Access Tokens are retired on 2026-12-01 and recommends GitHub Actions Trusted Publishing: a policy on the publisher names the repository, the workflow file, and the GitHub environment; the job requests an OIDC token (`id-token: write`) and runs `vsce publish --oidc`. The repository is public, so the 48-hour activation window for private repositories does not apply.

**AR-12:** The user asked why reviewers were needed. The only collaborator and organization member is `blendsdk` (admin), GitHub lets the person who started a run approve it, and an agent using the same `gh` login could approve through the API. The environment is still required: the Trusted Publishing policy names it, and it carries the `v*` tag rule.

**AR-40 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: implementation sequencing; stays inside AR-17's release order and creates no publication: nothing user-visible on the site or README changes before the release
Objective: execute with --auto-commit (push after each verified task) without deploying release text early, while letting the publish workflow be dispatched
Decision: execute on branch `release-1.3.0` in a separate worktree; --auto-commit pushes go to `origin/release-1.3.0`. Phase 1 (the publish workflow) is the only work fast-forwarded to `main` before the release, so `workflow_dispatch` can run dry runs from `main`. At release the branch is rebased onto `origin/main` locally and pushed to `main` as a fast-forward (no force push); the tag goes on that release commit; the remote branch is then deleted
Evidence: .github/workflows/docs.yml:3-10 deploys on pushes to main touching site/**; README.md is the public repository front page; GitHub runs workflow_dispatch only for workflows present on the default branch; AGENTS.md: main stays linear (PR #96 was rebase-merged)
Rejected alternatives: commit to main (publishes v1.3.0 install links and Marketplace claims before the release exists); a long-lived PR merged at release (same outcome, but a merge or squash breaks the per-task history the plan relies on)
Strongest counterargument: a rebase at release rewrites the branch's commit IDs, so per-task commit IDs recorded in the execution plan refer to the pre-rebase branch
Confidence: High — would change if the other session needs the extension or installer changes on main before the release
Hardening: in-context review; no blind challenger (reversible sequencing choice, no publication)
Policy version: 1
Root invocation ID: ad-20261127-vsmkt
Reopen triggers: main gains conflicting edits to the same files; the user wants the extension changes on main early
```

**AR-41 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: failure and recovery design inside AR-13 (dry_run exists) and AR-12 (dispatch is the approval)
Objective: a dispatch never publishes by accident
Decision: `dry_run` is a boolean input with default `true`; publishing requires unticking it on a tag dispatch
Evidence: Marketplace versions are permanent (AR-29); the publish job is the only irreversible step
Rejected alternatives: default `false` (one click from the tag publishes)
Strongest counterargument: one extra click on the real publish
Confidence: High
Hardening: in-context
Policy version: 1
Root invocation ID: ad-20261127-vsmkt
Reopen triggers: the user wants one-click publishing
```

**AR-42 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: internal interface and testing strategy for the approved AR-29 check
Objective: the version guard is tested before it guards an irreversible publish
Decision: `tools/vscode/scripts/release-version.mjs --ref <ref> --cargo <Cargo.toml> --manifest <package.json> [--publish]`, exit 0 on match, exit 1 with a one-line reason; the workflow passes `github.ref` through `env:`; covered by `node --test` like the other extension scripts
Evidence: tools/vscode/scripts/stage.mjs already reads compiler/Cargo.toml; `npm test` runs `node --test test/*.test.ts`
Rejected alternatives: inline shell in the workflow (untestable before the first real publish)
Strongest counterargument: one more script file
Confidence: High
Hardening: in-context
Policy version: 1
Root invocation ID: ad-20261127-vsmkt
Reopen triggers: the version of record moves out of compiler/Cargo.toml
```

**AR-43 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: testing strategy for the approved AR-33 bound
Objective: prove the bound fires and the install continues, in seconds
Decision: both installers read `CLORRAINE_EDITOR_TIMEOUT` (whole seconds, default 300; a non-numeric or zero value falls back to 300); tests set it to 1 with a fake editor that sleeps
Evidence: the installer suites already drive fake `code` launchers (tools/setup/tests/install_fixtures.sh)
Rejected alternatives: no automated test (the bound would ship unproven)
Strongest counterargument: a documented-nowhere environment variable in the installer
Confidence: High
Hardening: in-context
Policy version: 1
Root invocation ID: ad-20261127-vsmkt
Reopen triggers: the user objects to the override being user-settable
```

**AR-44 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: data validation inside AR-28 (records bind one candidate)
Objective: a record from the old schema can never pass for a new candidate
Decision: `acceptance-check` rejects a record that has a `vsix` key or lacks the `extension` key, naming the key
Evidence: tools/setup/acceptance.py:47-60 already rejects unknown and missing keys; no committed record exists
Rejected alternatives: ignoring `vsix` (an old record could look valid)
Strongest counterargument: none material; old records only bind old candidates
Confidence: High
Hardening: in-context
Policy version: 1
Root invocation ID: ad-20261127-vsmkt
Reopen triggers: a committed historical record must stay checkable
```

**AR-45 (delegated):**

```text
Authority: AI — delegated by --auto-design
Eligibility: implementation mechanisms inside AR-18, AR-28, AR-35, AR-6, and AR-27; no product scope change
Objective: make the approved behaviour explicit and testable
Decision: (a) the manifest declares `virtualWorkspaces: false`, stating the existing file-only behaviour (actions require file URIs); (b) an explicit invalid path setting also gets the "Open install guide" action, since the fix is still to install or point at a valid tool; (c) Linux acceptance records `manual.editor_build_run` and takes `EDITOR_CONFIRMED=1`, mirroring Windows `-EditorConfirmed`; (d) `package.sh --verify` rejects any extra `.vsix` in the output so a stale file cannot ride along; (e) when `--uninstall-extension` itself fails, the installer stops the VS Code step with `failed` and prints the `--force` command, leaving the old extension in place
Evidence: tools/vscode/src/actions.ts:204 (file URIs only); tools/setup/acceptance.py:55,230-232 (Windows manual keys); package.sh --verify already checks the asset list
Rejected alternatives: (a) omit the field (the listing would claim virtual-workspace support by default); (b) plain error for invalid settings; (c) automated LSP/Build-Run in acceptance (needs an editor-driving harness, a complexity escalation); (d) ignore extra files; (e) attempt the install anyway (two failures, unclear state)
Strongest counterargument: (a) and (d) are small additions the user did not name
Confidence: High
Hardening: in-context
Policy version: 1
Root invocation ID: ad-20261127-vsmkt
Reopen triggers: the user objects to any of (a)–(e)
```

**AR-46 (runtime, user decision with delegated details):** Verified on the Windows test host
(PowerShell 5.1.26100) before adopting it: `install.ps1` is ASCII without a BOM, so `irm` decodes
it unchanged; it parses as an in-memory script block with no errors; `iex` loads it; and a script
block bound with `-Help` prints usage and exits 0. The script never reads its own path. Its final
`exit` ends only the child `powershell -c` process and passes the installer's exit code to the
caller, so the documented command keeps the `powershell … -c` wrapper; pasting only
`irm … | iex` into an open PowerShell window would close that window when the installer ends.

```text
Authority: AI — delegated by --auto-design (details only; the command style is the user's)
Eligibility: documentation presentation inside the user's instruction; no installer behaviour change
Decision: versioned release-asset URL; keep the "verify the download first" section and the `-File` form for options and scripted installs
Rejected alternatives: `releases/latest/download/…` (v1.3.0 is a pre-release; v1.1.2 is Latest); a copy on the GitHub Pages site (a second copy to keep byte-identical); passing options through `[scriptblock]::Create((irm …))` (works, but harder to read than the existing `-File` form)
Confidence: High
Policy version: 1
Root invocation ID: ad-20261127-vsmkt
Reopen triggers: a stable release becomes Latest and the docs should track it; the user wants options in the one-liner
```
