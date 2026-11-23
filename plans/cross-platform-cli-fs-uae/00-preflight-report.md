# Preflight Report: Cross-platform CLI and FS-UAE delivery

> **Status**: ❌ PREFLIGHT BLOCKED — scope decision required; iteration-2/3 major findings remain
> **Iteration**: 3 (scoped re-scan after Linux-only revision)
> **Artifact**: implementation plan at `plans/cross-platform-cli-fs-uae/`
> **Codebase Grounded**: 30+ source files examined; file/line references checked against HEAD `8f454eae`
> **Last Updated**: 2026-11-23 16:40
> **Same-session bias**: ⚠️ the plan was created and preflighted in one session; an independent execution review is still required.

## Codebase Context Summary

**Tech stack:** std-only Rust workspace (`compiler/`), TypeScript VS Code client
(`tools/vscode/`), Makefile/Docker SDK for development and gates, Python release
producers/tests.

**Architecture:** `clc` emits C99; `clorraine` builds/runs profiles and stages
FS-UAE; `clorraine-lsp` serves stdio. The extension calls the driver. The old
Python installer/support pipeline is separate and now superseded.

**Key files examined:** `compiler/src/clorraine/{cli,environment,emulator,
artifacts,native_metadata,mod,run,toolchain,main,clorraine_main}.rs`,
`tools/vscode/src/{discovery,actions}.ts`, `tools/vscode/package.json`,
`Makefile`, `.github/workflows/*.yml`, `tools/setup/install.sh`,
`tools/setup/{install_native,native_archive,native_state}.py`,
`compiler/tests/native_driver_*`.

## Summary by Severity

| Severity | Count | Status |
|---|---:|---|
| CRITICAL | 5 | Addressed in revised plan |
| MAJOR | 10 | Addressed in revised plan |
| MINOR | 7 | Addressed or accepted as notes |
| OBSERVATION | 1 | Audit limitation only |

## Findings

