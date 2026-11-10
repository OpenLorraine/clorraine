# v1.1.2 Release Closeout (PLAN-27)

> **Feature**: Publish a patch release containing the landed constant-array
> and build-flag wins, with the full gate, checksummed artifacts, and the
> established runbook.
> **Status**: Plan preflighted — ready for execution
> **Created**: 2026-11-10
> **Last Updated**: 2026-11-10
> **Implements**: R1–R7
> **CodeOps Artifact Schema**: 1

**Original goal:** publish v1.1.2 so the two measured post-v1.1.1 wins reach
users, without adding language, ABI, spec, stdlib, or CLI surface.

## Overview

`main` is 31 commits past the published `v1.1.1`. The user-visible changes are
the constant-array emission (PLAN-25) and the release flag/DCE/strip work
(PLAN-26). The rest is planning, tests, tooling, and one behavior-neutral
compiler cleanup (`902ef30`).

The release is a patch:

- version `1.1.2` in Cargo, the lockfile, and the VS Code extension;
- one changelog entry and fresh release notes;
- the v1.1.2 runbook with the same gate set as v1.1.1 plus the flag audit;
- a scored comparison against `v1.1.1` and a same-compiler flag audit;
- source and Linux x86_64 tarballs with `SHA256SUMS`;
- an annotated tag and a published GitHub release.

## Minimum-Sufficient Baseline

**Smallest viable design:** bump the four version surfaces, write one changelog
entry and the release notes, refresh the runbook, run the gates already
defined by the repository, package with the existing toolchain, tag, and
publish. No new script, service, package, or release automation.

**Excluded machinery:** aarch64 artifacts, LSP packaging, CI release
automation, a new packaging framework, and any feature work.

**Approved complexity:** none beyond the existing manual release process.

## Document Index

| # | Document | Description |
|---|---|---|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Every material decision, including the auto-design records |
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Version, docs, gates, artifacts, publication, records |
| 02 | [Current State](02-current-state.md) | Release surfaces, runbook, gates, and repository state |
| 03-01 | [Version and Docs](03-01-version-and-docs.md) | Exact version surfaces, changelog, notes, runbook |
| 03-02 | [Gates, Artifacts, Publication](03-02-gates-and-publication.md) | Gate order, comparison, packaging, tag, publish |
| 05 | [Findings](05-findings.md) | Gate results, measurements, artifact hashes, publication |
| 07 | [Testing Strategy](07-testing-strategy.md) | Release checklist and pass conditions |
| 99 | [Execution Plan](99-execution-plan.md) | Phases and tasks |
| PF-R | [Requirements Preflight](00-preflight-report-requirements.md) | Adversarial requirements audit and rulings |
| PF-P | [Plan Preflight](00-preflight-report.md) | Adversarial plan audit and rulings |

## Decisions

| Decision | Value |
|---|---|
| Version | `1.1.2` (patch) |
| Scope | PLAN-25 constant arrays, PLAN-26 release flags, the `ffi` shadowing fix |
| Comparison base | `v1.1.1` (`2d1eb30`), same shipped flags |
| Gates | The v1.1.1 set plus `make bench-flags` |
| Artifacts | Source + Linux x86_64 tarballs + `SHA256SUMS`; no aarch64, no LSP; the binary tarball includes the notices and limitations docs |
| Tag | Annotated `v1.1.2` on the green release-evidence commit (docs-only diff from the gated commit) |
| Publication | Draft release, asset verification, then published; authorized by the user |
| Execution mode | `--auto-commit --auto-design`; push `main` and the tag only after the gates and artifact audit |

## Specialist Agents

**None** — the **closeout diff** is version metadata, docs, and records. The
existing `correctness-reviewer` checks the claims and surfaces. The shipped
codegen and emission changes were reviewed in PLAN-25 and PLAN-26; the release
re-runs the full gates and cites that evidence rather than re-deriving it.

_Detection evidence: the closeout diff touches `compiler/Cargo.toml`,
`Cargo.lock`, `tools/vscode/package*.json`, `README.md`, `RELEASE.md`,
`CHANGELOG.md`, `RELEASE-NOTES.md`, `docs/known-limitations.md`, and plan
records. The **release diff**
(v1.1.1..HEAD) contains the already-reviewed PLAN-25/PLAN-26 compiler and spec
changes; they are shipped, not modified, by this plan._

## Scope Boundary

In scope: version surfaces; changelog and release notes; runbook; gate run;
scored comparison and flag audit; artifacts and checksums; tag and publication;
release records and roadmap.

Out of scope: language, grammar, diagnostics, ABI, stdlib, runtime shim, spec,
CLI, aarch64/Windows artifacts, LSP packaging, release automation, features,
and any change to published history.

## Related Files

- `compiler/Cargo.toml`, `Cargo.lock`, `tools/vscode/package.json`,
  `tools/vscode/package-lock.json` — version surfaces.
- `README.md` — current-release line.
- `CHANGELOG.md`, `RELEASE-NOTES.md` — release writing.
- `RELEASE.md` — the runbook and publication record.
- `THIRD_PARTY_NOTICES.md`, `docs/known-limitations.md` — shipped in the
  binary tarball so its licensing links resolve.
- `plans/00-roadmap.md` — the release row.
- `build/release/v1.1.2/` — gitignored artifacts.
- `plans/constant-array-initializer/05-findings.md`,
  `plans/performance-levers-closeout/05-findings.md` — the measured claims.
