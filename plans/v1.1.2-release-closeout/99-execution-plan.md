# Execution Plan: v1.1.2 Release Closeout (PLAN-27)

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-10
> **Progress**: 0/40 tasks (0%)
> **CodeOps Artifact Schema**: 1

## Overview

Bump the version, run the full release gate on the gated commit, write the
release record, review the claims, tag locally, build and audit the artifacts,
push and publish, then close the plan. No product code changes.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
|---|---|---|
| 1 | Plan baseline and pre-release checks | 5 |
| 2 | Version bump and full gate (C1) | 14 |
| 3 | Release record, tag, artifacts, publication (C2) | 13 |
| 4 | Publication record and closeout (C3) | 8 |

**Total: 40 tasks across 4 phases**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes below are the single source of truth for progress.
> Mark a task `[x]` with the completion timestamp after its verify passes.
> A blocker is `[!]` with a short reason. Timestamps come from
> `date '+%Y-%m-%d %H:%M'`; never invent one.

---

## Phase 1: Plan Baseline and Pre-Release Checks

> **Lenses**: compiler and language; data and migration (versioned release
> artifacts).
> **Reasoning**: high — the tag and publication are irreversible.

### Step 1.1: Commit the plan and ground the baseline

**Reference**: [02](02-current-state.md) · AR-01/AR-03/AR-16

- [ ] 1.1.1 Add the PLAN-27 roadmap row at `Plan Preflighted` and commit the
  plan folder, this report, and the row
- [ ] 1.1.2 Fetch and assert `main == origin/main`; record the pre-release
  head, the `v1.1.1` tag object/commit, the version surfaces, and preserve the
  current `build/perf` evidence
- [ ] 1.1.3 Confirm `v1.1.1` builds and compiles the current benchmark set
  (including the post-v1.1.1 `lookup` benchmark) in the pinned container
- [ ] 1.1.4 Classify the 31 pre-plan commits since v1.1.1 (product,
  behavior-neutral, tooling, records) and record the release scope in
  `05-findings.md`
- [ ] 1.1.5 Confirm the only current-version statements are the allowlisted
  files and no repository test pins `1.1.1` as the current version

**Verify**: `git status -sb` clean; the plan commit exists; the baseline notes
exist.

---

## Phase 2: Version Bump and Full Gate (C1)

> **Lenses**: compiler and language; data and migration.
> **Reasoning**: max — every gate runs here.

### Step 2.1: Bump and verify

**Reference**: [03-01](03-01-version-and-docs.md) · [03-02](03-02-gates-and-publication.md) · R1/R4 · RT-1–RT-16

- [ ] 2.1.1 Bump `compiler/Cargo.toml`, regenerate `Cargo.lock`, bump
  `tools/vscode/package.json` and `package-lock.json`
- [ ] 2.1.2 Update the `README.md` current-release line
- [ ] 2.1.3 `cargo build --release --workspace`; assert `clc 1.1.2`,
  `clorraine 1.1.2`, ELF, `not stripped`, and run the hello-world checks
- [ ] 2.1.4 Commit C1 (`chore(release): bump the version to 1.1.2`); do not
  push before the gates
- [ ] 2.1.5 Delete stale `build/perf/{base,head,flags}.json` and
  `build/perf/base-clc`; assert a clean tree
- [ ] 2.1.6 Run `make check`
- [ ] 2.1.7 Run `make asan` and `make ubsan`
- [ ] 2.1.8 Run `make corpus-amiga`, `make differential`, and
  `make bench-shape`
- [ ] 2.1.9 Run `make bench-base COMMIT=v1.1.1`, `make bench`, and
  `make bench-compare REQUIRE="lookup:instructions:20 lookup:text:20"`
- [ ] 2.1.10 Run `make bench-flags`; assert `runs: 2`, clean identity, and all
  checks pass
- [ ] 2.1.11 Run `make verify`; assert `verify: all legs passed` and check
  `git status` for unintended RTG capture changes
- [ ] 2.1.12 Run `make vscode-check`
- [ ] 2.1.13 Record the gated commit, all identities, measurement tables, and
  gate results in `05-findings.md`
- [ ] 2.1.14 Dispatch the end-of-phase correctness review (version surfaces,
  identities, gate evidence); fix every critical/major finding before C2

**Verify**: every gate green; identity assertions pass; the review has no
open critical/major finding.

---

## Phase 3: Release Record, Tag, Artifacts, Publication (C2)

> **Lenses**: compiler and language; data and migration.
> **Reasoning**: max — the tag and assets become public.

### Step 3.1: Write the record and publish

**Reference**: [03-01](03-01-version-and-docs.md) · [03-02](03-02-gates-and-publication.md) · R2/R3/R5/R6 · RT-17–RT-24

- [ ] 3.1.1 Add the `[1.1.2]` changelog entry with the measured values
- [ ] 3.1.2 Rewrite `RELEASE-NOTES.md` for v1.1.2
- [ ] 3.1.3 Rewrite `RELEASE.md` through the pre-publication state
  (decision, gates, artifacts, publication checklist, failure handling)
