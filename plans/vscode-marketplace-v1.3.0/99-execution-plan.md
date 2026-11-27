# Execution Plan: VS Code Marketplace Extension and the v1.3.0 Release

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-27 18:55
> **Progress**: 51/55 tasks (92%)
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

- [x] 1.3.1 Write impl tests — `tools/vscode/test/release-version.impl.test.ts`; phase gate `make vscode-check` ✅ (completed: 2026-11-27 15:00) — 8 impl tests; make vscode-check passed (203 tests + audit + packaged client); workflow audit call fixed to --vsix
- [x] 1.3.2 **approval** Fast-forward `main` to the branch head (plan docs + Phase 1 only) and push (AR-40) ✅ (completed: 2026-11-27 15:12) — origin/main fast-forwarded to the branch head
- [x] 1.3.3 **approval** Dispatch `publish-vscode` from `main` with `dry_run=true`; build and audit pass, publish job skipped; record the run ID here ✅ (completed: 2026-11-27 15:13) — run 46013860719: build success (guard 1.2.0, audit ok, 27,589-byte VSIX artifact), publish skipped

**Verify (gate)**: `make vscode-check`

---

## Phase 2: Extension listing, driver command, guide action, drift notice

> **Phase baseline tree**: f0ddd83060361f2ce33a07015c1cb6254819b645
> **Reasoning**: high — user-visible listing and new extension behaviour

### Step 2.1: Spec tests

**Reference**: 03-01 · AR-9, AR-16, AR-18, AR-32, AR-35, AR-36, AR-38

- [x] 2.1.1 [spec-author] Write spec tests from ST-11..ST-20 — `tools/vscode/test/marketplace.spec.test.ts`, `guide.spec.test.ts`, `driver-command.spec.test.ts` (extend the fake-host fixtures as needed) ✅ (completed: 2026-11-27 15:16)
- [x] 2.1.2 Red phase: run the three spec files; they fail ✅ (completed: 2026-11-27 15:16) — 10/10 spec tests fail for missing behaviour

### Step 2.2: Implementation

- [x] 2.2.1 Listing files — `tools/vscode/{README.md,DEVELOPMENT.md,CHANGELOG.md,images/icon.png}` (03-01 item 6) ✅ (completed: 2026-11-27 15:21)
- [x] 2.2.2 Manifest, staging, and audit — `tools/vscode/{package.json,scripts/stage.mjs,scripts/audit_vsix.py}` (03-01 items 1, 7) ✅ (completed: 2026-11-27 15:21)
- [x] 2.2.3 Guide helpers, install-guide error at the three sites, `clorraine.driverPath`, drift notice — `tools/vscode/src/{guide.ts,extension.ts,actions.ts}` (03-01 items 2–5) ✅ (completed: 2026-11-27 15:21)
- [x] 2.2.4 Green phase: ST-11..ST-20 pass; update `manifest.test.ts` / `packaged.test.ts` only where they pin the intentionally changed manifest or file list ✅ (completed: 2026-11-27 15:21) — 218 tests: 217 pass, 1 skipped (packaged client needs make vscode-check); no existing pin changed

### Step 2.3: Impl tests and gate

- [x] 2.3.1 Write impl tests — `tools/vscode/test/guide.impl.test.ts`, audit negatives ✅ (completed: 2026-11-27 15:21) — 7 impl tests
- [x] 2.3.2 Phase gate `make vscode-check` ✅ (completed: 2026-11-27 15:24) — make vscode-check: 224 tests pass, audit ok (19 entries), packaged client ok
- [x] 2.3.3 Package once and review the staged listing (README, CHANGELOG, icon) by eye; record the VSIX size here ✅ (completed: 2026-11-27 15:24) — VSIX 41,947 bytes, 19 entries; vsixmanifest: Public Preview, ExtensionKind workspace, banner #0c1219, tags and links as intended

**Verify (gate)**: `make vscode-check`

---

## Phase 3: Portable starter tasks

