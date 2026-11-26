# Execution Plan: Windows CLI and installer

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-26 20:30
> **Progress**: 22/75 tasks (29%)
> **CodeOps Artifact Schema**: 1

## Overview

Make the driver, emitted C, language server, and editor work on Windows; add `install.ps1`, the
Windows candidate assets, and Windows acceptance evidence; document it. Development and builds stay
on Linux in this worktree; Windows runs go through the remote helper (AR-5, AR-21).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Windows build image, check step, remote helper | 6 |
| 2 | Emission: `#line` escaping and the freestanding shim | 7 |
| 3 | Driver on Windows | 11 |
| 4 | Language server and editor | 7 |
| 5 | Installer core | 10 |
| 6 | Installer toolchain, emulator, hellos, editor, starter | 8 |
| 7 | Packaging and acceptance evidence | 10 |
| 8 | Documentation | 5 |
| 9 | Windows acceptance and merge readiness | 11 |

**Total: 75 tasks across 9 phases**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes below are the **single source of truth** for progress. Every task line
> appears exactly once. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** and the Last Updated stamp after EVERY task. Only `[x]`
>    counts as complete.
> 4. **Resume** at the first `[~]` task, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>`.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

**Working rules for every phase**
- Work in `../clorraine-windows` on branch `windows-cli`; never edit the `main` checkout (AR-5).
- Use `IMAGE=clorraine-sdk:windows-cli` for every container command (AR-37).
- Windows commands run through `tools/setup/windows-remote.sh` in the signed-in desktop session;
  never copy ROMs or retained runs off the Windows host.
- Scratch output goes to `build/tmp`.
- Commits use the git-commit skill; pushing the branch uses the git-commit skill in push mode.

---

## Phase 1: Windows build image, check step, remote helper

> **Phase baseline tree**: a1b1d06e02cf5fcb960d1b816a5aaf1aea5ef8cf
> **Reasoning**: medium — small infrastructure change approved in AR-20/AR-21; no product behavior

### Step 1.1: Image, check step, helper

**Reference**: 03-01 §Image, §make check · 03-05 §Remote helper · AR-20, AR-21, AR-37
**Objective**: A pinned image that cross-builds Windows, a gate that compiles it, and a way to run commands on the Windows host.

- [x] 1.1.1 Append the MinGW-w64 and `x86_64-pc-windows-gnu` layer — `docker/Dockerfile` ✅ (completed: 2026-11-26 19:41)
- [x] 1.1.2 Build `clorraine-sdk:windows-cli` and confirm ST-1 — `make -C docker image IMAGE=clorraine-sdk:windows-cli` ✅ (completed: 2026-11-26 19:41) — built by layering the committed lines on `clorraine-sdk:dev` (AR-54; the from-source build fails upstream)
- [x] 1.1.3 Add `cargo check --workspace --target x86_64-pc-windows-gnu` to the check gate (ST-2) — `Makefile` ✅ (completed: 2026-11-26 19:52) — the step reports one Windows-only `unused_mut` warning in `emulator/storage.rs:104`, fixed with 3.2.4
- [x] 1.1.4 Add the non-gating remote helper and desktop-session runner — `tools/setup/windows-remote.sh`, `tools/setup/windows-desktop-run.ps1` ✅ (completed: 2026-11-26 19:44)
- [x] 1.1.5 Confirm the helper: `push` a scratch folder and `run winbox -- ver` print the expected output ✅ (completed: 2026-11-26 19:44) — also exact exit code 3, timeout 124, re-push replaces, ROM refused
- [x] 1.1.6 Phase gate — `make check IMAGE=clorraine-sdk:windows-cli` ✅ (completed: 2026-11-26 19:52)

**Deliverables**:
- [x] ST-1, ST-2 hold; the helper works against the Windows host
- [x] All verification passing

**Test slices**: N/A — full verify only
**Verify (gate)**: `make check IMAGE=clorraine-sdk:windows-cli`

---

## Phase 2: Emission — `#line` escaping and the freestanding shim

> **Phase baseline tree**: 69d6b2d1982880a5fd6c85f72b334ab304cb1f0b
> **Reasoning**: xhigh — emitted C changes on every host; emission-integrity and memory-safety review

### Step 2.1: Specification tests

