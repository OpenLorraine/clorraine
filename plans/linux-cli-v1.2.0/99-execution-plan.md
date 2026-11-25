# Execution Plan: Linux CLI v1.2.0

> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-25 00:01
> **Progress**: 28/28 tasks (100%)
>
> **Revision 2 (2026-11-25)**: the follow-up
> `plans/public-preview-v1.2.0/99-execution-plan.md` published the public
> `v1.2.0` prerelease and extended the installer to offer, verify, and install
> the pinned AmigaPorts rc24 toolchain, then run the host hello. The
> local-candidate boundary below records the earlier stage and is superseded by
> that plan.
> **CodeOps Artifact Schema**: 1

## Overview

Deliver the Linux x86_64 1.2.0 CLI-first path: install the CLI without Docker,
validate the user-downloaded AmigaPorts rc24 toolchain, build `.clo` programs,
and run them in FS-UAE with a licensed KS3.1 A1200 ROM. The editor uses the
same CLI. The old native Python installer/support pipeline is removed only
after a local candidate passes acceptance.

The upstream toolchain and FS-UAE behavior are already proven in
`spikes/07-linux-cli-fs-uae/README.md`. This plan turns that evidence into the
product path.

**🚨 Update this document after EACH completed task.**

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 1 | Toolchain selection and build | 7 |
| 2 | Verified FS-UAE guest run | 7 |
| 3 | Installer, candidate archive, editor | 9 |
| 4 | Acceptance and legacy removal | 5 |

**Total: 28 tasks across 4 phases**

> Mark `[~]` on implementation, `[x]` only after the task verify passes, `[!]`
> with `Blocked:` when stopped. Spec-red tasks stay `[~]` until green.

## Phase 1: Toolchain selection and build

> **Phase baseline tree**: `ba82931ba75c2a428633aa01bb8320ebfafb3430`
> **Expected modification set**: `compiler/src/clorraine/{cli,mod,environment,setup,toolchain,build,run}.rs`, `compiler/src/clorraine/native_metadata.rs` (retired), `compiler/src/clorraine/emulator.rs` (retired until Phase 2), the toolchain/setup, legacy-driver, and DX message tests under `compiler/tests/`, `tools/setup/toolchain_check.sh`, `Makefile`, `AGENTS.md`, `README.md`, `docs/`, `site/` (including `guide/known-limitations.md`), `RELEASE-NOTES.md`, `CHANGELOG.md`, `RELEASE.md`, the legacy distribution documentation assertion, roadmap/progress metadata, and this plan
> **Scope**: strict
> **Reasoning**: high — the public toolchain contract replaces the old SDK
> assembly and touches every build path

### Step 1.1: Specification tests first

- [x] 1.1.1 [spec-author] ✅ (completed: 2026-11-24 17:04) Add `compiler/tests/toolchain_setup_spec_test.rs` covering ST-7–ST-11: invalid roots, both-crt probe, atomic config, resolver precedence (including marker-gated legacy PATH), and absolute compiler invocation. Rewrite or retire the old `native_driver_spec_test.rs`/`native_driver_impl_test.rs` cases under AR-16; keep the legacy handoff test red/green contract explicit.
  - **Task verify**: `cargo test --test toolchain_setup_spec_test` fails for the intended missing behavior; the old native spec/impl cases are explicitly rewritten or retired; record the red output.

### Step 1.2: CLI, config, validator, resolver

- [x] 1.2.1 ✅ (completed: 2026-11-24 17:04) Add the `setup` command in `compiler/src/clorraine/setup.rs`, `--toolchain` on `setup`/`build`/`run`, config parsing and atomic write, and remove `--sdk-root`/`CLORRAINE_SDK`; update help and usage tests.
  - **Task verify**: CLI/config unit tests pass; `--sdk-root` is a usage error.
- [x] 1.2.2 ✅ (completed: 2026-11-24 17:04) Implement the full validator: canonical root, compiler/`m68k-amigaos` checks, glibc 2.38 message, rc24 `--version`, and both `-mcrt` probes; leave the old config untouched on failure.
  - **Task verify**: ST-7–ST-9 spec cases pass, including the probe failure case.