- [ ] 3.1.4 Generalize the LSP packaging sentence in
  `docs/known-limitations.md` §12
- [ ] 3.1.5 Complete the measurement, gate, and scope sections of
  `05-findings.md`
- [ ] 3.1.6 Dispatch the pre-publication claim review (changelog, notes,
  runbook against `05-findings.md` and `build/perf/*.json`); fix every
  critical/major finding
- [ ] 3.1.7 Commit C2 (`docs(release): record the v1.1.2 gates`); assert the
  diff from C1 is documentation-only; run a final `make check` and both
  `--version` assertions
- [ ] 3.1.8 Create the annotated `v1.1.2` tag locally on C2
- [ ] 3.1.9 Assert a clean tree and `HEAD == v1.1.2^{commit}`; build
  `cargo build --release --workspace`; run the hello-world program with both
  release binaries
- [ ] 3.1.10 Package the source and binary tarballs and `SHA256SUMS`; run the
  archive audit and `sha256sum -c`
- [ ] 3.1.11 Push `main` and the tag (only after the artifact audit passes)
- [ ] 3.1.12 Create the draft GitHub release; upload the three assets; verify
  `isDraft` and the asset names, sizes, and states
- [ ] 3.1.13 Publish the release; confirm the page lists the notes and all
  three assets; record the URL

**Verify**: the tag is on C2; the artifacts match the recorded hashes and pass
the audit; the published release has exactly three assets.

---

## Phase 4: Publication Record and Closeout (C3)

> **Lenses**: compiler and language; data and migration.
> **Reasoning**: medium — bounded closeout.

### Step 4.1: Close the record

**Reference**: [00-index](00-index.md) · AR-16/AR-17 · R7 · RT-25–RT-28

- [ ] 4.1.1 Update `RELEASE.md` with the publication status, URL, asset sizes,
  and SHA-256 values; move v1.1.1 to a historical section
- [ ] 4.1.2 Update `05-findings.md` with the publication record, final
  numbers, review rulings, and accepted limits
- [ ] 4.1.3 Mark PLAN-27 Done; set the roadmap to 571/573, 0 open, 2 parked;
  update the totals note
- [ ] 4.1.4 Refresh `codeops/specialist-check.json` to 74 plan folders and
  update `AGENTS.md`
- [ ] 4.1.5 Dispatch the final correctness review of the release record and
  claims
- [ ] 4.1.6 Fix every critical/major finding and rerun the affected checks
- [ ] 4.1.7 Commit C3 (`docs(release): record the v1.1.2 publication`); push;
  confirm a clean tree and `main == origin/main`
- [ ] 4.1.8 Final verification: tag on `origin`, release assets visible, plan
  40/40 Done

**Verify**: `git status -sb`, `git ls-remote --tags origin v1.1.2`,
`gh release view v1.1.2`, and the plan progress.

---

## Dependencies

```
Phase 1
    ↓
Phase 2  (C1 + gates)
    ↓
Phase 3  (C2 + claim review + local tag + artifacts + push + publish)
    ↓
Phase 4  (C3 + closeout)
```

The tag depends on the full gate (2.1.6–2.1.12) and the claim review (3.1.6).
Artifacts depend on the local tag (3.1.8). Push depends on the artifact audit
(3.1.10). Publication depends on the push (3.1.11). The closeout depends on
the publication record (3.1.13).

## Contingency

- **A missed gate stops the release.** An environmental/tooling fix may be
  applied; a compiler or test failure stops the release and needs a user
  ruling. Never lower a gate.
- **A flaky environmental leg** (FS-UAE/display, licensed media, container,
  network) may be rerun once with the failing leg and output recorded; a
  compiler or test failure is not flaky, and a second failure stops.
- **A dirty tree** before a measurement stops the measurement; clean or
  commit, then rerun.
- **The tag exists locally only**: delete and recreate after confirming
  `origin` lacks it. **On `origin`**: stop; never force-move.
- **A partial asset upload**: retry `gh release upload`; never replace an
  asset or delete the release without a recorded user ruling.
- **A push is rejected because `origin` advanced**: stop; do not rebase the
  measured commit.
- **A material RTG capture difference**: investigate; do not commit it to hide
  an unexplained rendering change.
- **The source snapshot is pre-publication**: the publication record lives in
  the repository and release page (AR-17).

## Success Criteria

1. ✅ All 40 tasks complete.
2. ✅ Version `1.1.2` on every surface; both `--version` lines exact.
3. ✅ The full gate is green on C1; identities are clean.
4. ✅ The release-tree comparison meets the 20% `lookup` floors with no
   regression above 1%.
5. ✅ `make bench-flags` passes with `runs: 2`.
6. ✅ The tag is annotated on C2; its diff from C1 is docs-only.
7. ✅ Two tarballs plus `SHA256SUMS`; the archive audit is clean.
8. ✅ The published release lists the notes and all three assets.
9. ✅ The roadmap reads 571/573, 0 open, 2 parked; the specialist check
   records 74.
10. ✅ The tree is clean and `main == origin/main`.