**Reference**: 03-01 §`#line` escaping, §Freestanding shim · ST-3 – ST-8 · AR-16, AR-23, AR-44

- [x] 2.1.1 [spec-author] Write ST-3 – ST-8 — `compiler/tests/windows_cli_spec_test.rs` ✅ (completed: 2026-11-26 19:58) — written by the session (no spec-test-author role in this repo)
- [x] 2.1.2 Run the suite and confirm ST-3 – ST-5, ST-7, ST-8 fail (red); record why ST-6 passes before the change ✅ (completed: 2026-11-26 19:58) — red: ST-8 does not compile (`native_syscalls_for` missing); ST-3/4/5 show the raw `\`, `"`, tab; both ST-7 cases fail. ST-6 passes: plain paths must stay byte-identical, which the base already does

### Step 2.2: Implementation

**Reference**: 03-01

- [x] 2.2.1 Add `c_string_body` and use it in both `#line` writers — `compiler/src/codegen.rs`, `compiler/src/codegen/cleanup.rs` ✅ (completed: 2026-11-26 20:00)
- [x] 2.2.2 Add `native_syscalls_for`, guard the shim's syscall branches and `uintptr_t` — `compiler/src/clorraine/toolchain.rs`, `compiler/runtime/shim_freestanding.c` ✅ (completed: 2026-11-26 20:00) — plus the shim's `offsetof` through `uintptr_t` (AR-56); MinGW `-Werror` builds and the corpus program's output matches on the Windows host
- [x] 2.2.3 Run the spec suite — all of ST-3 – ST-8 pass (green) ✅ (completed: 2026-11-26 20:00) — 7/7; freestanding suite 17/17

### Step 2.3: Implementation tests and hardening

- [x] 2.3.1 Write escaping and predicate impl tests; update only the shim-text pins the AR-44 change moves — `compiler/tests/windows_cli_impl_test.rs`, existing pinned impl tests ✅ (completed: 2026-11-26 20:01) — 6/6; no existing test pins the changed shim text
- [x] 2.3.2 Phase gate — `make check IMAGE=clorraine-sdk:windows-cli` ✅ (completed: 2026-11-26 20:06)

**Deliverables**:
- [ ] Linux and m68k compiled output unchanged for ASCII paths (emission-integrity diff)
- [ ] All verification passing

**Verify (gate)**: `make check IMAGE=clorraine-sdk:windows-cli`

---

## Phase 3: Driver on Windows

> **Phase baseline tree**: 037cf1ba62795d8fa4922d20021e28b43221a601
> **Reasoning**: high — many call sites; Linux behavior must not change except `CLORRAINE_CC`

### Step 3.1: Specification tests

**Reference**: 03-02 · ST-9 – ST-26 · AR-10, AR-14, AR-29, AR-32, AR-38, AR-43, AR-45

- [x] 3.1.1 [spec-author] Write ST-9 – ST-26 — `compiler/tests/windows_cli_spec_test.rs` ✅ (completed: 2026-11-26 20:25)
- [x] 3.1.2 Run the suite and confirm ST-9 – ST-26 fail (red) ✅ (completed: 2026-11-26 20:25) — the platform module and the four re-exports do not resolve; ST-17 (override ignored) and ST-18 (exit 0, not 1) fail at run time

### Step 3.2: Implementation

**Reference**: 03-02 §New Functions/Methods, §OS-parameterized entry points, §Call-site changes

- [x] 3.2.1 Add `platform.rs` with `HostOs` and its functions — `compiler/src/clorraine/platform.rs`, `compiler/src/clorraine/mod.rs` ✅ (completed: 2026-11-26 20:29)
- [x] 3.2.2 Wire the validator, display paths, and the config file — `setup.rs` ✅ (completed: 2026-11-26 20:29)
- [x] 3.2.3 Wire the PATH search and the host-compiler resolver — `environment.rs`, `toolchain.rs`, `mod.rs` ✅ (completed: 2026-11-26 20:29)
- [x] 3.2.4 Wire FS-UAE selection, the PE version, spawn, and storage messages — `emulator/select.rs`, `emulator/mod.rs`, `emulator/storage.rs` (also remove the Windows-only `unused_mut` at `storage.rs:104`) ✅ (completed: 2026-11-26 20:29)
- [x] 3.2.5 Wire host artifact names and the `new` launcher and summary — `artifacts.rs`, `new.rs` ✅ (completed: 2026-11-26 20:29)
- [x] 3.2.6 Run the spec suite — ST-9 – ST-26 pass (green); existing driver suites unchanged ✅ (completed: 2026-11-26 20:29) — spec 25/25, lib 483, 11 driver suites unchanged; Windows check has no warnings

