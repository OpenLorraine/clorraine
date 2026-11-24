# Linux CLI v1.2.0

> **Status**: 🔄 Executing — Phase 2 complete (14/28); post-phase review running
> **Version**: 1.2.0
> **Scope**: Linux x86_64 only
> **Evidence**: `spikes/07-linux-cli-fs-uae/README.md`
> **Supersedes**: the blocked `cross-platform-cli-fs-uae` plan

## Summary

Ship a Linux CLI-first 1.2.0 path that a user can install without Docker:

1. Install the versioned CLI archive.
2. Download the pinned AmigaPorts rc24 toolchain themselves.
3. Run `clorraine setup --toolchain PATH`.
4. Build a `.clo` program.
5. Run it in FS-UAE with their licensed Kickstart 3.1 A1200 ROM and see
   the guest output.

The editor uses the same installed CLI. The old native Python installer and
support/release pipeline are removed after a local candidate passes acceptance.
No release is published by this plan.

## Why this plan is smaller

The previous 43-task plan mixed a custom SDK assembly, an installer that
activated only after a successful toolchain build, a draft-release/anonymous
install gate, and a cross-platform port. Preflight found contradictions in that
combination. The spike removed the unknowns:

| Unknown | Evidence |
|---|---|
| Does rc24 build our emitted C? | Yes, both `nix20` and `nix13`; corpus passes. |
| Does FS-UAE run our binaries with a user ROM? | Yes, A1200/KS3.1, ~6 s to result. |
| Can we observe guest success? | Yes, the wrapper writes rc plus a status only for rc 0. |
| What host does rc24 need? | glibc 2.38; Ubuntu 24.04+ minimum (toolchain probe); 22.04 fails; full spike host 26.04. |
| Does the old resolver fit? | No; it expects the superseded `sdk/amiga` assembly. |

The new plan replaces that assembly with a small upstream-toolchain contract and
moves publication out of scope.

## Execution prerequisites

| Prerequisite | Why |
|---|---|
| Docker with the pinned `clorraine-sdk:dev` image (`make -C docker image` if missing) | `make check` runs the workspace tests in it; `make dist` builds release binaries in it. This is a maintainer dependency, not a user install dependency. |
| Upstream rc24 Linux toolchain, extracted | `setup --toolchain` and all Amiga/freestanding builds. |
| Node.js/npm for `make vscode-package` | The separate VSIX candidate asset and editor tests. |
| Licensed 512 KB Kickstart 3.1 A1200 ROM | Real FS-UAE acceptance and `make fs-uae-check`; KS3.0 is untested. |
| FS-UAE >= 3.2.0 and a display or `xvfb-run` | The guest runner. Ubuntu 24.04 ships 3.1.66, below the requirement; install the official 3.2.35 build. |

## Phases

| Phase | Title | Tasks |
|---|---|---|
| 1 | Toolchain selection and build | 7 |
| 2 | Verified FS-UAE guest run | 7 |
| 3 | Installer, candidate archive, editor | 9 |
| 4 | Acceptance and legacy removal | 5 |

**Total: 28 tasks.** See [99-execution-plan.md](99-execution-plan.md).

## Key decisions

| Decision | Value |
|---|---|
| Host minimum | Linux x86_64, glibc 2.38+ (Ubuntu 24.04+ minimum; 24.04 toolchain-probe tested; acceptance records the actual host) |
| Toolchain | User-downloaded AmigaPorts `v16.2-rc24`; pinned size/hash; never bundled |
| Selection | `--toolchain` > `CLORRAINE_TOOLCHAIN` > stored config > marker-gated legacy `PATH` for the Docker SDK; verified FS-UAE needs the native selection |
| Emulator | FS-UAE >= 3.2.0, A1200, licensed 512 KB KS3.1 ROM (KS3.0 untested) |
| Guest result | Compiled wrapper records the guest rc; status `ok` is written only for rc 0 |
| AROS | Manual only; observed but not supported/acceptance; no silent fallback |
| Install | Versioned archive, atomic `current`, side-by-side versions, optional PATH block |
| Publication | Out of scope; local candidate and evidence only |

Full details: [00-decisions.md](00-decisions.md).

## Documents

