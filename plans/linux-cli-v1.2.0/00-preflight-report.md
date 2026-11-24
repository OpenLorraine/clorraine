# Preflight report

> **Status**: ✅ PREFLIGHT PASSED WITH NOTES — revision 3; all critical/major findings resolved
> **Artifact**: `plans/linux-cli-v1.2.0/`
> **Evidence**: `spikes/07-linux-cli-fs-uae/README.md`
> **Code basis**: `0ebb3fb8`
> **Last Updated**: 2026-11-24

## Scope

The review checks the rewritten Linux-only plan against the spike evidence and
the current code. It looks for contradictions, missing behavior, untestable
claims, stale references, and unsafe installer/removal steps.

## Iteration 1 findings and resolutions

The first two independent reviews produced PF-001–PF-022 and PF-101–PF-120.
This table groups them by the revision that resolves them; exact file references
are in the current plan.

| Findings | Severity | Issue | Resolution |
|---|---|---|---|
| PF-001 | CRITICAL | A two-line echo marker let a return-1 program pass; `FailAt` is unavailable on the bare drive. | The plan now compiles `compiler/runtime/emulator_runner.c`, which uses `SystemTagList`, records rc, and writes status only for rc 0. Spike tested rc 0/1/5/9. |
| PF-002, PF-101 | CRITICAL/MAJOR | Old native-driver spec/impl suites pin removed flags, capture rejection, and process-status return. | AR-16 records the spec supersession; tasks 1.1.1/1.2.3 rewrite or retire those cases and keep the legacy handoff green. |
| PF-003, PF-011 | MAJOR | ST-11 mixed a real rc24 build into `make check`; editor/removal lacked spec-first ordering. | ST-11 is split into hermetic fake (make check) and local real build (toolchain-check); tasks 3.3.1/4.2.1 explicitly extend specs first. |
| PF-004 | MAJOR | Selection/media errors and runtime errors had conflicting exit codes. | R9/03-02/ST-12/ST-15 now agree: selection/media 2, post-spawn guest/runtime 1. |
| PF-005 | MAJOR | FS-UAE inherited stdout/stderr, so emulator noise could enter CLI stdout. | 03-02 redirects FS-UAE stdout/stderr to `logs/` and tests a noisy fake. |
| PF-006 | MAJOR | The status file could be observed empty/partial. | 03-02 re-reads a partial status for one second and requires exact `ok`; the wrapper writes rc first. |
| PF-007 | MAJOR | Read/capture failure could remove the private run before output was usable. | 03-02 retains the directory on read/capture/wait failure and writes capture before cleanup. |
| PF-008 | MAJOR | A plain PATH fallback bypassed `setup` and contradicted the selection contract. | The legacy PATH branch is marker-gated (`/opt/amiga/BUILD-PROVENANCE.txt`); AR-3/R5/index/ST-10 align. |
| PF-009 | MAJOR | The validator did not check the rc24 version. | 03-01 requires `16.2-rc24` in `--version`; ST-8 adds a wrong-version case. |
| PF-010 | MAJOR | Decision/index names used unprefixed `result.txt`/`status.txt`. | AR-7 and the index use the prefixed rc/status wrapper names; the spike keeps short names as history. |
| PF-012 | MINOR | Current-state line references were inaccurate. | 02-current-state now cites the marker, venv condition, parser, stage/storage, and discovery lines correctly. |
| PF-013 | MINOR | The rc24 subtree was not recorded; layout paths could reject a compatible root. | The spike records `m68k-amigaos/{libnix,ndk-include,...}`; the probe is the functional authority. |
| PF-014, PF-108 | MINOR/MAJOR | Purge/uninstall used a hardcoded config path. | 03-03 resolves `$XDG_CONFIG_HOME` exactly like the CLI and tests purge with/without XDG. |
| PF-015, PF-016, PF-017 | MINOR | AROS/KS3.0/Ubuntu 24.04 wording overclaimed the evidence. | AROS is manual-only/observed; KS3.1 is the tested ROM (KS3.0 untested); 24.04 is a toolchain probe, 26.04 the full-path host. |
| PF-018 | MINOR | Invalid timeout values and blocking wait were undefined. | 03-02 validates 1–3600 and bounds kill/wait; ST-16 covers both. |
| PF-019 | MINOR | Only the 524,299-byte ROM was tested. | 03-02 accepts 524,288 by size, marks 524,299 as the tested acceptance format, and tests both sizes. |
| PF-020 | MINOR | FS-UAE version detection was unproven. | The spike records `fs-uae --version` → `3.2.35`; 03-02 anchors the parse and ST-12 uses fixtures. |
| PF-021 | MINOR | Both-profile corpus evidence was missing. | The spike now includes a `hosted-amiga-13` `file-copier` run; ST-20/99 require it. |
| PF-022 | MINOR | The glibc rationale blamed the CLI, which is built on 2.36. | 03-03 attributes the 2.38 floor to the rc24 toolchain. |
| PF-102, PF-116 | MAJOR/MINOR | Editor review spec, fixtures, and new/legacy precedence were missing. | 03-04/99 list `actions.review.spec.test.ts`, both fixtures, persistent `globalState`, and new-key precedence. |
| PF-103, PF-118 | MAJOR/MINOR | Site/release notes/changelog retained old pipeline claims; retained README pointed at removed support-lock. | 03-04/99 add `site/`, `RELEASE-NOTES.md`, `CHANGELOG.md`, `RELEASE.md`, `make site-check`, and retained-README removal tests. |
| PF-104, PF-114, PF-120 | MAJOR/MINOR | VSIX delivery, release docs/links, SHA256SUMS, and version validation were undefined. | 03-03 defines a separate VSIX asset, release README/notice, SHA256SUMS coverage, VERSION validation, and dist-verify. |
| PF-105 | MAJOR | AR-10/R3/ST-5 disagreed on local-mode size verification. | AR-10/R3/03-03/ST-5 now say local mode verifies the supplied hash and records size; network mode pins size+hash. |
| PF-106, PF-107 | MAJOR | PID-only lock and move-before-switch were not crash-safe. | 03-03 uses PID+starttime atomic lock with stale retry, and unique `versions/<version>-<sha12>` dirs with an atomic `current` switch. |
| PF-109 | MAJOR | The PATH block was unquoted and prefix-ambiguous. | 03-03 shell-quotes the prefix and replaces one marked block; ST-6 covers spaces/metacharacters. |
| PF-110 | MAJOR | Producer layout/pin behavior had no testable oracle. | `tools/setup/package.sh` + `dist.spec.test.sh` test version/extra-file/stale-pin/checksum cases; `dist-verify` covers the real candidate. |
| PF-111, PF-112, PF-113 | MAJOR/MINOR | Acceptance was too weak; CANDIDATE arg and local hash command were undefined. | 03-03/07 require exact rc24 pin, FS-UAE min, hello/corpus hashes, VSIX hash, `CANDIDATE=`, and direct `sha256sum`. |
| PF-115 | MINOR | Host-check tests lacked fake `uname`/`getconf` seams. | 03-03/ST-1–ST-3 specify fake `uname`, `getconf`, and PATH fixtures. |
| PF-117 | MINOR | ST-6 was mapped to R3; 02 gap table and ST-28 task were wrong. | R14 owns ST-6; 01/02/07/99 mappings are corrected; ST-28 is in the docs/removal gates. |
| PF-119 | MINOR | Hardlink archive members were not rejected. | 03-03 rejects hardlinks in the type scan; ST-18 adds a hostile hardlink member. |