### Step 3.3: Implementation tests and hardening

- [x] 3.3.1 Write driver impl tests (PE bounds, verbatim forms, resolver errors, launcher fallbacks) — `compiler/tests/windows_cli_impl_test.rs` ✅ (completed: 2026-11-26 20:30) — impl 16/16; resolver unit tests in `environment.rs` (crate-private)
- [ ] 3.3.2 Cross-build, push, and run ST-27 – ST-30 on the Windows host through the helper
- [ ] 3.3.3 Phase gate — `make check IMAGE=clorraine-sdk:windows-cli`

**Deliverables**:
- [ ] ST-9 – ST-30 hold
- [ ] All verification passing

**Verify (gate)**: `make check IMAGE=clorraine-sdk:windows-cli`

---

## Phase 4: Language server and editor

> **Phase baseline tree**: _(recorded by exec-plan)_
> **Reasoning**: high — a server/client contract on paths; silent-skip risk in the dirty-buffer guard

### Step 4.1: Specification tests

**Reference**: 03-03 · ST-31 – ST-37 · AR-38

- [ ] 4.1.1 [spec-author] Write ST-31 – ST-35 — `compiler/tests/windows_cli_spec_test.rs`; ST-36, ST-37 — `tools/vscode/test/windows.spec.test.ts`
- [ ] 4.1.2 Run both suites and confirm ST-31 – ST-37 fail (red)

### Step 4.2: Implementation

- [ ] 4.2.1 Add `parse_for`, `from_path_for`, `normalize_for` and route every key and `sourceFiles` result through them — `compiler/src/lsp/uri.rs`, `lsp/workspace.rs`, `lsp/source_files.rs`
- [ ] 4.2.2 Add `samePath` and `installedToolPath`, use them in actions and discovery, update setting descriptions — `tools/vscode/src/actions.ts`, `discovery.ts`, `package.json`
- [ ] 4.2.3 Run both suites — ST-31 – ST-37 pass (green); existing LSP and extension suites unchanged

### Step 4.3: Implementation tests and hardening

- [ ] 4.3.1 Write impl tests — `compiler/tests/windows_cli_impl_test.rs`, `tools/vscode/test/windows.impl.test.ts`
- [ ] 4.3.2 Phase gate — `make check IMAGE=clorraine-sdk:windows-cli` and `make vscode-check`

**Deliverables**:
- [ ] ST-31 – ST-37 hold
- [ ] All verification passing

**Verify (gate)**: `make check IMAGE=clorraine-sdk:windows-cli` and `make vscode-check`

---

## Phase 5: Installer core

> **Phase baseline tree**: _(recorded by exec-plan)_
> **Reasoning**: xhigh — deletion, PATH, and junction handling change user state irreversibly

### Step 5.1: Specification tests

**Reference**: 03-04 §Flow 1–3, §PATH, §Uninstall and purge, §Presentation, §Template · ST-39 – ST-49, ST-59 – ST-63

- [ ] 5.1.1 [spec-author] Write ST-39 – ST-49 and ST-59 – ST-63 with the dot-source seam and a scratch registry key — `tools/setup/tests/install.spec.test.ps1`
- [ ] 5.1.2 Run on the Windows host against the empty template and confirm they fail (red)

### Step 5.2: Implementation

- [ ] 5.2.1 Write the template: parameters, conflicts, host check, style, lock, repair — `tools/setup/install.ps1`
- [ ] 5.2.2 Add download/verify, zip validation, extraction, version check, the junction switch, and rollback — `tools/setup/install.ps1`
- [ ] 5.2.3 Add the registry PATH edit and its removal — `tools/setup/install.ps1`
- [ ] 5.2.4 Add uninstall and purge with the reparse-safe walker and the in-use check — `tools/setup/install.ps1`
- [ ] 5.2.5 Run on the Windows host — ST-39 – ST-49, ST-59 – ST-63 pass (green)

