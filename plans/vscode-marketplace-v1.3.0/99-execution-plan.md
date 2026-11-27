# Execution Plan: VS Code Marketplace Extension and the v1.3.0 Release

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-27 14:56
> **Progress**: 6/55 tasks (10%)
> **CodeOps Artifact Schema**: 1

## Overview

Publish the extension through a Trusted Publishing workflow, switch the installers and starter to
the Marketplace, drop the VSIX from the candidate, update the docs, and release v1.3.0. Work runs
on branch `release-1.3.0` in worktree `../clorraine-release`; `--auto-commit` pushes go to that
branch; only Phase 1 reaches `main` before the release (AR-40).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Publish workflow and version guard | 9 |
| 2 | Extension listing, driver command, guide action, drift notice | 9 |
| 3 | Portable starter tasks | 6 |
| 4 | Installers install from the Marketplace | 8 |
| 5 | Four-asset candidate and acceptance evidence | 6 |
| 6 | Docs sweep, release text, version bump | 7 |
| 7 | Release v1.3.0 | 10 |

**Total: 55 tasks across 7 phases**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes below are the **single source of truth** for progress. Every task line
> appears exactly once. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** and the Last Updated stamp after EVERY task.
> 4. **Resume** at the first `[~]` task, else the first `[ ]` task.
> 5. **On blocker:** mark `[!]` and append `Blocked: <short reason>`.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'`. Gates run with
> `IMAGE=clorraine-sdk:windows-cli` while the shared `dev` image lacks the Windows target, and
> with `TMPDIR`/`CODEOPS_TMPDIR` under `build/tmp`. Steps marked **approval** stop and ask the
> user, even under `--auto-commit` (AR-12, AR-40; publication and external actions are reserved).

---

## Phase 1: Publish workflow and version guard

> **Phase baseline tree**: 78f9ab4ce37afde13047d5ff8fa2f2a913ff58e4
> **Scope mode**: strict · expected modification set: `.github/workflows/publish-vscode.yml`, `tools/vscode/scripts/release-version.mjs`, `tools/vscode/test/{publish-workflow.spec,release-version.spec,release-version.impl}.test.ts`, this plan
> **Reasoning**: xhigh — the only irreversible-publication surface; supply-chain and OIDC exposure

### Step 1.1: Branch and spec tests

**Reference**: 03-02 · AR-5, AR-12, AR-13, AR-29, AR-30, AR-40..AR-42

- [x] 1.1.1 Confirm worktree `../clorraine-release` on `release-1.3.0` (created with the plan commit) is up to date with `origin/main`; install `tools/vscode` dependencies ✅ (completed: 2026-11-27 14:54) — worktree at origin/main + plan commit; npm ci done
- [x] 1.1.2 [spec-author] Write spec tests from ST-1..ST-10 — `tools/vscode/test/publish-workflow.spec.test.ts`, `tools/vscode/test/release-version.spec.test.ts` ✅ (completed: 2026-11-27 14:55)
- [x] 1.1.3 Red phase: run the two spec files; they fail (no workflow, no script) ✅ (completed: 2026-11-27 14:55) — 11/11 spec tests fail: workflow and script missing

### Step 1.2: Implementation

- [x] 1.2.1 Implement the version guard — `tools/vscode/scripts/release-version.mjs` (03-02 §Version guard) ✅ (completed: 2026-11-27 14:55)
- [x] 1.2.2 Implement the workflow — `.github/workflows/publish-vscode.yml` (03-02 §Proposed Changes; look up and pin the artifact actions) ✅ (completed: 2026-11-27 14:56)
- [x] 1.2.3 Green phase: ST-1..ST-10 pass ✅ (completed: 2026-11-27 14:56) — 11/11 pass

### Step 1.3: Impl tests, landing, dry run

- [ ] 1.3.1 Write impl tests — `tools/vscode/test/release-version.impl.test.ts`; phase gate `make vscode-check`
- [ ] 1.3.2 **approval** Fast-forward `main` to the branch head (plan docs + Phase 1 only) and push (AR-40)
- [ ] 1.3.3 **approval** Dispatch `publish-vscode` from `main` with `dry_run=true`; build and audit pass, publish job skipped; record the run ID here

**Verify (gate)**: `make vscode-check`

---

## Phase 2: Extension listing, driver command, guide action, drift notice

> **Reasoning**: high — user-visible listing and new extension behaviour

### Step 2.1: Spec tests

**Reference**: 03-01 · AR-9, AR-16, AR-18, AR-32, AR-35, AR-36, AR-38

- [ ] 2.1.1 [spec-author] Write spec tests from ST-11..ST-20 — `tools/vscode/test/marketplace.spec.test.ts`, `guide.spec.test.ts`, `driver-command.spec.test.ts` (extend the fake-host fixtures as needed)
- [ ] 2.1.2 Red phase: run the three spec files; they fail

### Step 2.2: Implementation