| Document | Contents |
|---|---|
| [01-requirements.md](01-requirements.md) | R1–R14, quality requirements, non-goals |
| [02-current-state.md](02-current-state.md) | Code facts, gaps, reusable paths |
| [03-01-toolchain.md](03-01-toolchain.md) | Setup, validator, config, resolver, build |
| [03-02-fs-uae.md](03-02-fs-uae.md) | FS-UAE/ROM validation, staging, result loop, capture |
| [03-03-install-release.md](03-03-install-release.md) | Archive, installer, activation, acceptance, publication boundary |
| [03-04-editor-migration.md](03-04-editor-migration.md) | VS Code settings/actions, old pipeline removal |
| [07-testing-strategy.md](07-testing-strategy.md) | ST-1–ST-28, commands, evidence |
| [99-execution-plan.md](99-execution-plan.md) | 28 tasks and phase gates |
| [05-findings.md](05-findings.md) | Phase 1 review rulings |
| [00-preflight-report.md](00-preflight-report.md) | Independent review findings |

## Review routing

| Role | Kind | Use | Why |
|---|---|---|---|
| `amiga-platform-reviewer` | reviewer | Phase 2/3 review of FS-UAE, ROM, and A1200 behavior | The repository routes emulator/platform changes here. |
| Generic correctness reviewer | reviewer | Every phase; CLI/config/process correctness | The driver and installer are the main change surface. |
| Generic security reviewer | reviewer | Phase 3/4; download, archive, activation, PATH edits | Installer and removal safety. |

No emission-integrity or memory-safety auditor is scheduled: this plan does not
change emitted C, IR, symbol names, ownership, drop, or lowering. The existing
compiler flags and profiles are retained; the spike built the corpus with them.
If execution changes codegen, add those reviews.

## Same-session bias

⚠️ This plan and its spike were produced in one session. The preflight is an
independent read-only review; an execution-time review is still required.

## Related files

| Path | Relevance |
|---|---|
| `compiler/src/clorraine/cli.rs` | `setup`, `--toolchain`, removed `--sdk-root`, FS-UAE flags |
| `compiler/src/clorraine/setup.rs` (new) | `setup` command, config read/write, toolchain validator |
| `compiler/src/clorraine/mod.rs` | Command dispatch, capture, build/run wiring |
| `compiler/src/clorraine/environment.rs` | Replace old SDK assembly resolution |
| `compiler/src/clorraine/emulator.rs` | Result loop and staging |
| `compiler/src/clorraine/toolchain.rs` | Existing flags; keep |
| `compiler/src/clorraine/artifacts.rs` | `out.txt` path |
| `compiler/tests/toolchain_setup_spec_test.rs` (new) | Toolchain/setup contract |
| `compiler/runtime/emulator_runner.c` (new) | Guest wrapper source for rc/status |
| `compiler/tests/fs_uae_spec_test.rs` (new) | Runner contract |
| `compiler/tests/native_driver_support/`, `native_gate_spec_test.rs` | Old `sdk/amiga` fixtures; rewrite/remove obsolete cases |
| `compiler/tests/clorraine_driver_spec_test.rs` | Legacy handoff compatibility; keep green |
| `tools/setup/install.sh` | New Linux bootstrap |
| `tools/setup/package.sh` (new) | Candidate archive/checksum/installer assembly |
| `tools/setup/tests/dist.spec.test.sh` (new) | Producer tests |
| `tools/setup/tests/acceptance_spec.py` (new) | Evidence validator tests |
| `tools/setup/install_sdk.py` | Keep for Docker SDK setup |
| `tools/setup/install_native.py`, `native_archive.py`, `native_state.py` | Remove after acceptance |
| `tools/distribution/` | Remove code/tests/producers; keep `inputs/` |
| `tools/vscode/src/discovery.ts`, `actions.ts`, `package.json`, `README.md` | Editor update |
| `Makefile`, `.github/workflows/check.yml` | New targets and Ubuntu 24.04 |
| `site/`, `RELEASE-NOTES.md`, `CHANGELOG.md`, `RELEASE.md` | Public claims and removed commands |
| `THIRD_PARTY_NOTICES.md` | Retained inputs reference |