### Step 5.3: Implementation tests and hardening

- [ ] 5.3.1 Write impl tests (zip validator, reparse walker, mutex abandonment, PATH normalization) — `tools/setup/tests/install.impl.test.ps1`
- [ ] 5.3.2 Run both PowerShell suites on the Windows host
- [ ] 5.3.3 Phase gate — `make check IMAGE=clorraine-sdk:windows-cli`

**Deliverables**:
- [ ] ST-39 – ST-49, ST-59 – ST-63 hold on Windows
- [ ] All verification passing

**Verify (gate)**: `make check IMAGE=clorraine-sdk:windows-cli`; on Windows both PowerShell suites

---

## Phase 6: Installer toolchain, emulator, hellos, editor, starter

> **Phase baseline tree**: _(recorded by exec-plan)_
> **Reasoning**: xhigh — the only privilege boundary (elevated rc24 step)

### Step 6.1: Specification tests

**Reference**: 03-04 §Flow 4–8, §Elevated rc24 step · ST-50 – ST-58

- [ ] 6.1.1 [spec-author] Write ST-50 – ST-58 — `tools/setup/tests/install.spec.test.ps1`
- [ ] 6.1.2 Run on the Windows host and confirm they fail (red)

### Step 6.2: Implementation

- [ ] 6.2.1 Add the registration check, the elevated staging script, and the rc24 step — `tools/setup/install.ps1`
- [ ] 6.2.2 Add FS-UAE and ROM detection, the FS-UAE download, and the `setup` call — `tools/setup/install.ps1`
- [ ] 6.2.3 Add the hellos, VS Code, the starter, and the summary — `tools/setup/install.ps1`
- [ ] 6.2.4 Run on the Windows host — ST-50 – ST-58 pass (green); ST-39 – ST-49, ST-59 – ST-63 still pass

### Step 6.3: Implementation tests and hardening

- [ ] 6.3.1 Write impl tests (encoded elevated command, registry parsing, ROM globbing) — `tools/setup/tests/install.impl.test.ps1`
- [ ] 6.3.2 Phase gate — `make check IMAGE=clorraine-sdk:windows-cli`; both PowerShell suites on Windows

**Deliverables**:
- [ ] ST-50 – ST-58 hold on Windows
- [ ] All verification passing

**Verify (gate)**: `make check IMAGE=clorraine-sdk:windows-cli`; on Windows both PowerShell suites

---

## Phase 7: Packaging and acceptance evidence

> **Phase baseline tree**: _(recorded by exec-plan)_
> **Reasoning**: high — release asset names become public URLs; evidence is the only Windows gate

### Step 7.1: Specification tests

**Reference**: 03-05 §Candidate, §Acceptance evidence · ST-64 – ST-72 · AR-33, AR-46

- [ ] 7.1.1 [spec-author] Write ST-64 – ST-69 — `tools/setup/tests/windows-dist.spec.test.sh`; ST-70 – ST-72 — `tools/setup/tests/acceptance_windows_spec.py`; apply the two supersessions in 07 §Supersession
- [ ] 7.1.2 Run them and confirm they fail (red)

### Step 7.2: Implementation

- [ ] 7.2.1 Add the Windows stage check, the zip builder, and the five-line `SHA256SUMS` with `--verify` — `tools/setup/package.sh`
- [ ] 7.2.2 Assemble `install.ps1` in `package.sh` and wire the Windows build into `make dist` and `dist-check` — `tools/setup/package.sh`, `Makefile`
- [ ] 7.2.3 Make `acceptance.py` platform-keyed — `tools/setup/acceptance.py`
- [ ] 7.2.4 Write the Windows evidence producer — `tools/setup/acceptance.ps1`
- [ ] 7.2.5 Run them — ST-64 – ST-72 pass (green); existing dist and acceptance suites pass

### Step 7.3: Implementation tests and hardening

- [ ] 7.3.1 Write impl tests for the zip builder and the evidence schema — `tools/setup/tests/windows-dist.impl.test.sh`, `tools/setup/tests/acceptance_windows_impl.py`
- [ ] 7.3.2 Build a candidate with `make dist` and `make dist-verify`
- [ ] 7.3.3 Phase gate — `make check IMAGE=clorraine-sdk:windows-cli`