- [ ] 2.2.1 Listing files — `tools/vscode/{README.md,DEVELOPMENT.md,CHANGELOG.md,images/icon.png}` (03-01 item 6)
- [ ] 2.2.2 Manifest, staging, and audit — `tools/vscode/{package.json,scripts/stage.mjs,scripts/audit_vsix.py}` (03-01 items 1, 7)
- [ ] 2.2.3 Guide helpers, install-guide error at the three sites, `clorraine.driverPath`, drift notice — `tools/vscode/src/{guide.ts,extension.ts,actions.ts}` (03-01 items 2–5)
- [ ] 2.2.4 Green phase: ST-11..ST-20 pass; update `manifest.test.ts` / `packaged.test.ts` only where they pin the intentionally changed manifest or file list

### Step 2.3: Impl tests and gate

- [ ] 2.3.1 Write impl tests — `tools/vscode/test/guide.impl.test.ts`, audit negatives
- [ ] 2.3.2 Phase gate `make vscode-check`
- [ ] 2.3.3 Package once and review the staged listing (README, CHANGELOG, icon) by eye; record the VSIX size here

**Verify (gate)**: `make vscode-check`

---

## Phase 3: Portable starter tasks

> **Reasoning**: medium — small template change plus a spec-test rewrite under AR-25

### Step 3.1: Spec tests

**Reference**: 03-03 §Starter · AR-8, AR-25

- [ ] 3.1.1 [spec-author] Rewrite the starter tasks assertion from ST-21 — `compiler/tests/first_run_spec_test.rs`
- [ ] 3.1.2 Red phase: run `first_run_spec_test`; it fails

### Step 3.2: Implementation

- [ ] 3.2.1 Template and writer — `compiler/templates/starter/tasks.json`, `compiler/src/clorraine/new.rs`; starter README if it names the launcher
- [ ] 3.2.2 Remove `launcher_for`, its re-export, its unit tests, and its Windows spec/impl tests — `compiler/src/clorraine/{new.rs,mod.rs}`, `compiler/tests/windows_cli_{spec,impl}_test.rs` (AR-25)
- [ ] 3.2.3 Green phase: ST-21 passes on host; `cargo check --target x86_64-pc-windows-gnu` passes

### Step 3.3: Gate

- [ ] 3.3.1 Phase gate `make check IMAGE=clorraine-sdk:windows-cli`

**Verify (gate)**: `make check`

---

## Phase 4: Installers install from the Marketplace

> **Reasoning**: high — two installers, user-editor side effects, timeout handling

### Step 4.1: Spec tests

**Reference**: 03-03 §Installers · AR-6, AR-7, AR-14, AR-15, AR-26, AR-27, AR-33, AR-43

- [ ] 4.1.1 [spec-author] Rewrite the VS Code cases from ST-22..ST-29 (Linux) — `tools/setup/tests/install.spec.test.sh`, fixtures in `install_fixtures.sh`
- [ ] 4.1.2 [spec-author] Rewrite the VS Code cases from ST-22..ST-29 (Windows) — `tools/setup/tests/install.spec.test.ps1`
- [ ] 4.1.3 Red phase: Linux suite locally; PowerShell suite on the Windows host (`windows-remote.sh push` + `run` in the desktop session); both fail on the new cases

### Step 4.2: Implementation

- [ ] 4.2.1 Linux installer — `tools/setup/install.sh` (detection, bounded editor helper, flow, states, summary, help)
- [ ] 4.2.2 Windows installer — `tools/setup/install.ps1` (same, with `Invoke-Native` timeout and process-tree kill)
- [ ] 4.2.3 Green phase: both suites pass on their hosts

### Step 4.3: Impl tests and gate

- [ ] 4.3.1 Write impl tests — `tools/setup/tests/install.impl.test.sh`, `install.impl.test.ps1` (timeout parsing, call order, summary text); run both
- [ ] 4.3.2 Phase gate `make dist-check` and `make check IMAGE=clorraine-sdk:windows-cli`

**Verify (gate)**: `make check` (includes `dist-check`) + the PowerShell suites on the Windows host

---

## Phase 5: Four-asset candidate and acceptance evidence

> **Reasoning**: high — release gate and evidence schema

### Step 5.1: Spec tests

**Reference**: 03-04 · AR-6, AR-28, AR-34, AR-44

- [ ] 5.1.1 [spec-author] Write/rewrite spec tests from ST-29..ST-36 — `tools/setup/tests/{dist.spec.test.sh,windows-dist.spec.test.sh,acceptance_spec.py,acceptance_windows_spec.py}`
- [ ] 5.1.2 Red phase: run those suites; the new cases fail

### Step 5.2: Implementation

- [ ] 5.2.1 Packaging — `Makefile` (`dist`), `tools/setup/package.sh`
- [ ] 5.2.2 Acceptance — `tools/setup/acceptance.{py,sh,ps1}`, `Makefile` (`acceptance` inputs)
- [ ] 5.2.3 Green phase: ST-29..ST-36 pass

