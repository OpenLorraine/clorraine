# Version and Documentation: v1.1.2 Release Closeout (PLAN-27)

> **Document**: 03-01-version-and-docs.md
> **Parent**: [Index](00-index.md)

## Overview

This component bumps the version surfaces and writes the release record. It
does not change compiler behavior. The release notes use three measurement
bases and never mix them.

## Version Surfaces

| File | Current | Target | How |
|---|---|---|---|
| `compiler/Cargo.toml` | `1.1.1` | `1.1.2` | edit `version =` |
| `Cargo.lock` | `clc 1.1.1` | `clc 1.1.2` | `cargo update -p clc` or a workspace build that refreshes the lock |
| `tools/vscode/package.json` | `1.1.1` | `1.1.2` | edit `version` |
| `tools/vscode/package-lock.json` | `1.1.1` | `1.1.2` | npm version/lock update; the extension tests compare it to Cargo |
| `README.md` | "v1.1.1 is the current release" | "v1.1.2 is the current release" | edit the status paragraph |
| `RELEASE.md` | v1.1.1 runbook | v1.1.2 runbook | rewrite the decision, gates, artifacts, publication |
| `CHANGELOG.md` | 1.1.1 top | add `## [1.1.2]` | new section |
| `RELEASE-NOTES.md` | v1.1.1 notes | v1.1.2 notes | rewrite |
| `docs/known-limitations.md` | LSP sentence names v1.1.1 | generic current policy | edit §12 only |
| `tools/demo/package.json` | independent `0.0.0` | unchanged | none |

`clc --version` and `clorraine --version` print `CARGO_PKG_VERSION`; both must
print `1.1.2`. The Rust driver binaries are not stripped; `file` must report
`not stripped` for both.

## Changelog Entry

```markdown
## [1.1.2] - YYYY-MM-DD

### Changed

- Module-level constant scalar array initializers emit one read-only data
  template and one copy instead of one store per element. Isolated under the
  old flags, `lookup` net instructions fell 13,653 → 9,470 (−30.6%) and
  `.text` 117,440 → 92,976 (−20.8%). Against `v1.1.1` on `hosted-amiga`,
  with the shipped flags applied to both trees, `lookup` net instructions fell
  13,653 → 9,470, `.text` 64,328 → 39,904 (−38.0%), and the binary
  82,544 → 41,740 (−49.4%).
- Release programs on Linux host, `hosted-amiga`, and `hosted-amiga-13` build
  with `-O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`; the two
  GCC release rows in `spec/17-codegen.md` §13 match. Release programs are
  stripped; debug builds, freestanding, and non-Linux host program builds keep
  symbols.
- The same-compiler flag audit measures the five scored benchmarks at `.text`
  57.1–69.3% and binaries 64.2–74.7% smaller; host binaries fall 20.6–36.5%;
  the 1.3 audit set falls 49.1–88.9% in `.text`; calibration falls
  3,372 → 2,928 (2.0)
  and 3,348 → 2,947 (1.3). Scored net instructions are unchanged by the flags.
- The constant-array template uses a collision-free symbol, so it cannot
  shadow a user `ffi` symbol. The collision existed only in unreleased builds.
- The Linux x86_64 tarball contains `clc` and `clorraine`, not
  `clorraine-lsp`; build the server from source.

### Verification

`make check`, `make asan`, `make ubsan`, `make corpus-amiga`,
`make differential`, `make bench-shape`, `make bench-base COMMIT=v1.1.1`,
`make bench`, `make bench-compare`, `make bench-flags`, `make verify`, and
`make vscode-check` pass for v1.1.2. `make verify` prints
`verify: all legs passed`.
```

The release-tree comparison (`v1.1.1` → head, shipped flags) is `lookup`
13,653 → 9,470 net (−30.6%), `.text` 64,328 → 39,904 (−38.0%), binary
82,544 → 41,740 (−49.4%). The same-compiler flag audit measures the five
scored benchmarks at `.text` 57.1–69.3% and binaries 64.2–74.7%, host
binaries 20.6–36.5%, the 1.3 audit set 49.1–88.9%, and calibration
3,372 → 2,928 (2.0) / 3,348 → 2,947 (1.3). The notes quote the release-tree headline and the flag audit as
supporting evidence.

## Release Notes

`RELEASE-NOTES.md` structure:

1. **Title** — `# Clorraine v1.1.2`.
2. **Intro** — patch release; language and CLI unchanged.
3. **Highlights** — constant arrays, smaller release programs, collision-free
   template symbol.
4. **Verification** — the gate list and the release-tree numbers.
5. **Known limitations** — stripped release programs, freestanding/non-Linux
   exceptions, `clc`/`clorraine` keep symbols, the two §13 release rows now
   include section GC and strip, `clorraine-lsp` not in the tarball, and the
   accepted limits doc.
6. **Licence** — MIT; `LICENSE` and `THIRD_PARTY_NOTICES.md` are in the
   tarball.

The notes must not claim a runtime win from the flags. The scored net
instruction count is unchanged by the flags; the constant-array change
improves `lookup` net by 30.6%.

## Runbook

`RELEASE.md` keeps its section structure, rewritten for v1.1.2:

| Section | Content |
|---|---|
| Release decision | version, date, licence, source tag, artifacts, spec, aarch64 deferral |
| Pre-release gates | the full gate list, exact `REQUIRE`, identity assertions, gate table, Node/`gh` prerequisites |
| Licence and metadata audit | every surface, plus the new tarball docs |
| Build and artefact packaging | exact commands, staging tree, archive audit, checksums |
| Conditional publication | draft creation, asset verification, publish, page confirmation |
| Historical publication | v1.1.1, v1.1.0, v1.0.0 sections preserved after publication |
| Failure handling | the R3.5 table: gate, version, licence, dirty tree, tag, build, bench-base, publication, archive |

The runbook records the **gated commit** and the **tag commit** and asserts the
tag's diff from the gated commit is documentation-only. Gate results belong in
`05-findings.md` (full ledger) and the runbook (summary); pre-tag it holds gate
results and measured values, post-tag it holds the URL, asset sizes, hashes,
and final status.

## Documentation Policy

- **Current-release statements** change: `README.md`, `RELEASE.md` decision
  and publication status, notes, changelog.
- **Historical statements** stay: `docs/known-limitations.md` "fixed in
  v1.1.1", `RELEASE.md` historical sections for v1.1.0/v1.0.0.
- The LSP packaging sentence is restated generically, not tied to v1.1.1.
- The spec is not edited by this plan. The two §13 release rows already
  changed in PLAN-26 under the user's separate ruling; the runbook's
  Specification row records that.

## Closeout Diff Allowlist

The plan's own commits may touch only:

```
compiler/Cargo.toml
Cargo.lock
tools/vscode/package.json
tools/vscode/package-lock.json
README.md
CHANGELOG.md
RELEASE-NOTES.md
RELEASE.md
docs/known-limitations.md
AGENTS.md
codeops/specialist-check.json
plans/00-roadmap.md
plans/v1.1.2-release-closeout/**
```

No `compiler/src/**`, `spec/**`, `stdlib/**`, or `compiler/runtime/**` file may
change. A diff outside this list is a stop condition.
