# Requirements Preflight Report: v1.1.2 Release Closeout (PLAN-27)

> **Document**: 00-preflight-report-requirements.md
> **Parent**: [Index](00-index.md)
> **Audit target**: [`01-requirements.md`](01-requirements.md)
> **Context documents**: [`00-ambiguity-register.md`](00-ambiguity-register.md),
> [`00-index.md`](00-index.md), `RELEASE.md`, `CHANGELOG.md`,
> `RELEASE-NOTES.md`, `plans/00-roadmap.md`
> **Scope mode**: strict
> **Verdict**: ✅ **PASSED** — every critical/major finding is resolved; every
> minor finding was fixed, not accepted.

> ⚠️ **Same-session review.** The requirements were authored in this session.
> Two independent read-only auditors scanned them against the real repository,
> and a third re-scanned the fixes. The two reserved decisions (version and
> publication) were made by the user.

## Codebase Context Summary

| Fact | Evidence |
|---|---|
| v1.1.1 is the previous published release | annotated tag `v1.1.1` at `2d1eb30` |
| 31 commits have landed since | `git log --oneline v1.1.1..HEAD` |
| The release-tree comparison is measurable | preflight rebuilt `v1.1.1`; `lookup` `.text` 64,328, binary 82,544, net 13,653; head 39,904 / 41,740 / 9,470 |
| Version surfaces | `compiler/Cargo.toml:5`, `Cargo.lock:6-7`, `tools/vscode/package.json:5`, `tools/vscode/package-lock.json:3,9`, `README.md:54` |
| The v1.1.1 binary tarball omits the notices file its README links | v1.1.0/v1.1.1 tarball listing; `README.md:112-114` |
| The v1.1.1 runbook lists six tarball files | `RELEASE.md:113` |
| `make verify` includes the A600/A1200 release gates | `Makefile:217-218` |
| `make bench-flags` exists and records `dirty` | `Makefile:119-120`; `tools/perf/flag_audit.py` |
| `make verify` regenerates tracked RTG PNGs | `docker/scripts/rtg-smoke.sh:320` |
| The v1.1.1 tag points at the gate-evidence commit, not the measured head | tag `2d1eb30`; measured head `2bc279e` |

