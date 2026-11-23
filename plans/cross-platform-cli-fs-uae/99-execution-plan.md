# Execution Plan: Cross-platform CLI and FS-UAE delivery

> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-23 16:20
> **Progress**: 0/43 tasks (0%)
> **CodeOps Artifact Schema**: 1

## Overview

Deliver the Linux v1.2.0 CLI-first path: install our CLI, validate a
user-downloaded upstream Amiga toolchain, build programs and run them in FS-UAE.
macOS arm64 and Windows x64 are deferred to a separate follow-up plan. The old
Python installer/support pipeline is removed in Phase 4.

**🚨 Update this document after EACH completed task.**

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 1 | Feasibility and pins | 4 |
| 2 | CLI setup, toolchain resolver and Linux installer | 12 |
| 3 | FS-UAE, AROS and Linux editor | 10 |
| 4 | Linux v1.2.0 release, docs and migration | 9 |
| 5 | Linux acceptance and publication | 8 |

**Total: 43 tasks across 5 phases**

> Mark `[~]` on implementation, `[x]` only after the task verify passes, `[!]`
> with `Blocked:` when stopped. Spec-red tasks stay `[~]` until green.

## Phase 1: Feasibility and pins

> **Phase baseline tree**: recorded at execution start
> **Reasoning**: high — toolchain compatibility can invalidate the design

- [ ] 1.1.1 Download/extract the upstream Linux rc24 toolchain and build/run hello for `hosted-amiga` and `hosted-amiga-13` in FS-UAE with a user ROM; record the exact tag/hash and result.
- [ ] 1.1.2 Verify FS-UAE's no-ROM AROS launch behavior and record that it is launch-only, not guest proof.
- [ ] 1.1.3 Record the exact toolchain tag/hash and Linux spike evidence; stop for a plan revision if Linux cannot build or run.
- [ ] 1.1.4 Run the existing compiler corpus through the upstream toolchain (host compile plus one Amiga link/run path) and record the exact command/result; stop if it fails.

**Deliverables**: spike evidence; exact upstream tag.
**Task verify**: evidence files present and reviewed.
**Verify (gate)**: Linux hello build/run evidence.

## Phase 2: CLI setup, toolchain resolver and Linux installer

> **Phase baseline tree**: recorded at execution start
> **Reasoning**: high — install, config, resolver and rollback

### Step 2.1: Specification tests

**Reference**: 07 ST-1–ST-15; 03-01; AR-4/5/6/7/9/19/26/28/30/31

- [ ] 2.1.1 [spec-author] Write setup/config/resolver specs — `compiler/tests/setup_spec_test.rs`
- [ ] 2.1.2 [spec-author] Write bootstrap/install specs — `tools/setup/tests/test_bootstrap_spec.py`
- [ ] 2.1.3 Run the new specs red with the targeted commands and record the failures — `07-testing-strategy.md`

**Deliverables**: ST-1–ST-15 red tests.
**Task verify**: `cargo test --test setup_spec_test` and the bootstrap test file, expected nonzero; log recorded.
**Verify (gate)**: red evidence for both files.

### Step 2.2: Implementation

**Reference**: 03-01 §Commands/§Layout/§Toolchain validation/§Resolver/§Bootstrap; 03-02 §Linux

- [ ] 2.2.1 Add `setup` and its flags/help; convert both binary entry points to `args_os()` with usage errors.
- [ ] 2.2.2 Implement home/config validation (absolute, no duplicates, 0700/0600, no root), toolchain validation and completion-marker activation.
- [ ] 2.2.3 Change `environment.rs` to accept the upstream `bin/` root and the precedence `--toolchain/--sdk-root > env > config > adjacent`, preserving the old installed layout.
- [ ] 2.2.4 Implement FS-UAE/ROM config discovery and write config only after a successful hello build.
- [ ] 2.2.5 Replace `install.sh` with the Linux download/verify/run bootstrap, absolute CLI invocation, side-by-side version/marker and lock.
- [ ] 2.2.6 Implement failed-smoke rollback, same-version replacement and uninstall/PATH reversal behavior.
- [ ] 2.2.7 Run the setup and bootstrap specs green.

**Deliverables**: setup, resolver, installers, tests.
**Task verify**: `cargo test --test setup_spec_test`; `python3 -m unittest -v test_bootstrap_spec` in `tools/setup/tests`.
**Verify (gate)**: `make check`

### Step 2.3: Implementation tests and hardening

**Reference**: 07 Implementation tests; 03-01 §Error handling

- [ ] 2.3.1 Add setup implementation tests — `compiler/tests/setup_impl_test.rs`
- [ ] 2.3.2 Run `make check` and the existing native-driver suite; record the exact command.

**Deliverables**: green setup/ha/resolver tests.
**Task verify**: `make check`
**Verify (gate)**: `make check`

## Phase 3: FS-UAE, AROS and Linux editor

> **Phase baseline tree**: recorded at execution start
> **Reasoning**: high — primary user workflow

### Step 3.1: Specification tests

**Reference**: 07 ST-16–ST-29; 03-03; AR-11/12/13/18/21/29

- [ ] 3.1.1 [spec-author] Write build/FS-UAE/AROS/vamos specs — `compiler/tests/platform_port_spec_test.rs`
- [ ] 3.1.2 [spec-author] Write editor discovery/action specs — `tools/vscode/test/discovery.spec.test.ts`, `actions.spec.test.ts`
- [ ] 3.1.3 Run the new specs red with targeted commands and record the failures.

**Deliverables**: ST-16–ST-29 red tests.
**Task verify**: targeted Rust/Node commands, expected nonzero; logs recorded.
**Verify (gate)**: red evidence.

### Step 3.2: Implementation