> **Reasoning**: medium — small template change plus a spec-test rewrite under AR-25

### Step 3.1: Spec tests

**Reference**: 03-03 §Starter · AR-8, AR-25

- [x] 3.1.1 [spec-author] Rewrite the starter tasks assertion from ST-21 — `compiler/tests/first_run_spec_test.rs` ✅ (completed: 2026-11-27 15:30)
- [x] 3.1.2 Red phase: run `first_run_spec_test`; it fails ✅ (completed: 2026-11-27 15:30) — the rewritten starter spec test fails; the other four pass

### Step 3.2: Implementation

- [x] 3.2.1 Template and writer — `compiler/templates/starter/tasks.json`, `compiler/src/clorraine/new.rs`; starter README if it names the launcher ✅ (completed: 2026-11-27 15:32)
- [x] 3.2.2 Remove `launcher_for`, its re-export, its unit tests, and its Windows spec/impl tests — `compiler/src/clorraine/{new.rs,mod.rs}`, `compiler/tests/windows_cli_{spec,impl}_test.rs` (AR-25) ✅ (completed: 2026-11-27 15:32)
- [x] 3.2.3 Green phase: ST-21 passes on host; `cargo check --target x86_64-pc-windows-gnu` passes ✅ (completed: 2026-11-27 15:32) — first_run 12, windows_cli_spec 32, windows_cli_impl 25, clorraine unit 68; Windows cargo check clean

### Step 3.3: Gate

- [x] 3.3.1 Phase gate `make check IMAGE=clorraine-sdk:windows-cli` ✅ (completed: 2026-11-27 15:39) — make check passed: 4,529 Rust tests + spec, setup, dist, bench, fuzz suites; review fixes re-verified

**Verify (gate)**: `make check`

---

## Phase 4: Installers install from the Marketplace

> **Reasoning**: high — two installers, user-editor side effects, timeout handling

### Step 4.1: Spec tests

**Reference**: 03-03 §Installers · AR-6, AR-7, AR-14, AR-15, AR-26, AR-27, AR-33, AR-43

- [x] 4.1.1 [spec-author] Rewrite the VS Code cases from ST-22..ST-29 (Linux) — `tools/setup/tests/install.spec.test.sh`, fixtures in `install_fixtures.sh` ✅ (completed: 2026-11-27 15:48)
- [x] 4.1.2 [spec-author] Rewrite the VS Code cases from ST-22..ST-29 (Windows) — `tools/setup/tests/install.spec.test.ps1` ✅ (completed: 2026-11-27 15:48)
- [x] 4.1.3 Red phase: Linux suite locally; PowerShell suite on the Windows host (`windows-remote.sh push` + `run` in the desktop session); both fail on the new cases ✅ (completed: 2026-11-27 15:48) — Linux: 15 new assertions fail; Windows: 7 of 8 new cases fail (the -NoVSCode case already holds); existing cases pass

### Step 4.2: Implementation

- [x] 4.2.1 Linux installer — `tools/setup/install.sh` (detection, bounded editor helper, flow, states, summary, help) ✅ (completed: 2026-11-27 15:52)
- [x] 4.2.2 Windows installer — `tools/setup/install.ps1` (same, with `Invoke-Native` timeout and process-tree kill) ✅ (completed: 2026-11-27 15:52)
- [x] 4.2.3 Green phase: both suites pass on their hosts ✅ (completed: 2026-11-27 15:52) — Linux spec 196 checks; Linux impl 26; Windows spec 36/36; Windows impl 35/35

### Step 4.3: Impl tests and gate

- [x] 4.3.1 Write impl tests — `tools/setup/tests/install.impl.test.sh`, `install.impl.test.ps1` (timeout parsing, call order, summary text); run both ✅ (completed: 2026-11-27 15:53) — Linux impl 38 checks; Windows impl 37/37
- [x] 4.3.2 Phase gate: the four installer suites (Linux spec/impl, Windows spec/impl on the host); `make check` moves to the Phase 5 gate (AR-48) ✅ (completed: 2026-11-27 15:54) — Linux spec 196 + impl 38 checks; Windows spec 36/36 + impl 37/37; dist-check deferred to the phase 5 gate (5 expected packager failures)

