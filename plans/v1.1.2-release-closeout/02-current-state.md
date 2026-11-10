# Current State: v1.1.2 Release Closeout (PLAN-27)

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Release Process

The repository releases manually from `main`. `RELEASE.md` is the runbook and
the version of record is `compiler/Cargo.toml`. The v1.1.1 sequence was:

1. `chore(release): prepare v1.1.1` (`2bc279e`) — version bump, changelog, and
   notes.
2. `docs(release): record the v1.1.1 release gates` (`2d1eb30`) — the tagged
   commit, with the gate table filled in.
3. Annotated tag `v1.1.1` (tag object `144e53b`, commit `2d1eb30`).
4. Build the source and Linux x86_64 tarballs, `SHA256SUMS`, and publish.
5. `docs(release): record the v1.1.1 publication` (`a8f49b8`) — post-tag.

The published release has exactly three assets: the source tarball, the Linux
x86_64 tarball, and `SHA256SUMS`. `RELEASE-NOTES.md` is the release body, not
an asset. The binary tarball contains six files: `clc`, `clorraine`,
`LICENSE`, `README.md`, `RELEASE-NOTES.md`, and `CHANGELOG.md`.

### What Changed Since v1.1.1

| Commit | Change | User-visible |
|---|---|---|
| `cef0b91`, `fd7fe30` | Constant array initializers emit a data template + copy; collision-free symbol | Yes |
| `b019771`, `cda97cc` | Section GC + strip release flags; two spec §13 rows | Yes |
| `902ef30` | Remove dead compiler code and unused warnings | No (behavior-neutral) |
| `a589353`, `ac92308` | Transient vbcc config-probe retry | Tooling |
| Plans, tests, docs | PLAN-25/PLAN-26 records, pins, roadmap | No |

`v1.1.1` is 31 pre-plan commits behind `main` (`3227321`); this plan adds its
own record commits. The roadmap is 570/572 with
two parked rows; there is no open feature.

## Relevant Files

| File | Purpose | Change Needed |
|---|---|---|
| `compiler/Cargo.toml` | Version of record | `1.1.1` → `1.1.2` |
| `Cargo.lock` | Locked `clc` version | Regenerate to `1.1.2` |
| `tools/vscode/package.json`, `package-lock.json` | Extension version | `1.1.1` → `1.1.2` |
| `tools/demo/package.json` | Separate npm package | No change |
| `README.md` | Current-release line | `v1.1.1` → `v1.1.2` |
| `CHANGELOG.md` | Release history | Add one `[1.1.2]` section |
| `RELEASE-NOTES.md` | Release body | Rewrite for v1.1.2 |
| `RELEASE.md` | Runbook and publication record | Rewrite for v1.1.2; keep historical v1.1.0/v1.0.0 sections |
| `docs/known-limitations.md` | Accepted limits | Generalize the LSP packaging sentence; keep the v1.1.1 fix history |
| `THIRD_PARTY_NOTICES.md` | Upstream notices | No content change; add to the binary tarball |
| `plans/00-roadmap.md` | Tracker | Add PLAN-27; close at 571/573 |
| `codeops/specialist-check.json` | Specialist coverage | Refresh to 74 plan folders |
| `AGENTS.md` | Coverage line | 74 plan folders, 2026-11-11 or the closeout date |

## Gate Surface

| Gate | What it proves |
|---|---|
| `make check` | Spec check, registry, workspace tests, bench tests, fuzz tests and smoke |
| `make asan` / `make ubsan` | Sanitizer legs |
| `make corpus-amiga` / `make differential` | Portable corpus behavior on host and m68k |
| `make bench-shape` | Warning-free GCC/vbcc/freestanding shapes |
| `make bench-base COMMIT=v1.1.1` / `make bench` / `make bench-compare` | Release-tree emission comparison |
| `make bench-flags` | Same-compiler old/GC/shipped size and constructor audit |
| `make verify` | Full acceptance, including the A600/A1200 release gates |
| `make vscode-check` | Extension type-check, tests, package audit, packaged client |

`make verify` runs the RTG gates, which regenerate the tracked
`docs/amiga-sdk/*.png` captures. An unintended change is restored before the
evidence commit; a material difference is investigated and stops the release.

## Dependencies

### Internal Dependencies

- The measured claims come from `plans/constant-array-initializer/05-findings.md`
  and `plans/performance-levers-closeout/05-findings.md`.
- The gate definitions come from `Makefile` and `AGENTS.md`.
- The release procedure comes from `RELEASE.md`.
- The tag and publication require a clean, green `main`.

### External Dependencies

- The pinned Docker SDK image for the release build and gates.
- GitHub for the tag and release publication.
- No new crate, npm package, or service.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A measurement basis is mixed in the notes | Medium | High | The three-basis table; the correctness review checks every claim |
| A stale `build/perf` document is compared | Medium | High | Delete first; assert commit/dirty/sha identity |
| `make verify` leaves regenerated PNGs dirty | Medium | Medium | Check `git status` after verify; commit or restore before the evidence commit |
| A flaky FS-UAE gate delays the release | Medium | Low | One audited rerun; a second failure stops |
| A partial asset upload publishes an incomplete release | Low | High | Draft release, asset verification, then publish |
| The tag points at the wrong commit | Low | High | Record the gated and tagged hashes; assert the docs-only diff |
| The new tarball docs change a hash unexpectedly | Low | Low | Recompute and record; artifact names stay stable |