- [x] 1.2.3 ✅ (completed: 2026-11-24 17:04) Replace the old native-assembly resolver with flag > env > config; keep the marker-gated legacy `PATH`/handoff branch for the Docker SDK and its tests; pass the absolute compiler for native selections; keep the existing flag tables; remove the bundled venv; keep `vamos` as a `PATH` tool for non-emulator runs; rewrite/remove the old `native_driver_support`, `native_gate`, and marker tests; keep the legacy handoff tests green.
  - **Task verify**: ST-10–ST-11 spec cases pass; a host build needs no toolchain.
- [x] 1.2.4 ✅ (completed: 2026-11-24 17:15) Add `make toolchain-check TOOLCHAIN=PATH` to build hello, the four corpus programs, and the freestanding skeleton with rc24; run it against the pinned asset.
  - **Task verify**: hello, all four corpus programs, and the freestanding skeleton build; record the commands and sizes.

### Step 1.3: Documentation and gate

- [x] 1.3.1 ✅ (completed: 2026-11-24 17:15) Document the rc24 URL/size/SHA-256, glibc 2.38+ requirement, setup/config, wrapper oracle, KS3.1 limitation, and A500-vs-A1200 caveat in `docs/`, the root README, `site/`, `RELEASE-NOTES.md`, `CHANGELOG.md`, and `RELEASE.md`; update `docs/known-limitations.md` and the `AGENTS.md` command table for `toolchain-check`.
  - **Task verify**: `make check` and `make site-check` pass; every new command in the docs is copy-paste valid; the claims grep finds no positive Ubuntu 22.04 support, `~/.clorraine` path, bundled SDK/`vamos`, or installer-placed VSIX claim (negative “cannot” statements are allowed).
- [x] 1.3.2 ✅ (completed: 2026-11-24 17:15) Phase gate: run `make check`, `make site-check`, and `make toolchain-check TOOLCHAIN=<pinned rc24>`; record the baseline tree and evidence.
  - **Task verify**: both commands pass; no `--sdk-root` remains in the CLI/compiler code or tests. The editor update is tracked in Phase 3.

### Phase 1 evidence

- **Red:** `cargo test --test toolchain_setup_spec_test` failed 6 of 8 cases with
  `unknown command 'setup'`; the captured log is `build/tmp/toolchain-setup-red.log`.
- **Spec green:** 13/13 setup/selection cases pass, including the glibc floor,
  both runtime probes, config preservation, precedence, no-fallback, the
  native-emulator phase gate, and the `vamos` dependency.
- **Real toolchain:** `setup` on rc24 reported
  `m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b 20261010182711` and stored its
  canonical root in a temporary XDG config.
- **Real builds:** `make toolchain-check` built hello (430,000 bytes),
  file-copier (440,708), string-parser (437,272), buffer-container (437,872),
  volatile-mmio (429,732), and freestanding (8,160).
- **Review fixes:** the post-phase review’s major findings were fixed; the
  added failure-path tests (probe timeout and config-rename cleanup) are green.
  The only red log remains the initial public-path run; the fix tests target the
  corrected failure paths and are recorded here rather than re-simulated.
- **Gates:** `make check`, `make site-check`, and the real rc24
  `make toolchain-check` pass; logs are under `build/tmp/phase1-*.log`.

**Deliverables**: setup command, validator, resolver, build path, docs, real build evidence.
**Phase gate**: `make check` + `make site-check` + `make toolchain-check`.

## Phase 2: Verified FS-UAE guest run

> **Phase baseline tree**: `052694c6131b584400096fb87c1ee45de728d008`
> **Expected modification set**: `compiler/src/clorraine/emulator/` (new) and `compiler/src/clorraine/{mod,cli,environment,artifacts,run}.rs`, `compiler/runtime/emulator_runner.c` (new), `compiler/tests/fs_uae_spec_test.rs`, `compiler/tests/fs_uae_selection_spec_test.rs`, and `compiler/tests/fs_uae_support/` (new), `compiler/tests/toolchain_setup_spec_test.rs`, `tools/setup/fs_uae_check.sh` (new), `Makefile`, `docs/linux-cli.md`, `site/` (CLI/emulator/limitations pages), `README.md`, `RELEASE-NOTES.md`, `CHANGELOG.md`, roadmap/progress metadata, and this plan
> **Scope**: strict
> **Reasoning**: high — process lifecycle, guest marker, cleanup, and output
> correctness

