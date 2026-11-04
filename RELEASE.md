# Release Checklist: Clorraine v1.0.0

This document is the release runbook for Clorraine v1.0.0. It records the approved release
decision, the licence and metadata audit, the verification gates, and the conditional publication
steps. Publication is a separate, explicitly authorized step: the production-readiness plan can
close with this checklist complete even when no artifact has been published.

## Release decision

| Item | Decision |
| ---- | -------- |
| Version | `1.0.0` |
| Licence | MIT |
| Source | Git tag on `main` |
| Binary artifact | Linux x86_64 tarball |
| Integrity | `SHA256SUMS` |
| Release notes | `RELEASE-NOTES.md` and one `CHANGELOG.md` entry |
| aarch64 artifact | Deferred to PLAN-07; developer builds remain supported |

The version of record is set in `compiler/Cargo.toml` and mirrored in `Cargo.lock`. The release is
manual; automation is out of scope for the first tag.

**Publication status (2026-11-04):** **published** as
[v1.0.0](https://github.com/OpenLorraine/clorraine/releases/tag/v1.0.0). The tag, source tarball,
Linux x86_64 tarball, `SHA256SUMS`, release notes, and changelog are attached. The conditional
publish steps below are complete.

## Pre-release gates

- [x] `make check` — specification check plus the full workspace test suite
- [x] `make verify` — all four legs, including the FS-UAE corpus and the six runtime faults
- [x] `make asan` — every suite that references `RDC_ASAN`, one bounded process per suite
- [x] `make differential` — host and hosted-amiga output match for the three differential programs

Current evidence (2026-11-03):

| Gate | Result |
| ---- | ------ |
| `make check` | exit 0; `spec check: OK (27 files, 111 compile-fail tests)`; 92 passing test groups |
| `make verify` | exit 0; `verify: all legs passed` |
| `make asan` | 24/24 suites passed (serial, with the `-no-pie` ASan link wrapper) |
| `make differential` | 3/3 programs match; `DIFF_ONLY=1` negative control exits nonzero without regenerating output |

## Licence and metadata audit

- [x] `LICENSE` contains the MIT text with the OpenLorraine copyright line
- [x] `compiler/Cargo.toml` declares `version = "1.0.0"`, `license = "MIT"`, a description, and
      the repository URL
- [x] `Cargo.lock` records `clc` version `1.0.0`
- [x] `tools/demo/package.json` declares `"license": "MIT"`
- [x] `tools/demo/package-lock.json` records `"license": "MIT"` on the root package
- [x] `README.md` names MIT and links `LICENSE`
- [x] `THIRD_PARTY_NOTICES.md` preserves the upstream Hamurabi MIT notice
- [x] `docs/known-limitations.md` exists and is linked from `README.md` and `spec/README.md`

| File | Expected | Observed |
| ---- | -------- | -------- |
| `LICENSE` | MIT text, OpenLorraine copyright | present |
| `compiler/Cargo.toml` | `1.0.0`, `MIT`, description, repository | present |
| `Cargo.lock` | `clc 1.0.0` | present |
| `tools/demo/package.json` | `MIT` | present |
| `tools/demo/package-lock.json` | root package `MIT` | present |
| `README.md` | MIT section, `LICENSE`, and notices link | present |
| `THIRD_PARTY_NOTICES.md` | upstream Hamurabi MIT text | present |
| `spec/README.md` | known-limitations link | present |

## Conditional publication

These steps run only after the user authorizes publication or after the final production gate
passes. They are not part of the 45-task main plan.

- [x] Confirm the version of record is `1.0.0` in `compiler/Cargo.toml` and `Cargo.lock`
- [x] Build the release binaries for Linux x86_64 and smoke-test them
- [x] Assert the exact output of `clc --version` is `clc 1.0.0`
- [x] Compile and run a hello-world program with the release binary
- [x] Create the `v1.0.0` source tag
- [x] Package the source and the Linux x86_64 binary tarball
- [x] Write `SHA256SUMS` for every published artifact
- [x] Add `RELEASE-NOTES.md` and one `CHANGELOG.md` entry
- [x] Confirm no NDK content, ROM, or other restrictively licensed third-party file is redistributed
- [x] Attach the notes, changelog, and checksums to the release page

If publication cannot run, record the reason in the plan and release notes. The main plan still
closes production-ready; only publication remains blocked.

## Failure handling

| Failure | Action |
| ------- | ------ |
| A verification gate fails | Stop; fix the implementation or the environment, then rerun the full gate |
| The version does not match | Fix `compiler/Cargo.toml`, regenerate `Cargo.lock`, and rebuild |
| A licence or metadata field is missing | Add it and rerun `./spec/check.sh` and `make check` |
| Publication is not authorized | Do not tag or upload; record the blocker and keep the checklist open |
| An artifact would contain NDK or ROM content | Do not publish; rebuild the artifact from allowlisted sources only |