**Verify (gate)**: installer suites on Linux and the Windows host (AR-48); `make check` at the Phase 5 gate

---

## Phase 5: Four-asset candidate and acceptance evidence

> **Reasoning**: high — release gate and evidence schema

### Step 5.1: Spec tests

**Reference**: 03-04 · AR-6, AR-28, AR-34, AR-44

- [x] 5.1.1 [spec-author] Write/rewrite spec tests from ST-29..ST-36 — `tools/setup/tests/{dist.spec.test.sh,windows-dist.spec.test.sh,acceptance_spec.py,acceptance_windows_spec.py}` ✅ (completed: 2026-11-27 16:03)
- [x] 5.1.2 Red phase: run those suites; the new cases fail ✅ (completed: 2026-11-27 16:03) — dist spec 36 and windows-dist 24 failures (every build needs --vsix today); acceptance 10 failures and 4 errors

### Step 5.2: Implementation

- [x] 5.2.1 Packaging — `Makefile` (`dist`), `tools/setup/package.sh` ✅ (completed: 2026-11-27 16:07)
- [x] 5.2.2 Acceptance — `tools/setup/acceptance.{py,sh,ps1}`, `Makefile` (`acceptance` inputs) ✅ (completed: 2026-11-27 16:07)
- [x] 5.2.3 Green phase: ST-29..ST-36 pass ✅ (completed: 2026-11-27 16:07) — dist spec 50, windows-dist spec 33, acceptance specs OK

### Step 5.3: Impl tests and gate

- [x] 5.3.1 Update impl tests — `tools/setup/tests/{windows-dist.impl.test.sh,acceptance_windows_impl.py}`; phase gate `make check IMAGE=clorraine-sdk:windows-cli`; one local `make dist` + `make dist-verify` shows four assets ✅ (completed: 2026-11-27 16:18) — impl suites pass; make check passed (4,529 Rust tests + setup/dist/bench/fuzz); make dist wrote 4 assets + 4-line SHA256SUMS; dist-verify passed; review fixes re-verified

**Verify (gate)**: `make check`

---

## Phase 6: Docs sweep, release text, version bump

> **Reasoning**: medium — wide but mechanical; must not reach `main` before the release

**Reference**: 03-05 §Docs sweep, §Release text, §Version bump · AR-34, AR-36, AR-37, AR-26

- [x] 6.1.1 Repository docs — `README.md`, `docs/{README,linux-cli,windows-cli,known-limitations}.md`, `AGENTS.md`, `plans/windows-cli/08-merge-notes.md` (03-05 §Docs sweep) ✅ (completed: 2026-11-27 16:19)
- [x] 6.1.2 Site guide pages — `site/guide/{quick-start,known-limitations,roadmap}.md`, `site/status.md` ✅ (completed: 2026-11-27 16:20)
- [x] 6.1.3 Site project pages and theme — `site/project/{brief,contributing}.md`, `site/.vitepress/theme/components/HomePage.vue`, `site/.vitepress/theme/Layout.vue`; sweep grep returns nothing ✅ (completed: 2026-11-27 16:20) — sweep grep empty; make site-check passed
- [x] 6.1.4 Release text — `CHANGELOG.md`, `RELEASE-NOTES.md`, `RELEASE.md`, `tools/vscode/CHANGELOG.md` (1.3.0 entry final) ✅ (completed: 2026-11-27 16:22) — CHANGELOG 1.3.0, RELEASE-NOTES (checksums marker for 7.1.4), RELEASE.md runbook; retained-inputs guard updated (AR-49)
- [x] 6.1.5 Version bump to 1.3.0 — `compiler/Cargo.toml`, `Cargo.lock`, `tools/vscode/package.json`, `tools/vscode/package-lock.json` ✅ (completed: 2026-11-27 16:22) — Cargo.toml, Cargo.lock, package.json, package-lock.json (own entries only); guard accepts v1.3.0
- [x] 6.1.6 Docs/test pins that read the README or guides still pass (`docs_guide_spec_test`, `first_run_spec_test`, `windows_cli_spec_test`) ✅ (completed: 2026-11-27 16:36) — docs_guide, first_run, windows_cli spec tests pass; no other 1.2.0 pin as the current version
- [x] 6.1.7 Phase gate `make check IMAGE=clorraine-sdk:windows-cli`, `make vscode-check`, `make site-check` ✅ (completed: 2026-11-27 16:36) — make check (4,529), make vscode-check (224), make site-check; review fixes re-verified incl. dist-check and Windows spec 37/37