| ID | Severity | Finding | Resolution in revised plan |
|---|---|---|---|
| PF-001 | 🔴 CRITICAL | Toolchain root did not match the driver: the old resolver requires `sdk/amiga`, receipts and a bundled venv, and hard-rejects non-Linux. `--toolchain` could not work. | Manual upstream toolchain + explicit resolver contract in 03-01 §Driver toolchain resolution; Phase 2 task 2.2.3; old layout retained for compatibility. |
| PF-002 | 🔴 CRITICAL | Spec-author/red tasks were verified with `make check`, which fails while tests are intentionally red. | Phase 2/3/4 red tasks now name targeted test commands and stay `[~]` until green; `make check` only for green tasks. |
| PF-003 | 🔴 CRITICAL | Removing `tools/distribution/` left `make check` calling `distribution-check`. | Phase 4 task 4.2.4 removes the target/call site and adds `release-check`; removal test covers Makefile/CI references. |
| PF-004 | 🔴 CRITICAL | Acceptance had no candidate staging; only a draft workflow existed. | 03-04 §Candidate staging + Phase 5 task 5.2.1 download the draft and test its exact bytes. |
| PF-005 | 🔴 CRITICAL | Phase 1 spikes asked our CLI to build on macOS/Windows before the port. | Phase 1 now builds with the upstream compiler directly; our-CLI checks move to Phase 6. |
| PF-006 | 🟠 MAJOR | `curl \| sh` and unsafe download/extraction; no SHA-256 backend. | AR-26 removes toolchain downloads; install scripts are download/verify/run with platform hash tools (AR-28/31). No remote text to shell. |
| PF-007 | 🟠 MAJOR | Release/editor test files had no Makefile/CI runner; `make check` couldn't see them. | `release-check` target added; editor tasks use `make vscode-check`; Phase 6 adds hosted test matrix. |
| PF-008 | 🟠 MAJOR | FS-UAE success only meant emulator launch; acceptance could pass without the program running. | AR-29: AROS launch-only; guest-success acceptance requires a user ROM and observable output. 07 evidence schema. |
| PF-009 | 🟠 MAJOR | Release composition contradicted the platform gate. | AR-27: v1.2.0 Linux-only; macOS/Windows archives are follow-up releases; ST-38. |
| PF-010 | 🟠 MAJOR | Extension settings/config migration undefined; `CLORRAINE_DRIVER` did not exist. | 03-03 defines config discovery and migration/removal of duplicate settings; `CLORRAINE_DRIVER` removed from the contract. |
| PF-011 | 🟠 MAJOR | `make verify` (Docker) did not test the upstream toolchain. | 01/02 now require a separate upstream-toolchain corpus leg; `make verify` remains the Docker gate. |
| PF-012 | 🟠 MAJOR | WSL/minimum OS claims had no detection; macOS runner label retired. | AR-32: WSL unsupported/untested, no heuristic; macOS 14+/Windows 10 1809+ documented; Phase 6 uses macos-15. |
| PF-013 | 🟠 MAJOR | No acceptance evidence schema/command; hashes not tied to tested bytes. | 07 defines the JSON evidence record; Phase 5 validates candidate/evidence hashes and guest result. |
| PF-014 | 🟠 MAJOR | Side-by-side activation, absolute bootstrap invocation and rollback were contradictory. | AR-30 and 03-01 define version dirs/marker, absolute CLI invocation, switch-after-smoke and rollback. |
| PF-015 | 🟡 MINOR | Windows `setup.exe` target/elevation was undefined. | Manual upstream install + `--toolchain`; Clorraine never runs it (AR-26). |
| PF-016 | 🟡 MINOR | Unsafe archive links and extraction TOCTOU for the upstream toolchain. | No toolchain extraction by Clorraine; installers extract only our own release archive with fixed names/hashes. |
| PF-017 | 🟡 MINOR | Duplicate ST ownership and missing coverage percentages. | ST-1–12 setup, ST-13–15 bootstrap; coverage percentages removed. |
| PF-018 | 🟡 MINOR | `--force` and `CLORRAINE_DRIVER` were unapproved surface. | Removed from the plan. |
| PF-019 | 🟡 MINOR | Docs pages outside the named list still described the old tarball. | Phase 4 task 4.2.3 includes README, quick-start, RELEASE/notes, site status/brief/roadmap/known-limitations and root known-limitations. |
| PF-020 | 🟡 MINOR | Notices referenced removed `tools/distribution/inputs`. | Phase 4 task 4.2.4 updates notices or deliberately retains referenced inputs. |
| PF-021 | 🟡 MINOR | `make verify` prerequisites (X display, licensed media) were unrecorded. | Phase 3/5 gates name the private-display/media prerequisites; CI is separate. |
| PF-022 | 🟡 MINOR | Path guards (commas/trailing-space ROM/control chars) were omitted. | 03-03 §Staging retains and tests them. |
| PF-023 | 🔵 OBSERVATION | Preflight was read-only; no real platform/toolchain run. | Execution Phase 1/5 must provide the real evidence. |

## Decision Record

The user confirmed the simplification: manual upstream toolchain, Linux-only
v1.2.0, download-verify-run installers, launch-only AROS, side-by-side updates
(AR-26–32). All critical/major findings are addressed by those decisions and the
task/document revisions above.

## Remaining notes

- The plan's real feasibility is still unproven until Phase 1 runs the upstream
  toolchain and FS-UAE on Linux and the hosted macOS/Windows spikes.
- The old `v1.2.0-release-installer` plan is closed and unpublished (AR-50).
- No source, workflow or release asset has been changed by this preflight.

## Iteration-2/3 findings (blocking)

The Linux-only revision removed Phase 6 but left these material issues. IDs
continue the preflight sequence; all are open until the scope decision.