### Step 2.1: Specification tests first

- [x] 2.1.1 [spec-author] ✅ (completed: 2026-11-24 21:05) Add `compiler/tests/fs_uae_spec_test.rs` and `compiler/tests/fs_uae_selection_spec_test.rs` with fake `fs-uae` and fake `m68k-amigaos-gcc` seams covering ST-12–ST-17: version/ROM/timeout errors, wrapper staging/compile, rc 1/5/9, partial status, read/capture failure, bounded wait, and read-before-cleanup.
  - **Task verify**: the new test fails for the intended missing behavior; record the red output.

### Step 2.2: Runner

- [x] 2.2.1 Resolve FS-UAE (`--fs-uae` > `CLORRAINE_FS_UAE` > PATH), require `3.2.0` or newer with `3.2.35` as the tested output, validate a 512 KB KS3.1 A1200 ROM (524,299 tested; 524,288 accepted size), validate the 1–3600 timeout, and reject 256 KB with the A1200 message. ✅ (completed: 2026-11-24 21:08)
  - **Task verify**: ST-12 cases pass.
- [x] 2.2.2 Stage the private 0700 drive with the compiled `runner`, `program`, `CNEXT:runner` startup, A1200 config, and isolated emulator directories; compile `compiler/runtime/emulator_runner.c` to `drive/runner` with the native compiler, `-mcrt=nix20`, `-O2 -Wall -Wextra -Werror`, and `-lamiga -lm`. ✅ (completed: 2026-11-24 21:08)
  - **Task verify**: ST-13 staging test passes; no old `program\n` script remains.
- [x] 2.2.3 Implement the rc/status poll, child stdout/stderr redirection, read-before-cleanup, bounded kill/wait, read/capture failure retention, and cleanup; success requires rc 0 plus status `ok`; honor the 90-second default and reject invalid timeout values. Spawn FS-UAE directly, never as a background `xvfb-run` child, and verify no emulator process remains. ✅ (completed: 2026-11-24 21:08)
  - **Task verify**: ST-14–ST-16 pass, including the timeout and cleanup-ordering cases.
- [x] 2.2.4 Add `--emulator --capture` writing the guest output to `out.txt` before cleanup; remove the old rejection; accept both Amiga profiles. ✅ (completed: 2026-11-24 21:08)
  - **Task verify**: ST-17 passes and the existing A500-profile CLI test still passes.
- [x] 2.2.5 Add `make fs-uae-check KICKSTART=PATH`; run hello, the four `hosted-amiga` corpus programs, one `hosted-amiga-13` corpus, and a temporary rc-1 negative via the candidate driver on a real A1200/KS3.1 FS-UAE; compare outputs and require the negative to exit 1. ✅ (completed: 2026-11-24 21:22)
  - **Task verify**: the real guest output is `Hello, Amiga, from Clorraine!`; the rc-1 negative exits 1 with no status; record the FS-UAE and ROM versions/hashes.

### Step 2.3: Gate

- [x] 2.3.1 Phase gate: run `make check` and `make fs-uae-check KICKSTART=<KS3.1 A1200>`; run the four corpus programs as guests and compare expected output. ✅ (completed: 2026-11-24 21:23)
  - **Task verify**: all four corpus outputs match; one `hosted-amiga-13` corpus run matches; the rc-1 negative fails; no emulator run remains after the command.

### Phase 2 evidence

- **Spec red:** `cargo test --test fs_uae_spec_test` failed 19 of 19 cases
  because the native runner is unavailable; the captured log is
  `build/tmp/fs-uae-red.log`. Selection-error cases also fail because their
  messages name the new validation, not the phase-boundary rejection.
- **Spec green:** 19/19 runner cases pass, including selection, the two ROM
  sizes, staging and wrapper flags, CR-trimmed output, rc 1/5/9, partial and
  malformed markers, prompt exit, timeout, termination, read/capture failure,
  noisy emulator streams, capture ordering, and the A500 profile.
- **Scoped checks:** 12/12 toolchain setup cases and 56 driver unit tests pass;
  `make check` passes. The runtime choices AR-19–AR-26 are recorded in
  `00-decisions.md`, and `03-02-fs-uae.md` matches the implemented loop.