**Verify (gate)**: `make check && make vscode-check && make site-check`

---

## Phase 7: Release v1.3.0

> **Reasoning**: xhigh — irreversible publication and external actions; every outward step asks first

**Reference**: 03-05 §Release procedure · AR-10, AR-17, AR-19, AR-20, AR-28, AR-31, AR-39, AR-40

- [x] 7.1.1 Confirm the user holds other pushes to `main`; rebase `release-1.3.0` onto `origin/main`; `make verify` with the Windows-capable image on the private Xvfb display ✅ (completed: 2026-11-27 17:28; `make verify` all legs passed on 68419c7e; no rebase needed)
- [x] 7.1.2 **approval** Push tag `v1.3.0`; dispatch `publish-vscode` from the tag with `dry_run=true` and check the summary ✅ (completed: 2026-11-27 17:52; tag v1.3.0 at d3f5cf84; dry run 46037087599: build success, publish skipped, VSIX 1.3.0 audited, 41,948 bytes)
- [x] 7.1.3 **approval** Dispatch from the tag with `dry_run=false`; record run ID and SHA-256; confirm the gallery lists 1.3.0 ✅ (completed: 2026-11-27 18:14; run 46040096710; VSIX 41,948 bytes, SHA-256 68c8785de819e7ce48bb400d1a72add1b906f15d9a760b72cc65cf68a0b4e377; the gallery lists 1.3.0 public/preview and serves the same bytes)
- [x] 7.1.4 `make dist` and `make dist-verify`; record the four asset hashes in `RELEASE-NOTES.md` (checksums section) ✅ (completed: 2026-11-27 18:16; four assets + SHA256SUMS, dist-verify ok; Linux archive ae398e15…, Windows zip f712d9c8…, install.sh fd50b736…, install.ps1 bcc56ec5…; table in the uncommitted notes working copy per RELEASE.md step 7)
- [x] 7.1.5 Linux acceptance with `PUBLISH_RUN`/`PUBLISH_SHA256`; user confirms the editor; rerun with `EDITOR_CONFIRMED=1`; `make acceptance-check` ✅ (completed: 2026-11-27 18:30; build/acceptance/20261127T172930Z; full FS-UAE matrix ok; extension openlorraine.clorraine 1.3.0 from the Marketplace; user confirmed LSP, Build/Run and auto-update after the v1.2.0 VSIX install; acceptance-check ok)
- [x] 7.1.6 **approval** Windows acceptance: create the standard test account over `ssh winbox`, push the candidate, run `acceptance.ps1` in the desktop session, user confirms UAC and the editor, `make acceptance-check`, remove the account and files ✅ (completed: 2026-11-27 18:55; as `Clorraine Test` on build 26100: UAC and editor confirmed, extension 1.3.0 from the Marketplace, corpus 5/5, `acceptance-check: ok`, record build/acceptance/windows-1.3.0/evidence.json sha256 91ca7448…; user ruling: keep the account, its profile and the C:\ClorraineAcceptance toolkit for future releases; rc24, C:\ProgramData\Clorraine, the candidate and the evidence removed)
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