### Step 5.3: Impl tests and gate

- [ ] 5.3.1 Update impl tests — `tools/setup/tests/{windows-dist.impl.test.sh,acceptance_windows_impl.py}`; phase gate `make check IMAGE=clorraine-sdk:windows-cli`; one local `make dist` + `make dist-verify` shows four assets

**Verify (gate)**: `make check`

---

## Phase 6: Docs sweep, release text, version bump

> **Reasoning**: medium — wide but mechanical; must not reach `main` before the release

**Reference**: 03-05 §Docs sweep, §Release text, §Version bump · AR-34, AR-36, AR-37, AR-26

- [ ] 6.1.1 Repository docs — `README.md`, `docs/{README,linux-cli,windows-cli,known-limitations}.md`, `AGENTS.md`, `plans/windows-cli/08-merge-notes.md` (03-05 §Docs sweep)
- [ ] 6.1.2 Site guide pages — `site/guide/{quick-start,known-limitations,roadmap}.md`, `site/status.md`
- [ ] 6.1.3 Site project pages and theme — `site/project/{brief,contributing}.md`, `site/.vitepress/theme/components/HomePage.vue`, `site/.vitepress/theme/Layout.vue`; sweep grep returns nothing
- [ ] 6.1.4 Release text — `CHANGELOG.md`, `RELEASE-NOTES.md`, `RELEASE.md`, `tools/vscode/CHANGELOG.md` (1.3.0 entry final)
- [ ] 6.1.5 Version bump to 1.3.0 — `compiler/Cargo.toml`, `Cargo.lock`, `tools/vscode/package.json`, `tools/vscode/package-lock.json`
- [ ] 6.1.6 Docs/test pins that read the README or guides still pass (`docs_guide_spec_test`, `first_run_spec_test`, `windows_cli_spec_test`)
- [ ] 6.1.7 Phase gate `make check IMAGE=clorraine-sdk:windows-cli`, `make vscode-check`, `make site-check`

**Verify (gate)**: `make check && make vscode-check && make site-check`

---

## Phase 7: Release v1.3.0

> **Reasoning**: xhigh — irreversible publication and external actions; every outward step asks first

**Reference**: 03-05 §Release procedure · AR-10, AR-17, AR-19, AR-20, AR-28, AR-31, AR-39, AR-40

- [ ] 7.1.1 Confirm the user holds other pushes to `main`; rebase `release-1.3.0` onto `origin/main`; `make verify` with the Windows-capable image on the private Xvfb display
- [ ] 7.1.2 **approval** Push tag `v1.3.0`; dispatch `publish-vscode` from the tag with `dry_run=true` and check the summary
- [ ] 7.1.3 **approval** Dispatch from the tag with `dry_run=false`; record run ID and SHA-256; confirm the gallery lists 1.3.0
- [ ] 7.1.4 `make dist` and `make dist-verify`; record the four asset hashes in `RELEASE-NOTES.md` (checksums section)
- [ ] 7.1.5 Linux acceptance with `PUBLISH_RUN`/`PUBLISH_SHA256`; user confirms the editor; rerun with `EDITOR_CONFIRMED=1`; `make acceptance-check`
- [ ] 7.1.6 **approval** Windows acceptance: create the standard test account over `ssh winbox`, push the candidate, run `acceptance.ps1` in the desktop session, user confirms UAC and the editor, `make acceptance-check`, remove the account and files
- [ ] 7.1.7 **approval** `gh release create v1.3.0 --prerelease` with the four assets, `SHA256SUMS`, and `RELEASE-NOTES.md` as the body
- [ ] 7.1.8 **approval** Push the release commit to `main` (fast-forward); delete `origin/release-1.3.0`; confirm the site shows v1.3.0; fresh installs from the published release on Linux and Windows
- [ ] 7.1.9 **approval** Close #38, #57, #59, #60, #65 with notes; open the Open VSX follow-up issue (AR-39)
- [ ] 7.1.10 Draft the AmigaDev announcement for the user; update roadmap row T-05 and this plan; remove the worktree

**Verify (gate)**: `make verify`; both acceptance records pass `make acceptance-check`

---

## Dependencies

```
Phase 1 (workflow; lands on main, dry run)
    ↓
Phase 2 (extension) ──→ Phase 3 (starter uses the driver command)
    ↓                      ↓
Phase 4 (installers) ←─────┘
    ↓
Phase 5 (candidate + acceptance)
    ↓
Phase 6 (docs, release text, version)
    ↓
Phase 7 (release)
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ `make verify` passes on the release commit; both acceptance records pass
3. ✅ No warnings/errors
4. ✅ No dead code (`launcher_for` and the VSIX pin paths removed)
5. ✅ Publication hardened: build/publish split, version guard, SHA re-check, no stored secret
6. ✅ Documentation updated; the sweep grep is empty
7. ✅ Phase reviews passed (strict profile)
8. ✅ Post-completion project re-analysis (exec-plan)