- **Commit:** the four runner tasks share one module and land in one commit
  (`feat(cli): run verified FS-UAE guests`). Capture is part of the same
  observe/capture/cleanup path, so splitting it would leave a non-building
  intermediate commit.
- **Module split:** the runner now lives in
  `compiler/src/clorraine/emulator/{mod,select,storage,observe,diagnostics}.rs`
  (each under 300 lines); the spec suite and `make check` stay green.
- **Real guests:** `make fs-uae-check` passed hello, file-copier,
  string-parser, buffer-container, volatile-mmio, the hosted-amiga-13
  file-copier, and the rc-1 negative on FS-UAE 3.2.35 with rc24 and the
  licensed 524,299-byte KS3.1 A1200 ROM (SHA-256 `541f7467…1bb0`). No FS-UAE
  or Xvfb process survived. The log is `build/tmp/fs-uae-check.log`.
- **Phase gate:** `make check`, `make site-check`, and the real
  `make fs-uae-check` pass after the docs update. The final logs are
  `build/tmp/phase2-final-{check,site,fs-uae}.log`.
- **Review:** correctness returned no critical/major (ledger RV-020–RV-024); the
  Amiga specialist found one major gate-process finding (SR-001) and two minor
  documentation findings. SR-001 is fixed with a required `pgrep`, a clean
  process start, and an empty-process assertion after every invocation; the
  minor and observation findings are fixed. Fix logs are
  `build/tmp/phase2-fix-*.log`. A fix-diff re-review is the final review pass.

**Deliverables**: verified FS-UAE runner, capture, real guest evidence.
**Phase gate**: `make check` + `make fs-uae-check` + corpus guest diff.

## Phase 3: Installer, candidate archive, editor

> **Phase baseline tree**: `227a8804de40c9a8dc567ece04c48e61e555e58d`
> **Expected modification set**: `tools/setup/{install.sh,package.sh}`, `tools/setup/tests/{install.spec.test.sh,install.impl.test.sh,dist.spec.test.sh,install_fixtures.sh}`, `Makefile`, `.github/workflows/check.yml`, `tools/vscode/src/{discovery,actions}.ts`, `tools/vscode/test/*` and fixtures, `tools/vscode/package.json`, `tools/vscode/README.md`, `README.md`, `docs/`, `site/`, `RELEASE-NOTES.md`, `CHANGELOG.md`, `RELEASE.md`, `AGENTS.md`, roadmap/progress metadata, and this plan
> **Scope**: strict
> **Reasoning**: high — download integrity, atomic activation, and editor
> executable selection

### Step 3.1: Installer specification tests first

- [x] 3.1.1 [spec-author] Add `install.spec.test.sh`, `install.impl.test.sh`, and `dist.spec.test.sh` with fixture stage trees and fake `uname`/`getconf`/`curl`/`sha256sum` covering ST-1–ST-6, ST-18, ST-19, and ST-24. ✅ (completed: 2026-11-24 22:03)
  - **Task verify**: the tests fail for the intended missing behavior; the fixture covers producer version/extra-file/stale-pin failures and installer host checks; record the red output.

### Step 3.2: Candidate and installer

- [x] 3.2.1 Add `make dist VERSION=1.2.0` and `make dist-verify`: build binaries in the pinned container, validate VERSION against Cargo/VSIX, build the VSIX, generate release README/notice, package the exact layout with `tools/setup/package.sh`, produce `SHA256SUMS` covering archive/VSIX/installer, assemble `install.sh` with pinned bytes/hash, and check all pins/contents; no publication. ✅ (completed: 2026-11-24 22:07)
  - **Task verify**: `make dist-verify` passes; the archive contains no toolchain, ROM, FS-UAE, or Python helper.
- [x] 3.2.2 Implement host checks (Linux/x86_64, glibc 2.38, conditional `curl`, fake-test seams), network download with HTTPS-only/redirect/timeout/max-size flags, exact byte/hash verification, and the local direct-hash `--archive/--checksums` mode. ✅ (completed: 2026-11-24 22:07)
  - **Task verify**: ST-1–ST-5 pass; a bad hash/size never extracts.
