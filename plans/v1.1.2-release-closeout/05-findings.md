# Findings and Evidence: v1.1.2 Release Closeout (PLAN-27)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Baseline pending — filled during execution

## Purpose

This document is the release evidence ledger: repository state, the
release-tree comparison, the same-compiler flag audit, gate results, artifact
hashes, tag identity, publication record, review rulings, and accepted limits.
`RELEASE.md` is the summary and publication record.

## Baseline

| Item | Value |
|---|---|
| Previous release | annotated tag `v1.1.1` (tag object `144e53b`) → commit `2d1eb3026fc6f75885785e6ff9601f99aaba4021` |
| Pre-plan `main` | `3227321572a7d214e7415e2a5f5724a1d8872853` (equal to `origin/main` at fetch) |
| Plan baseline (C0) | `9fb48ac438b96fcc52b348e3017e56e8aa5a262f` |
| Version | `1.1.1` in Cargo, lock, and the VS Code package |
| `clc` in the tree | `clc 1.1.1` |

The pre-Phase-2 evidence was preserved as `build/perf/{base,head,flags}-plan26.json`
and `build/perf/plan27-baseline.txt`; it describes the PLAN-26 runs, not the
release. Phase 2 deletes and regenerates the live JSONs.

**v1.1.1 benchmark check:** a temporary worktree at `v1.1.1` built `clc`, and
that compiler emitted the current `benchmarks/lookup/program.clo` for
`hosted-amiga`. Compiled with the shipped flags, `.text` is 64,328 and the
program prints `lookup: 134217728`. The base can measure the current set.

**Current-version statements:** `README.md`, `RELEASE.md`, `CHANGELOG.md`,
`RELEASE-NOTES.md`, and `docs/known-limitations.md`; no test under
`compiler/tests`, `tools/vscode/test`, or `spec/` pins `1.1.1`.

**Environment:** the repository is private on GitHub (`gh` authenticated as
`blendsdk`, ADMIN); Node 22.23.3 exists under
`~/.nvm/versions/node/v22.23.3/bin` but is not on the default `PATH`, so
`make vscode-check` needs that directory prepended.

## Release Scope

The 31 pre-plan commits since v1.1.1 classify as:

| Class | Commits | Notes |
|---|---|---|
| Product | `cef0b91`, `fd7fe30`, `b019771` | constant-array emission, collision-free template symbol, release GC+strip |
| Behavior-neutral compiler | `902ef30`, `cda97cc` | dead-code removal; per-profile release seams still on `-O2` |
| Tests and tooling | `a886a34`, `a589353`, `ac92308`, `ca30d36`, `4e6e1d1`, `866104c`, `e3fdbf3`, `29b85fb` | the `lookup` benchmark, vbcc probe retries, test pins, the flag audit |
| Records | the remaining 17 | PLAN-25/PLAN-26 plans, reviews, roadmap, publication notes |

No other user-visible feature is in the range. The closeout adds only the
release records.

## Measurements

_Pending Phase 2._ Three bases, kept separate:

1. isolated constant-array evidence (old flags);
2. release-tree comparison (`v1.1.1` → head, shipped flags);
3. same-compiler flag audit (`make bench-flags`).

## Gate Evidence

_Pending Phase 2._ Every gate, its result, and the gated commit.

## Tag and Artifacts

_Pending Phase 3._ Gated commit, evidence commit, tag object, artifact names,
sizes, SHA-256 values, and the archive audit.

## Publication

_Pending Phase 3/4._ Draft asset verification, published URL, and final status.

## Review Rulings

_Pending Phase 4._ The correctness review's findings and resolutions.

## Accepted Limits

_Pending Phase 4._ Stripped release programs, freestanding/non-Linux symbol
retention, the LSP exclusion, and the source-snapshot boundary.