## Re-scan findings and resolutions

The focused re-scan found four majors and eleven minor/observation items. All
are resolved in revision 3.

| ID | Severity | Issue | Resolution |
|---|---|---|---|
| RF-001 | MAJOR | `--modify-path` quoting/idempotence for spaces/quotes/globs. | The block now uses a single-quoted `clorraine_bin` variable, a double-quoted case pattern, rejects `:` in the prefix, and tests spaces, a single quote, and a glob. |
| RF-002 | MAJOR | Versioned archive root vs extraction; `current/bin` assumed at the wrong level. | The installer requires exactly one top-level directory and extracts with `--strip-components=1`; the installer test uses the produced layout. |
| RF-003 | MAJOR | Acceptance recorded a toolchain hash without hashing the archive. | `make acceptance` now takes `TOOLCHAIN_ARCHIVE`, hashes it against the rc24 pin, extracts it to a private root, and uses that root. |
| RF-004 | MAJOR | R9/R10 were unconditional but the legacy PATH branch keeps the host handoff. | R9/R10 are qualified with “when a native toolchain is selected”; the marker-gated legacy handoff output/exit is stated separately. |
| RF-005 | MINOR | Wrapper tests/compile command were split/underspecified. | Wrapper oracle cases live in Phase 2 `fs_uae_spec_test.rs`; the exact compile command (source, `nix20`, `-O2 -Wall -Wextra -Werror`, `-lamiga -lm`, output) is pinned. |
| RF-006 | MINOR | Index said R1–R13 but R14 exists. | Index now says R1–R14. |
| RF-007 | MINOR | Summary/prerequisites still said KS3.0/3.1. | All now say KS3.1; KS3.0 is explicitly untested. |
| RF-008 | MINOR | Acceptance JSON example showed only two corpus entries. | The example now shows all four hosted-amiga entries plus the hosted-amiga-13 entry. |
| RF-009 | MINOR | rc/status newline/trim and partial-rc handling were undefined. | The host trims one newline/optional CR, requires exact `0`/`ok`, and re-reads empty/partial rc or status for one second. |
| RF-010 | MINOR | `setup.rs` was named as a unit-test layer but not in the file plan. | `compiler/src/clorraine/setup.rs` is now in the related files and task 1.2.1. |
| RF-011 | MINOR | Purge could remove the lock; stale `current.new` handling was unstated. | The lock is a sibling (`PREFIX/share/clorraine.lock`); purge removes it last; step 8 removes stale `current.new`; crash/purge tests added. |
| RF-012 | MINOR | Checker did not require version or the negative record. | The checker requires `version=1.2.0` and the rc-1 negative record. |
| RF-013 | MINOR | Ubuntu 24.04 wording could imply a full-path test. | Decisions/index now say 24.04 is the minimum/toolchain probe; the full spike host was 26.04; acceptance records actual. |
| RF-014 | OBS | Spike consequence text still used short result/status names. | The spike now names `clorraine-rc.txt`/`clorraine-status.txt` and the wrapper. |
| RF-015 | OBS | Network mode URL targets an unpublished release. | Network mode is documented as post-publication; candidate acceptance uses local `--archive`. |

## Verdict

**Preflight passed with notes.** No critical or major finding remains after
revision 3. The notes are operational: execution needs the pinned SDK image,
Node/npm, the rc24 toolchain archive, a licensed KS3.1 A1200 ROM, and a display
or `xvfb-run`; `make verify` remains the acceptance gate. An execution-time
review is still required because the plan and its reviews share a session.