- [x] 3.2.3 Implement member-type validation, PID+starttime lock with stale retry, unique `versions/<version>-<sha12>` directories, atomic `current` switch, rollback, prefix-safe symlinks, quoted/replaced PATH block, XDG config resolution, and `--uninstall [--purge]`. ✅ (completed: 2026-11-24 22:07)
  - **Task verify**: ST-6 passes, including interrupted activation, live/stale/concurrent locks, PATH quoting/replacement, and purge with/without XDG.
- [x] 3.2.4 Add `make dist-check` to `make check`; update `.github/workflows/check.yml` to Ubuntu 24.04 and the hermetic installer tests; keep `setup-check` for the Docker SDK installer; add `dist-check` to the `AGENTS.md` command table. ✅ (completed: 2026-11-24 22:07)
  - **Task verify**: `make check` and `make dist-check` pass with no network or Docker.

### Step 3.3: Editor

- [x] 3.3.1 First extend `discovery.spec.test.ts`, `actions.spec.test.ts`, `actions.review.spec.test.ts`, `manifest.test.ts`, and the shared fixtures with failing cases; then update discovery to `~/.local/bin`, add `clorraine.toolchain.path`, remove `sdk.path`, implement new-key precedence and the persistent legacy warning, and update actions to one `--toolchain` flag. ✅ (completed: 2026-11-24 22:21)
  - **Task verify**: ST-21–ST-23 pass; the review spec and fixtures are updated; a second activation does not warn again.
- [x] 3.3.2 Update `package.json`, `tools/vscode/README.md`, and site/release docs for `toolchain.path`, the VSIX install command, and the legacy warning; remove `--sdk-root` from docs. ✅ (completed: 2026-11-24 22:21)
  - **Task verify**: ST-22/ST-23 pass; `make vscode-check` and `make site-check` pass.

### Step 3.4: Local install end-to-end and gate

- [x] 3.4.1 Install the candidate into a temporary HOME with `--archive/--checksums`, run `setup --toolchain`, build hello, run it in FS-UAE, and record the archive/installer/VSIX hashes and output. ✅ (completed: 2026-11-24 22:21)
  - **Task verify**: ST-20 passes from the candidate archive, not a checkout.
- [x] 3.4.2 Phase gate: run `make check`, `make dist-check`, `make dist-verify`, `make vscode-check`, and `make site-check`; re-run the local install E2E. ✅ (completed: 2026-11-24 22:21)
  - **Task verify**: all pass; the installed `clorraine --version` is `1.2.0`.

### Phase 3 evidence

- **Spec red:** `install.spec.test.sh` failed 25 checks against the old
  bootstrap template; `dist.spec.test.sh` failed 33 checks because the
  packaging script was absent. Logs are `build/tmp/install-spec-red.log` and
  `build/tmp/dist-spec-red.log`.
- **Installer green:** 81 public installer checks and 21 internal checks pass.
- **Producer green:** 36 packaging checks pass, including exact layout,
  version mismatch, extra/forbidden files, stale installer pins, missing VSIX,
  SHA256SUMS coverage, and release README links.
- **Candidate:** `make dist` wrote the versioned archive, VSIX, `install.sh`,
  and `SHA256SUMS`; `make dist-verify` passes. The final asset identity is
  recorded in the local E2E bullet below.
- **Gates:** `make dist-check` and `make check` pass; CI runs on Ubuntu 24.04
  with `make dist-check` and `make setup-check`.
- **Supersession:** replacing `tools/setup/install.sh` invalidated the old
  native/bootstrap tests that pinned its slots, so `make check` stopped calling
  `distribution-check` and `setup-check` now runs only the Docker SDK tests
  (`test_cli`, `test_detect`, `test_fetch`). The old target and files are
  removed after acceptance in Phase 4.
- **Editor:** 174 editor tests pass, including `~/.local/bin` discovery,
  `toolchain.path`, one `--toolchain` argument, the one-time legacy warning,
  the manifest settings, and the packaged client. `make vscode-check` passes.