**Reference**: 03-03 §Discovery/§AROS/§Run/§Editor

- [ ] 3.2.1 Implement optional ROM/AROS launch-only fallback and the single FS-UAE precedence (flag/env/config/PATH/defaults).
- [ ] 3.2.2 Replace the bundled `vamos` requirement with optional PATH lookup and per-OS messages.
- [ ] 3.2.3 Update extension discovery/actions to read `<home>/config`, use the existing machine path setting and migrate/remove duplicate SDK/FS-UAE/ROM settings.
- [ ] 3.2.4 Package and audit the VSIX.
- [ ] 3.2.5 Run the FS-UAE/editor specs green and a Linux private-display run with a user ROM.

**Deliverables**: FS-UAE and editor client working on Linux.
**Task verify**: `make check`; `make vscode-check`
**Verify (gate)**: `make vscode-check`; private-display FS-UAE evidence.

### Step 3.3: Implementation tests and hardening

- [ ] 3.3.1 Add platform/editor implementation tests — `compiler/tests/platform_port_impl_test.rs`, `tools/vscode/test/actions.impl.test.ts`
- [ ] 3.3.2 Run `make vscode-check` and the Linux FS-UAE evidence command; record paths/hashes.

**Deliverables**: green editor/emulator tests.
**Task verify**: `make vscode-check`
**Verify (gate)**: `make vscode-check`; `make verify` (documented private display/media prerequisites).

## Phase 4: Linux v1.2.0 release, docs and migration

> **Phase baseline tree**: recorded at execution start
> **Reasoning**: high — distribution and destructive removal

### Step 4.1: Specification tests

**Reference**: 07 ST-30–ST-33; 03-04; AR-16/22/27/28

- [ ] 4.1.1 [spec-author] Write release/workflow/asset/removal specs — `tools/release/tests/test_release_spec.py`
- [ ] 4.1.2 Run the release specs red and record the failures.

**Deliverables**: release ST tests; `release-check` target plan.
**Task verify**: `python3 -m unittest -v test_release_spec` in `tools/release/tests`, expected nonzero.
**Verify (gate)**: red evidence.

### Step 4.2: Implementation

**Reference**: 03-04 §Release/§Docs/§Migration

- [ ] 4.2.1 Add `.github/workflows/release.yml` with the Linux v1.2.0 hosted build, read-only jobs, one draft upload and checksums.
- [ ] 4.2.2 Add candidate staging from the draft and approval-hash binding.
- [ ] 4.2.3 Update README, quick-start, RELEASE, release notes, site status/brief/roadmap, known limitations and notices.
- [ ] 4.2.4 Remove the old installer/support/workflows; update Makefile (`distribution-check` removal, hermetic `release-check` addition), `check.yml`, `AGENTS.md` and notices.
- [ ] 4.2.5 Run the release specs green under `release-check`.

**Deliverables**: Linux release workflow, docs, removal.
**Task verify**: `make release-check` (new target)
**Verify (gate)**: `make check`, `make site-check`

### Step 4.3: Implementation tests and hardening

- [ ] 4.3.1 Add release implementation/negative tests — `tools/release/tests/test_release_impl.py`
- [ ] 4.3.2 Run `make check`, `make verify`, `make vscode-check`, `make site-check`; independent release review.

**Deliverables**: green full gates; review clean.
**Task verify**: `make check`
**Verify (gate)**: all four gates.

## Phase 5: Linux acceptance and publication

> **Phase baseline tree**: recorded at execution start
> **Reasoning**: max — real evidence and publication authority

- [ ] 5.1.1 [spec-author] Write acceptance/evidence checks — `tools/release/tests/test_acceptance_spec.py`
- [ ] 5.1.2 Run acceptance checks red for absent evidence.
- [ ] 5.2.1 Build the Linux candidate from draft assets; run clean Ubuntu 22.04/24.04 and Debian 12 install/setup/build/FS-UAE with a user ROM and observable output; record the JSON evidence.
- [ ] 5.2.2 Verify candidate/evidence hashes and the guest-success record.
- [ ] 5.2.3 Request and record explicit publication approval bound to reviewed hashes.
- [ ] 5.2.4 Publish Linux assets/scripts/VSIX as a non-latest prerelease; run the anonymous-install check; only then mark the release latest.
- [ ] 5.3.1 Add acceptance evidence/hash negative tests — `tools/release/tests/test_acceptance_impl.py`
- [ ] 5.3.2 Final audit, close evidence, sync roadmap and run post-completion analysis.

**Deliverables**: published v1.2.0 Linux release and evidence.
**Task verify**: `make check`
**Verify (gate)**: `make verify`, `make vscode-check`, `make site-check`, anonymous install.

## Dependencies

```text
Phase 1
  ↓
Phase 2 (Linux setup/install)
  ↓
Phase 3 (Linux FS-UAE/editor)
  ↓
Phase 4 (Linux release/docs/removal)
  ↓
Phase 5 (Linux acceptance/publication)
```

## Success Criteria

1. ✅ All phases completed; no `[~]`/`[!]` tasks.
2. ✅ Linux v1.2.0 installed, builds host/both Amiga profiles, runs FS-UAE with user ROM and guest output.
3. ✅ `make check`, `make verify`, `make vscode-check`, `make site-check` pass.
4. ✅ Release workflow produces Linux assets/checksums/scripts/VSIX; no self-hosted/Docker.
5. ✅ Old installer/support pipeline removed; no dangling Makefile/CI/notices references.
6. ✅ macOS/Windows follow-up is deferred to a separate plan; no unverified assets in v1.2.0.
7. ✅ Docs match tested platforms; no WSL/unsupported claims.
8. ✅ Publication approval recorded and anonymous install passes.
