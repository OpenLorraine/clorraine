# Requirements: v1.1.2 Release Closeout (PLAN-27)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Revision**: revised — requirements preflight findings resolved
> **Status**: Approved
> **Base**: previous published release `v1.1.1` (`2d1eb302…`)

## Feature Overview

Clorraine has two completed, measured improvements on `main` that are not in
any published release:

- **Constant array initializers** (PLAN-25): a module-level fixed array with a
  complete constant scalar list emits one read-only data template and one copy
  instead of one store per element. Isolated under the old flags, `lookup` net
  instructions fell 13,653 → 9,470 (−30.6%) and `.text` 117,440 → 92,976
  (−20.8%).
- **Build flags and dead-code elimination** (PLAN-26): release programs on
  Linux host, `hosted-amiga`, and `hosted-amiga-13` now build with
  `-O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`. The
  same-compiler audit measures the five `hosted-amiga` scored benchmarks
  (sieve, matrix, strings, moves, lookup) at `.text` 57.1–69.3% smaller and
  stripped binaries 64.2–74.7% smaller; the `hosted-amiga-13` audit set
  (calibration, sieve, four corpus programs) is 49.1–88.9% smaller in
  `.text`.

The repository is otherwise idle: the roadmap is 570/572, with the two parked
items (LorraineOS, ARM64) explicitly inactive. The practical next step is to
publish a patch release that delivers these wins, with the same rigor as
v1.1.1: version bump, changelog, release notes, runbook, full gates,
checksummed artifacts, annotated tag, and publication.

This release adds no new language, ABI, specification, standard-library, or
CLI behavior. It ships the existing tree and fixes the release tarball's
missing licensing references.

## Scope Boundary

**In scope:** the v1.1.2 version surfaces; one changelog entry and the release
notes; the v1.1.2 release runbook; the release gate run on the gated commit;
the scored comparison against v1.1.1 and the same-compiler flag audit; the
source and Linux x86_64 artifacts with `SHA256SUMS`; the annotated `v1.1.2` tag
and its publication; the release record, roadmap row, and closeout.

**Out of scope:** any new compiler, codegen, IR, stdlib, runtime-shim, spec, or
CLI behavior; a new feature or optimization; aarch64 or Windows artifacts;
`clorraine-lsp` in the binary tarball; release automation; a version bump
beyond `1.1.2`; moving or editing the `v1.1.1` tag.