**Deliverables**:
- [ ] A five-asset local candidate under `build/dist`
- [ ] All verification passing

**Verify (gate)**: `make check IMAGE=clorraine-sdk:windows-cli`

---

## Phase 8: Documentation

> **Phase baseline tree**: _(recorded by exec-plan)_
> **Reasoning**: medium — user-facing text; security guidance must be exact (AR-6)

### Step 8.1: Docs

**Reference**: 03-05 §Documentation · ST-74 · AR-6, AR-15, AR-18, AR-19, AR-30, AR-38, AR-39, AR-42

- [ ] 8.1.1 Write `docs/windows-cli.md` and link it from `docs/README.md`
- [ ] 8.1.2 Add the Windows limitations — `docs/known-limitations.md`
- [ ] 8.1.3 Add the Windows install beside the Linux one — `README.md`, `site/`
- [ ] 8.1.4 Run `make site-check` and the docs guide spec (ST-74)
- [ ] 8.1.5 Phase gate — `make check IMAGE=clorraine-sdk:windows-cli`

**Deliverables**:
- [ ] W17 met; ST-74 holds
- [ ] All verification passing

**Verify (gate)**: `make check IMAGE=clorraine-sdk:windows-cli` and `make site-check`

---

## Phase 9: Windows acceptance and merge readiness

> **Phase baseline tree**: _(recorded by exec-plan)_
> **Reasoning**: high — real-host acceptance with a person approving UAC

### Step 9.1: Acceptance

**Reference**: 03-05 §Acceptance evidence · ST-38, ST-73 · AR-46, AR-49, AR-50

- [ ] 9.1.1 Push the Phase 7 candidate to the Windows host with the helper
- [ ] 9.1.2 Run the installer with the spike's rc24 registration present and confirm it is reused (AR-50)
- [ ] 9.1.3 Uninstall the spike rc24 with its own uninstaller; create the standard user `Clorraine Test` and hand the user its password (AR-49)
- [ ] 9.1.4 With the user signed in as `Clorraine Test` and approving UAC: run `acceptance.ps1`; collect the record
- [ ] 9.1.5 The user confirms the UAC prompt and a Build/Run in FS-UAE from VS Code (ST-38); record both in the evidence
- [ ] 9.1.6 Validate the record — `make acceptance-check EVIDENCE=… CANDIDATE=…` (ST-73)
- [ ] 9.1.7 Final Linux gate — `make verify`

### Step 9.2: Cleanup and merge readiness

- [ ] 9.2.1 Remove `Clorraine Test` and `%USERPROFILE%\clorraine-spike` on the Windows host (AR-49, AR-50)
- [ ] 9.2.2 Draft the Clang follow-up issue text and the merge notes (image rebuild, `AGENTS.md` and roadmap edits per AR-37); file or open nothing without the user's approval
- [ ] 9.2.3 Rebase on `main`, rerun `make check IMAGE=clorraine-sdk:windows-cli`, and push the branch with the git-commit skill in push mode
- [ ] 9.2.4 Ask the user to approve opening the pull request

**Deliverables**:
- [ ] A validated Windows acceptance record and a passing `make verify`
- [ ] The branch ready for review; nothing published

**Verify (gate)**: `make verify`; `make acceptance-check EVIDENCE=… CANDIDATE=…`

---

## Dependencies

```
Phase 1
    ↓
Phase 2 ──→ Phase 3 ──→ Phase 4
                ↓
            Phase 5 ──→ Phase 6
                            ↓
                        Phase 7 ──→ Phase 8
                            ↓
                        Phase 9
```

Phase 3 needs Phase 2 (ST-30 runs a backslash path). Phases 5–6 need Phase 3's driver on
Windows. Phase 7 needs the installer. Phase 9 needs every earlier phase.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ `make check IMAGE=clorraine-sdk:windows-cli`, both PowerShell suites on Windows, and `make verify` pass
3. ✅ The Windows acceptance record passes `make acceptance-check`
4. ✅ No warnings/errors
5. ✅ No dead code
6. ✅ Security hardened — verified downloads, admin-only elevated staging, reparse-safe deletion, exact PATH edits
7. ✅ Documentation updated
8. ✅ Phase reviews completed per the index routing
9. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