- **Local E2E:** the final candidate installed into a temp HOME from its exact
  archive, `setup --toolchain` accepted rc24, and the installed
  `clorraine 1.2.0` ran hello, file-copier, string-parser, buffer-container,
  volatile-mmio, the hosted-amiga-13 file-copier, and the rc-1 negative on
  FS-UAE 3.2.35. The log records `no fs-uae/Xvfb survivor` after every guest
  and `processes after: none`. Assets (final build): archive 6,530,342 bytes
  `7c43a330…80f0`, installer 20,193 bytes `f9924eb5…2478`, VSIX 26,435 bytes
  `9385b870…6904`; ROM `541f7467…1bb0`. Log
  `build/tmp/phase3-local-e2e-final2.log`.
- **Phase gate:** `make check`, `make dist-check`, `make dist-verify`,
  `make vscode-check`, `make site-check`, and the installed-candidate E2E pass.
  `build/dist` contains the archive, VSIX, installer, and checksums. The
  installed driver reports `clorraine 1.2.0`.
- **Review:** correctness found one major (purge coverage) and a set of
  minor test/hardening gaps; security found four minor installer hardening
  findings; the Amiga specialist found three minor evidence/docs findings. All
  are fixed: 83 installer spec checks, 21 internal checks, 36 producer checks,
  174 editor tests, and the final E2E with explicit survivor evidence. The
  fix-diff re-review found no critical/major; its four residual observations
  (purge assertion, test-root registry, EXIT-trap cleanup, design command list)
  are fixed and recorded without a third review pass.
- **Boundary:** the candidate is local; no tag, release, upload, or publication
  occurred.

**Deliverables**: installer, archive, editor update, local candidate E2E.
**Phase gate**: `make check` + `make dist-check` + `make vscode-check` + E2E.

## Phase 4: Acceptance and legacy removal

> **Phase baseline tree**: `261344e6eca285e256e8176e4b1e5248e5823200`
> **Expected modification set**: `tools/setup/{acceptance.py,acceptance.sh,install_sdk.py}`, `tools/setup/tests/{acceptance_spec.py,test_retained_inputs.py}`, the removed native installer/tests and `tools/distribution/` producers/tests (except `inputs/`), `.github/workflows/` release definitions, `Makefile`, `AGENTS.md`, `README.md`, `docs/`, `site/`, `RELEASE.md`, `RELEASE-NOTES.md`, `CHANGELOG.md`, `THIRD_PARTY_NOTICES.md`, roadmap/progress metadata, and this plan
> **Scope**: strict
> **Reasoning**: high — destructive removal and release evidence must follow
> proof

### Step 4.1: Acceptance

- [x] 4.1.1 [spec-author] Add `acceptance_spec.py` first: valid record, wrong platform/arch, wrong toolchain tag/hash, old FS-UAE, wrong hello/corpus output, missing VSIX, and candidate-hash mismatch; then implement `make acceptance-check EVIDENCE=... CANDIDATE=...`. ✅ (completed: 2026-11-24 22:59)
  - **Task verify**: the spec fails before implementation and passes after; ST-25 cases pass.
- [x] 4.1.2 Add `make acceptance` to install the exact candidate into a fresh HOME, verify and extract the supplied toolchain archive against the rc24 pin, run setup/build/FS-UAE for hello, four `hosted-amiga` corpus programs, one `hosted-amiga-13` corpus, and an rc-1 negative; assert no FS-UAE/Xvfb process remains after each run; write `evidence.json` with archive/installer/VSIX/toolchain-archive/ROM/FS-UAE/negative/corpus/output hashes. ✅ (completed: 2026-11-24 23:01)
  - **Task verify**: ST-20 and the evidence schema pass; the output is the expected hello line.

### Step 4.2: Remove the old pipeline

- [x] 4.2.1 First add `test_retained_inputs.py` red; then remove the old native installer, native tests, support/distribution producers/tests, and release workflows; keep `install_sdk.py`, its tests, `tools/distribution/inputs/`, and the notices; remove `distribution-check` and keep `dist-check` in `make check`; update CI, `AGENTS.md`, site, release notes, changelog, and retained README refs. ✅ (completed: 2026-11-24 23:02)
  - **Task verify**: ST-26–ST-28 pass; retained inputs/notices/README refs resolve; no dangling old target, workflow, site, or release-note command.

### Step 4.3: Re-verify and close