The **closeout diff** (this plan's commits) changes only version metadata,
release docs, and plan records. The **release diff** (31 commits since
v1.1.1) ships the already-reviewed PLAN-25 and PLAN-26 compiler and spec
changes plus one behavior-neutral compiler cleanup (`902ef30`).

## Comparisons

- **Release-tree comparison (scored):** the `v1.1.1` compiler vs the v1.1.2
  head, both measured by the current harness with the shipped release flags.
  `make bench-compare REQUIRE="lookup:instructions:20 lookup:text:20"` enforces
  output equality, the 1% guard, and the constant-array floors. This isolates
  the emitted-C improvement from the flag change.
- **Flag audit (attribution):** one `clc` binary, old vs GC vs shipped flags,
  via `make bench-flags`. This is where the same-compiler size win is
  measured: the five `hosted-amiga` scored benchmarks are `.text` 57.1–69.3%
  and binaries 64.2–74.7% smaller; the `hosted-amiga-13` audit set is
  49.1–88.9% smaller in `.text`.
- **Isolated PLAN-25 evidence:** the original old-flag measurement of the
  constant-array change, cited as historical evidence.

The three bases are never mixed. The release notes use the release-tree
comparison for the headline and the flag audit for the size range.

## Functional Requirements

### R1 — Version and metadata

- **R1.1** `compiler/Cargo.toml` declares `version = "1.1.2"`.
- **R1.2** `Cargo.lock` records `clc` version `1.1.2` with no other unintended
  lock change.
- **R1.3** `tools/vscode/package.json` and `tools/vscode/package-lock.json`
  record `1.1.2`; the extension tests compare this to the Cargo version.
- **R1.4** `tools/demo/package.json` keeps its independent package version.
- **R1.5** `target/release/clc --version` prints exactly `clc 1.1.2` and
  `target/release/clorraine --version` prints exactly `clorraine 1.1.2`.
- **R1.6** `README.md` names v1.1.2 as the current release; historical
  statements about earlier releases stay truthful.
- **R1.7** `file target/release/clc target/release/clorraine` reports Linux
  x86_64 executables; the binaries themselves ship with symbols (the `-s`
  flag applies to generated programs, not the Rust driver).

### R2 — Changelog and release notes

- **R2.1** `CHANGELOG.md` gains exactly one `## [1.1.2] - <date>` section above
  1.1.1, with `### Changed` and `### Verification` subsections. A `### Fixed`
  subsection appears only for a defect that existed in a published release;
  the symbol-shadowing fix existed only in unreleased builds, so it is listed
  under `### Changed`.
- **R2.2** The changelog names the constant-array emission change, the release
  flag/DCE/strip change including the two updated GCC release rows in
  `spec/17-codegen.md` §13, and the collision-free data-template symbol.
- **R2.3** The changelog records the release-tree comparison against `v1.1.1`
  with the shipped flags for `hosted-amiga` `lookup`: net instructions
  13,653 → 9,470 (−30.6%), `.text` 64,328 → 39,904 (−38.0%), binary
  82,544 → 41,740 (−49.4%). It separately records the same-compiler flag
  audit (five hosted-amiga scored benchmarks: `.text` 57.1–69.3%, binaries
  64.2–74.7%; host binaries 20.6–36.5%; 1.3 audit set `.text` 49.1–88.9%;
  calibration
  3,372 → 2,928 on 2.0 and 3,348 → 2,947 on 1.3) and states that scored net
  instructions are unchanged by the flags.
- **R2.4** `RELEASE-NOTES.md` is rewritten for v1.1.2 with Highlights,
  Verification, Known limitations, and Licence sections.
- **R2.5** The notes state that **programs** produced by
  `clorraine build --release` on Linux host, `hosted-amiga`, or
  `hosted-amiga-13` are stripped; the `clc` and `clorraine` binaries ship with
  symbols; debug builds keep symbols; freestanding and non-Linux host program
  builds keep the `-O2` line and symbols.
- **R2.6** No claim exceeds the recorded evidence; the size win is not
  presented as a runtime win.
- **R2.7** The notes and changelog state that the compile matrix's two GCC
  release rows now include section GC and strip, and that the binary tarball
  excludes `clorraine-lsp` (build it from source).
- **R2.8** `docs/known-limitations.md` §12 no longer ties the LSP packaging
  exclusion to v1.1.1; it describes the current binary tarball policy
  generically.

### R3 — Release runbook

- **R3.1** `RELEASE.md` becomes the v1.1.2 runbook: title, decision table,
  gate list, licence audit, artifact names, and publication checklist.
- **R3.2** The runbook's gate list includes `make bench-flags` and the scored
  comparison against `v1.1.1` with the exact `REQUIRE` string.
- **R3.3** The runbook records the exact gate results on the **gated release
  commit**, the gated and tagged commit hashes, and the assertion that the
  tag's diff from the gated commit is documentation-only. A final `make check`
  and both `--version` assertions run on the tag commit.
- **R3.4** After publication, the runbook records the published URL, asset
  names, sizes, and SHA-256 values, and v1.1.1 moves into a historical
  section; v1.1.0 and v1.0.0 sections are preserved.
- **R3.5** Failure handling covers a failed gate, a version mismatch, a
  licence/metadata miss, a dirty tree, an existing local or remote tag, a
  failed release build after tagging, the `bench-base` worktree race, a failed
  publication, and a forbidden archive content.
- **R3.6** The runbook's Specification row states that the frozen v1.0 chapters
  and approved v1.1 amendments are unchanged, with the §13 release rows
  updated by the separately authorized PLAN-26 ruling.

### R4 — Release gates

- **R4.1** Before measuring, delete stale `build/perf/{base,head,flags}.json`,
  `git status --porcelain` empty, and assert the recorded identities:
  `base.json.compiler.git == 2d1eb302…` with `dirty: false`;
  `head.json.compiler.git ==` the gated commit with `dirty: false`;
  `flags.json.clc.git ==` the gated commit, `clc.dirty: false`,
  `clc.version == "clc 1.1.2"`, `clc.sha256 == head.json.compiler.sha256`,
  and `runs: 2`.
- **R4.2** `make check` passes on the gated release commit.
- **R4.3** `make asan` and `make ubsan` pass.
- **R4.4** `make corpus-amiga` and `make differential` pass.
- **R4.5** `make bench-shape` is warning-free.
- **R4.6** `make bench-base COMMIT=v1.1.1`, `make bench`, and
  `make bench-compare REQUIRE="lookup:instructions:20 lookup:text:20"` pass;
  no scored benchmark is more than 1% worse.
- **R4.7** `make bench-flags` passes on the clean gated commit with `runs: 2`
  and the floor, constructor, and repeat checks green.
- **R4.8** `make verify` prints `verify: all legs passed`.
- **R4.9** `make vscode-check` passes.
- **R4.10** After `make verify`, `git status --porcelain` is checked; an
  unintended RTG capture change is restored before the evidence commit. A
  capture that differs materially from the golden is investigated; it is not
  committed to hide an unexplained rendering change.
- **R4.11** A flaky **environmental** gate (FS-UAE/display, licensed media,
  container startup, or network) may be rerun once with the failing leg and
  output recorded; a compiler or test failure is never treated as flaky and a
  second failure stops the release.

### R5 — Artifacts

- **R5.1** Build `cargo build --release --workspace` inside the pinned SDK
  container.
- **R5.2** Compile and run `examples/hello/hello.clo` with the release `clc`
  and `clorraine`; assert the exact output
  `Hello, Amiga, from Clorraine!`. The `clc` path is
  `target/release/clc --profile host -o build/release/hello.c
  examples/hello/hello.clo` followed by a host `cc -std=c99 -O2` build; the
  `clorraine` path is `target/release/clorraine build --release --profile
  host examples/hello/hello.clo` followed by running the artifact.
- **R5.3** Confirm both binaries are Linux x86_64 ELF files and that the
  driver binaries are not stripped.
- **R5.4** Delete and recreate `build/release/v1.1.2/` and the
  `clorraine-1.1.2/` staging tree before packaging, then write under
  `build/release/v1.1.2/`:
  - `clorraine-1.1.2-src.tar.gz` via
    `git archive --format=tar.gz --prefix=clorraine-1.1.2/ -o <path> v1.1.2`;
  - `clorraine-1.1.2-linux-x86_64.tar.gz` from a staging tree
    `clorraine-1.1.2/` containing `clc`, `clorraine`, `LICENSE`, `README.md`,
    `RELEASE-NOTES.md`, `CHANGELOG.md`, `THIRD_PARTY_NOTICES.md`, and
    `docs/known-limitations.md`;
  - `SHA256SUMS` generated with `sha256sum *.tar.gz > SHA256SUMS` and verified
    with `sha256sum -c SHA256SUMS`.
- **R5.5** Audit both archives: no NDK content, ROMs, Workbench media,
  `build/` output, `target/` output, `docker/.env`, or
  `docker/compose.override.yaml`. The committed
  `docker/.env.example` and `docker/compose.override.yaml.example` are
  allowed.
- **R5.6** Record the artifact sizes and SHA-256 values in `RELEASE.md` and
  the findings.
- **R5.7** No aarch64 artifact; aarch64 host builds remain supported from
  source. The binary tarball excludes `clorraine-lsp`.
- **R5.8** The source tarball is the tag snapshot; the publication record is
  added to the repository and the release page after publication, not inside
  the tag snapshot.

### R6 — Tag and publication

- **R6.1** Create an annotated tag `v1.1.2` on the green release-evidence
  commit with the message `Clorraine v1.1.2`.
- **R6.2** Push `main` and the tag only after every gate is green **and** the
  release build and archive audit have passed. The tag stays local until
  then.
- **R6.3** Create the GitHub release as a **draft** with `RELEASE-NOTES.md` as
  the body (`--notes-file`), upload both tarballs and `SHA256SUMS` (three
  assets), verify `isDraft` and the asset names/sizes/states with
  `gh release view v1.1.2 --json isDraft,assets`, then publish with
  `gh release edit v1.1.2 --draft=false`.
- **R6.4** Confirm the published (non-draft) release page lists the notes and
  all three checksummed assets; record the URL.
- **R6.5** If publication fails, record the exact blocker in `RELEASE.md`; a
  partial upload is retried with `gh release upload`; never replace an
  existing asset or delete the release without a recorded user ruling.
- **R6.6** If `v1.1.2` exists locally but not on `origin`, delete and recreate
  it after confirming the remote tag is absent. If it exists on `origin`,
  stop; never force-move a published tag.

### R7 — Records and closeout

- **R7.1** The plan folder records the decisions, design, testing strategy,
  findings, and gate evidence; `05-findings.md` is the evidence ledger and
  `RELEASE.md` the summary/publication record.
- **R7.2** The roadmap row for PLAN-27 moves through the stages and closes as
  Done; the tracker reads **571/573 rows done, 0 open, 2 parked**, and the
  roadmap-totals note is updated to match.
- **R7.3** `RELEASE.md` records the final publication status.
- **R7.4** `codeops/specialist-check.json` is refreshed with 74 plan folders
  and `AGENTS.md` records the same coverage date/count.
- **R7.5** The tree is clean and `main` equals `origin/main` at closeout.
- **R7.6** The correctness review finds no open critical/major issue; every
  material claim is checked against the evidence.

## Non-Functional Requirements

- **N1 — No new behavior:** the closeout diff contains no compiler, stdlib,
  runtime, spec, or CLI behavior change; the shipped changes are the
  already-reviewed commits since v1.1.1.
- **N2 — Safety:** no runtime check, sanitizer leg, fuzz leg, or fault probe is
  removed or weakened.
- **N3 — Reproducibility:** the artifact build uses the pinned Docker image and
  the recorded tag; checksums are recorded.
- **N4 — Dependencies:** no new crate, npm package, script framework, or
  service. Packaging uses the existing toolchain and shell utilities.
- **N5 — Integrity:** the published tag is immutable; artifacts are built from
  the tag, not from a dirty tree.
- **N6 — Licensing:** the license and third-party notices are unchanged; the
  binary tarball now includes `THIRD_PARTY_NOTICES.md` and
  `docs/known-limitations.md` so the licensing references in the shipped
  README and notes resolve. The other documentation links in the README stay
  source-tree/site links.
- **N7 — Truthfulness:** every version, size, instruction, and gate claim is
  backed by a recorded run on the release tree.

## Security Considerations

- **Data sensitivity:** N/A — no user data, credentials, or PII.
- **Input validation:** N/A — no new input surface.
- **Authentication/authorization:** N/A — no service or role surface.
- **Injection risks:** N/A — packaging uses fixed argument lists and existing
  paths; no new shell interpolation of untrusted input.
- **Encryption:** N/A — no data at rest or in transit beyond GitHub's HTTPS.
- **Rate limiting:** N/A — no endpoint.
- **Supply chain:** the artifact contains only repository-built binaries and
  allowlisted documentation; the archive audit excludes NDK, ROM, Workbench,
  and local environment content. SHA-256 sums are published.
- **Security testing:** the full ASan/UBSan, fuzz, fault-probe, and acceptance
  gates stay green.

## Acceptance Summary

| Gate | Pass condition |
|---|---|
| Requirements preflight | No open critical/major finding |
| Plan preflight | No open critical/major finding |
| Version surfaces | `1.1.2` in Cargo, lock, extension, README; both `--version` lines exact; driver binaries not stripped |
| Changelog/notes | One 1.1.2 entry; three measurement bases kept separate; claims backed by the release-tree evidence |
| Runbook | v1.1.2 decision, gates, artifacts, gated/tagged hashes, publication record |
| Identity | `base.json` = `v1.1.1`, `head.json` = gated commit, both clean; `flags.json` clean with `runs: 2` |
| Full gate | `check`, `asan`, `ubsan`, `corpus-amiga`, `differential`, `bench-shape`, `bench-base`, `bench`, `bench-compare`, `bench-flags`, `verify`, `vscode-check` green |
| Scored comparison | No regression above 1%; `lookup` 20% floors met |
| Artifacts | Two tarballs + `SHA256SUMS`; notices/limitations included; archive audit clean; hashes recorded |
| Tag | Annotated `v1.1.2` on the green evidence commit; diff from gated commit docs-only |
| Publication | Draft release verified, then published; notes body and three assets |
| Records | Plan Done; roadmap 571/573; specialist check 74; tree clean; `main == origin/main` |
