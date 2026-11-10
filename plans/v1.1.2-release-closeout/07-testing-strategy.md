# Testing Strategy: v1.1.2 Release Closeout (PLAN-27)

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

This release adds no runtime behavior. The "tests" are release verification
cases: exact version outputs, documented claims checked against recorded
measurements, gate results, artifact contents, tag identity, and publication
state. The existing compiler, sanitizer, corpus, fuzz, and emulator gates are
the behavioral oracle.

No new Rust or Python test is required. The extension tests compare the
packaged `tools/vscode/package.json` to `compiler/Cargo.toml`, covering that
surface; the Cargo/lock/README/changelog surfaces are checked by the release
cases below.

| Area | Verification |
|---|---|
| Version surfaces | exact strings and `--version` output |
| Claims | each number traced to `build/perf/*.json` or a findings ledger |
| Gates | every repository gate on the gated commit |
| Artifacts | file list, exclusions, checksums, ELF/version checks |
| Tag | annotated object, commit, docs-only diff from the gated commit |
| Publication | draft asset list, then published page |

## Release Verification Cases

### Version and docs

| # | Scenario | Expected |
|---|---|---|
| RT-1 | `compiler/Cargo.toml`, `Cargo.lock`, `tools/vscode/package.json`, `package-lock.json` | all record `1.1.2`; demo package unchanged |
| RT-2 | `cargo build --release --workspace`; `target/release/clc --version`, `target/release/clorraine --version` | `clc 1.1.2`, `clorraine 1.1.2` |
| RT-3 | `file target/release/clc target/release/clorraine` | Linux x86_64 ELF, not stripped |
| RT-4 | `README.md` status paragraph | names v1.1.2 as current |
| RT-5 | `CHANGELOG.md` top section | one `[1.1.2]` section with Changed/Verification; no false Fixed |
| RT-6 | `RELEASE-NOTES.md` | v1.1.2; highlights, verification, limits, licence; no runtime overclaim |
| RT-7 | `RELEASE.md` | v1.1.2 decision, gates, artifacts, publication; historical sections preserved |
| RT-8 | `docs/known-limitations.md` §12 | generic LSP packaging sentence; v1.1.1 fix history preserved |

### Measurement and gates

| # | Scenario | Expected |
|---|---|---|
| RT-9 | `build/perf/base.json` / `head.json` / `flags.json` identities | base `v1.1.1`, head/flags at the gated commit, both clean, `runs: 2`, SHA-256 match |
| RT-10 | `make bench-base COMMIT=v1.1.1` + `make bench` + `make bench-compare REQUIRE="lookup:instructions:20 lookup:text:20"` | pass; `lookup` net 13,653 → 9,470, `.text` 64,328 → 39,904; no scored regression above 1% |
| RT-11 | `make bench-flags` | `runs: 2`; floors, constructors, repeat all pass |
| RT-12 | `make check`, `make asan`, `make ubsan` | pass |
| RT-13 | `make corpus-amiga`, `make differential` | pass |
| RT-14 | `make bench-shape`, `make verify` | warning-free; `verify: all legs passed` |
| RT-15 | `make vscode-check` | pass |
| RT-16 | `git status --porcelain` after `make verify` | clean; an unintended RTG capture change restored; a material difference investigated |

### Artifacts

| # | Scenario | Expected |
|---|---|---|
| RT-17 | Source archive | prefix `clorraine-1.1.2/`; no `.git`, `build/`, `target/`, `docker/.env`, `docker/compose.override.yaml`, NDK, ROM, Workbench |
| RT-18 | Binary archive | eight files: `clc`, `clorraine`, `LICENSE`, `README.md`, `RELEASE-NOTES.md`, `CHANGELOG.md`, `THIRD_PARTY_NOTICES.md`, `docs/known-limitations.md` |
| RT-19 | `sha256sum -c SHA256SUMS` | both tarballs OK |
| RT-20 | Hello world with release `clc` and `clorraine` | `clc --profile host -o build/release/hello.c examples/hello/hello.clo`, then `cc` and run; `clorraine build --release --profile host` and run; both print `Hello, Amiga, from Clorraine!` |

### Tag and publication

| # | Scenario | Expected |
|---|---|---|
| RT-21 | `git cat-file -t v1.1.2` and `git rev-parse v1.1.2^{commit}` | `tag`; the evidence commit |
| RT-22 | `git diff --stat <gated>..v1.1.2^{commit}` | documentation and plan files only |
| RT-23 | `gh release view v1.1.2 --json isDraft,assets` before publishing | `isDraft: true`; exactly three assets with correct names/sizes |
| RT-24 | Published (non-draft) release page | notes body and three assets visible |
| RT-25 | `plans/00-roadmap.md` | PLAN-27 Done; 571/573; totals note updated |
| RT-26 | `codeops/specialist-check.json`, `AGENTS.md` | 74 plan folders and the matching date |
| RT-27 | Final `git status -sb` and `git log` | clean; `main` equals `origin/main`; tag pushed |
| RT-28 | Claim traceability | every number in the notes/changelog appears in `05-findings.md` with its source |

## Red/Green

There is no new unit test to drive red/green. The release verification is
sequential: RT-1/RT-4..RT-8 are red before the version/docs commits and green
after; RT-2/RT-3 require the C1 release build; RT-9..RT-16 are green only on
the gated commit; RT-17..RT-28 are green only after the tag and publication.
A failure at any step stops the release and is recorded in the findings.

## Test Data

- `build/perf/{base,head,flags}.json` — generated, gitignored evidence.
- `build/release/v1.1.2/` — generated artifacts and checksums.
- The committed `benchmarks/` and `spec/tests/` sources are unchanged by the
  closeout; the release diff adds the `lookup` benchmark and the two §13
  release rows.

## Verification Checklist

- [ ] Every RT case has an exact expected result.
- [ ] The three measurement bases are never mixed.
- [ ] Identity assertions pass before any gate result is recorded.
- [ ] The tag's diff from the gated commit is docs-only.
- [ ] The draft release has exactly three assets before it is published.
- [ ] The final tree is clean and `main == origin/main`.