- [x] 4.3.1 Re-run `make check`, `make dist-check`, `make acceptance-check EVIDENCE=<file> CANDIDATE=<dir>`, `make site-check`, and `make verify`; confirm the removal did not change compiler behavior. ✅ (completed: 2026-11-24 23:07)
  - **Task verify**: all pass; `make verify` covers the existing A500/A1200/RTG gates. If the pinned SDK image or media is unavailable, record the blocker and do not mark the task complete.
- [x] 4.3.2 Final review: update the index/roadmap to complete, confirm no publication/tag/upload occurred, and write the publication handoff note. ✅ (completed: 2026-11-24 23:46)
  - **Task verify**: the plan tree is clean, the roadmap is current, and the publication boundary is explicit.

### Phase 4 evidence

- **Acceptance spec red:** `PYTHONPATH=tests python3 -m unittest acceptance_spec`
  failed with `ModuleNotFoundError: No module named 'acceptance'`; log
  `build/tmp/acceptance-spec-red.log`.
- **Acceptance spec green:** 19/19 cases pass, covering the valid record,
  platform/arch/glibc, toolchain tag/hash/bytes, FS-UAE floor, hello output and
  hash, guest rc, corpus hash/profile/set, negative, missing VSIX, candidate
  archive/installer mismatch, and missing fields.
- **Target:** `make acceptance-check EVIDENCE=... CANDIDATE=...` runs the
  checker; `make dist-check` runs the spec hermetically.
- **Acceptance run:** `make acceptance` installed the exact candidate into a
  fresh HOME, verified and extracted the pinned rc24 archive, and ran hello,
  the four hosted-amiga corpus programs, the hosted-amiga-13 file-copier, and
  the rc-1 negative on FS-UAE 3.2.35. Every guest matched and every run left
  no FS-UAE/Xvfb survivor. Evidence:
  `build/acceptance/20261124T220050Z/evidence.json`; `make acceptance-check`
  reports `ok`. The evidence records glibc 2.43, the candidate hashes, the
  rc24 and ROM pins, and the corpus hashes.
- **Removal red/green:** `test_retained_inputs.py` failed 4 cases before the
  removal (old paths and `distribution-check` present); after removal 9/9
  pass. The retained `libamiga.a` hash, notices, retained README, removed
  paths, new Makefile targets, and current-doc claims are checked. `make
  dist-check` runs 28 tests; `make setup-check` runs 29; `make check` and
  `make site-check` pass.
- **Full gate:** `make verify` passes on a private Xvfb
  (`DISPLAY=:99`, `-ac`), printing `verify: all legs passed`; log
  `build/tmp/phase4-verify-xvfb.log`. The first attempt on the desktop
  Xwayland display failed the existing `sdk-a500-paddle` capture because the
  host X auth was rotated; the same gate passes under the private Xvfb, which
  is the documented SDK-display path.
- **Phase 4 review:** correctness returned no critical/major (RV-047–RV-053);
  security returned no critical/major (SA-006–SA-015). The evidence directory
  is now self-contained; the acceptance script uses private copies of the
  toolchain/installer; the checker requires exact asset names and rejects
  symlinks; the fake-emulator result/rc ordering race is fixed. `make
  dist-check` runs 37 tests; the re-run acceptance evidence is
  `build/acceptance/20261124T225355Z/evidence.json` and `acceptance-check`
  reports `ok`.
- **Closeout:** `06-publication-handoff.md` records the exact candidate,
  evidence, gates, and approval boundary. No tag, release, upload, or
  publication occurred; existing tags stop at v1.1.2.

**Deliverables**: acceptance evidence, old pipeline removed, full gate, closeout.
**Phase gate**: `make acceptance-check` + `make check` + `make verify`.

## Definition of done

- A fresh Linux x86_64 host with glibc 2.38+ can install the candidate archive
  without Docker, validate the rc24 toolchain, build hello, and run it in FS-UAE
  with a licensed KS3.1 A1200 ROM.
- The guest result is read from the exact candidate bytes and recorded with all
  hashes.
- The four corpus programs pass as real guests.
- `make check`, `make dist-check`, `make vscode-check`, `make site-check`,
  `make acceptance-check`, and `make verify` pass.
- The old native installer/support pipeline is removed; retained inputs and
  notices resolve.
- No tag, release, upload, or publication occurs.

## Publication boundary

The plan ends with a local candidate and evidence. Publishing a GitHub release
requires a separate explicit approval and a follow-up release plan.