| ID | Severity | Area | Issue |
|---|---|---|---|
| PF-024 | 🟠 MAJOR | Installer | Activation order/rollback/lock ownership contradict each other; install.sh switches bin before setup. |
| PF-025 | 🟠 MAJOR | Installer | `tar -tf`/`tar -tvf` cannot implement the claimed link/type policy reliably; hostile-member fixture missing. |
| PF-026 | 🟠 MAJOR | Toolchain | The upstream-toolchain corpus leg has no repeatable target/task; hello alone is insufficient. |
| PF-027 | 🟠 MAJOR | FS-UAE | `hosted-amiga-13 --emulator` is promised but the recipe hard-codes A1200; reject or add an A500 model. |
| PF-028 | 🟠 MAJOR | Installer | `install.sh` accepts macOS arm64 although v1.2.0 has no macOS archive. |
| PF-029 | 🟠 MAJOR | Release | `install.ps1` is documented for v1.2.0 but excluded from its assets. |
| PF-030 | 🟠 MAJOR | Acceptance | Draft-asset staging cannot prove local bytes were tested; evidence lacks all asset hashes. |
| PF-031 | 🟠 MAJOR | Acceptance | `guest_success` has no observable oracle; emulator launch alone can pass. |
| PF-032 | 🟠 MAJOR | Gates | `release-check` mixes hermetic workflow tests with network/draft acceptance tests. |
| PF-033 | 🟠 MAJOR | Publication | Distribution is published before the anonymous-install gate. |
| PF-034–PF-036 | 🟠 MAJOR | Follow-up | Phase 6 had no packaging task, wrong red-scan order, wrong corpus mapping; removed by Linux-only split. |
| PF-037 | 🟠 MAJOR | Phase 2 | install.ps1/PowerShell tests cannot run on the Linux-only phase. |
| PF-038 | 🟠 MAJOR | Editor | Legacy settings migration behavior is not fully specified/tested. |
| PF-048 | 🟠 MAJOR | Report | The preflight report did not record iteration-2 findings until this section. |
| PF-049 | 🟠 MAJOR | Installer | Fresh-install order: setup requires a toolchain that is documented as a later user step. |
| PF-050 | 🟠 MAJOR | Installer | Absolute invocation path contradicts staging vs `<home>/bin`. |
| PF-051 | 🟠 MAJOR | Installer | Config is written before the bin switch; same-version replacement is not atomic. |
| PF-052 | 🟠 MAJOR | Installer | Archive rule rejects directory entries; tar text parsing is fragile. |
| PF-053 | 🟠 MAJOR | Security | Bootstrap lacks a compiled-in archive hash/size and hardened curl flags; checksum selection unclear. |
| PF-054 | 🟠 MAJOR | Toolchain | Corpus leg is not the existing corpus and has no target. |
| PF-055 | 🟠 MAJOR | FS-UAE | `result.txt` oracle is prose only; no task writes/reads/hashes it before cleanup. |
| PF-056 | 🟠 MAJOR | FS-UAE | A1200 recipe accepts any ROM; a KS1.3 ROM fails opaquely. |
| PF-057 | 🟠 MAJOR | FS-UAE | `hosted-amiga-13 --emulator` rejection has no task/ST and contradicts an existing CLI test. |
| PF-058 | 🟠 MAJOR | Acceptance | `acceptance-check` has no target/task/green run and runs after publication. |
| PF-059 | 🟠 MAJOR | CI | `release-check` is claimed in host CI but no step adds it. |
| PF-060 | 🟠 MAJOR | Publication | Anonymous-install gate has no ST/command/evidence field. |
| PF-061 | 🟠 MAJOR | CLI | `--toolchain`/`CLORRAINE_TOOLCHAIN` resolver surface contradicts setup-only contract. |
| PF-062 | 🟠 MAJOR | Compatibility | glibc 2.35 check has no implementation task; an old host cannot run the binary to report it. |
| PF-063 | 🟠 MAJOR | PATH | `--modify-path` is Must Have but install.sh/task/ST do not define the profile edit/reversal. |
| PF-064 | 🟠 MAJOR | Scope | Overview/gaps still describe macOS/Windows and a downloaded toolchain, contradicting R3/R12/AR-26. |
| PF-065 | 🟠 MAJOR | Migration | `00-index` says remove `tools/distribution/` wholesale while notices/inputs are retained. |

| ID | Severity | Issue |
|---|---|---|
| PF-066–PF-075 | 🟡/🔵 | Report counts/resolutions, marker/lock details, gzip vs xz, toolchain tag in quick-start, acceptance platform restriction, extension README/warning state, security phase, FS-UAE minimum version, approved-surface wording, removal-before-acceptance order. |

**Verdict:** preflight is blocked. Do not execute this plan until the scope is
reduced and the remaining majors are closed. Recommended next step: run the
Phase 1 Linux toolchain spike first, then rewrite a smaller Linux v1.2.0 plan
from actual evidence (CLI tarball install, user-supplied toolchain, existing
FS-UAE driver, no release publication in the same plan).