## Scan 1 — Soundness and Delivery

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| PF-001 | 🔴 MAJOR | Three measurement bases were conflated: the isolated old-flag constant-array result, the release-tree comparison, and the same-compiler flag audit. `.text` 20.8% was presented as the release-tree win; it is 38.0%, and 57–74% is the flag effect. | Added the three-row basis table to AR-03 and R2.3; the release-tree row is the headline, the flag audit is separate. |
| PF-002 / PF-106 | 🔴 MAJOR | N6 required third-party notices in the binary tarball but R5.4 omitted them; the shipped README/notes link a file the tarball lacked. | R5.4 now includes `THIRD_PARTY_NOTICES.md` and `docs/known-limitations.md`; AR-08 and N6 agree. |
| PF-003 | 🔴 MAJOR | The index/AR-11 claimed the release diff was version/docs only; `v1.1.1..HEAD` changes 12 compiler files and the spec. | Distinguished the **closeout diff** (version/docs/records) from the **release diff** (shipped, already-reviewed PLAN-25/26); specialist rationale updated. |
| PF-004 / PF-103 / PF-101 | 🔴 MAJOR | The plan said gates ran "on the tagged tree", but the tag follows the evidence commit; the changelog also preceded its measurement. | New sequence: bump commit → gates on it → docs/evidence commit with measured values → tag → artifacts → publish → publication commit. R3.3 records both hashes and the docs-only diff. |
| PF-005 | 🔴 MAJOR | "Release binaries are stripped" was false for the `clc`/`clorraine` Rust binaries; only generated programs get `-s`. | R2.5/R1.7 scope the claim to `clorraine build --release` programs and assert the driver binaries are not stripped. |
| PF-102 | 🔴 MAJOR | `gh release create` publishes before asset upload; a partial upload would leave a public release with missing assets. | R6.3/AR-10 use a draft release, verify `isDraft` and assets with `gh release view --json isDraft,assets`, then publish; a partial upload is retried. |
| PF-006 | 🟡 MINOR | "Scored" ranges excluded the 1.3 `sieve` row and cited calibration as scored. | The five scored benchmarks are `.text` 57.1–69.3% / binaries 64.2–74.7%; the 1.3 audit set (calibration, sieve, four corpus) is 49.1–88.9%; calibration profiles labeled. |
| PF-007 / PF-104 | 🟡 MINOR | `bench-flags` did not enforce a clean tree, and stale perf documents were not rejected. | R4.1/R4.7 delete stale JSONs, require a clean tree, and assert `base`/`head`/`flags` identities, `dirty: false`, `runs: 2`, and the SHA-256 match. |
| PF-008 | 🟡 MINOR | "Release notes attached" would imply four assets. | `RELEASE-NOTES.md` is the body (`--notes-file`); three assets. |
| PF-009 / PF-105 | 🟡 MINOR | Artifact commands, binary prefix, and archive audit were underspecified; the source snapshot carries the pre-publication runbook. | Exact `git archive` and `sha256sum -c` commands, prefix `clorraine-1.1.2/`, denylist with committed `.example` files allowed, and R5.8 records the snapshot boundary. |
| PF-010 / PF-107 | 🟡 MINOR | The symbol-shadowing fix was listed as `Fixed` although only unreleased builds had the bug. | Moved to `### Changed`; described as collision-free symbol naming found before release. |
| PF-011 | 🟡 MINOR | Roadmap counter and specialist coverage were unpinned/stale. | R7.2 pins 571/573; R7.4 refreshes the specialist check to 74. |
| PF-012 / PF-108–PF-115 | 🟡 MINOR | Under-specified hello output, flaky class, `REQUIRE` string, citations, spec-row currency, failure paths, AR-13 wording, "runtime" naming, floor provenance, evidence ownership. | All fixed: hello output named; flaky class defined; `REQUIRE` pinned; citations corrected; §13/LSP doc currency added (R2.7/R2.8/R3.6); failure paths expanded (R3.5/R6.6); AR-13 reworded; "net instructions" used; AR-04 confirms the floor; AR-17 sets the evidence boundary. |

## Re-scan

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| RS-001 | 🟠 MAJOR | AR-03 still called "57–74%" a headline, mixing `.text` and binary bounds. | AR-03 uses the release-tree headline (49.4% binary / 38.0% `.text`) and keeps the five-scored/1.3-audit ranges separate. |
| RS-002 | 🟠 MAJOR | `flags.json` identity was not fully asserted. | R4.1 now asserts `clc.git`, `clc.dirty`, `clc.version`, and `clc.sha256 == head.json.compiler.sha256`. |
| RS-003 | 🟡 MINOR | The 10% floor margin sentence was inverted. | Reworded: a 10% floor is a weaker gate with more slack. |
| RS-004 | 🟡 MINOR | Index status and missing preflight link. | Status aligned; this report supplies the link. |
| RS-005 | 🟡 MINOR | N6 overclaimed which README links resolve. | N6 is scoped to the licensing links. |
| RS-006 | 🟡 MINOR | The LSP packaging sentence in `docs/known-limitations.md` still named v1.1.1. | R2.8 adds the generic rewrite to the closeout. |
| RS-007 | 🟡 MINOR | The archive audit allowed only one `.example` file. | Both committed `.example` files are allowed. |
| RS-008 | 🟡 MINOR | The 1.3 calibration pair was missing. | Added 3,348 → 2,947. |
| RS-009 | 🟡 MINOR | Stale artifacts could enter `SHA256SUMS`. | R5.4 deletes and recreates the artifact and staging directories first. |
| RS-010 | 🟡 MINOR | The roadmap-totals note would go stale. | R7.2 updates both the progress line and the note. |

## Final Verification

- Every critical/major finding is resolved in the owning document and traced
  into a requirement.
- Every minor finding is fixed, not accepted.
- The two reserved decisions have explicit user rulings: version `1.1.2` and
  publication when green.
- The 13 audit dimensions were re-scanned over the final target; no new
  critical/major issue remains.
- The requirements are internally consistent with the repository evidence and
  the roadmap arithmetic.

**Verdict: ✅ PREFLIGHT PASSED — 23 consolidated findings (8 major, 15
minor/observation) across two scans and one re-scan; all resolved, 0 open.**
