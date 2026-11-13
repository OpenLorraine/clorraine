# Release Runbook: Clorraine v1.1.2

This document is the release runbook for Clorraine v1.1.2. It records the
approved release decision, the licence and metadata audit, the verification
gates, the artifact build, and the publication sequence. Publication is
authorized by the user once every gate is green; the local release can be
complete while the tag and artifacts remain unpublished.

## Release decision

| Item | Decision |
| ---- | -------- |
| Version | `1.1.2` |
| Release date | 2026-11-11 |
| Licence | MIT |
| Source | Annotated Git tag `v1.1.2` on `main` |
| Binary artefact | Linux x86_64 tarball |
| Integrity | `SHA256SUMS` |
| Release notes | `RELEASE-NOTES.md` and one `CHANGELOG.md` entry |
| Specification | Frozen v1.0 chapters plus the approved v1.1 amendments; the two GCC release rows in §13 were updated by the separate PLAN-26 ruling |
| aarch64 artefact | Deferred; aarch64 host builds remain supported |

> **License change after publication (2026-11-13).** v1.1.2 was published
> under the MIT License, as recorded in this runbook. The repository now uses
> the Apache License 2.0 with the Clorraine Output Exception for future
> releases. The published v1.1.2 artifacts are unchanged and remain available
> under MIT. Future release artifacts should include `LICENSE`,
> `LICENSE-EXCEPTION.md`, `NOTICE`, and `TRADEMARKS.md`.

The version of record is set in `compiler/Cargo.toml` and mirrored in
`Cargo.lock`. The workspace extension version in `tools/vscode/package.json`
and its lock are bumped with the release. The release is manual; release
automation is out of scope.

**Publication status (2026-11-11):** **published** as
[v1.1.2](https://github.com/OpenLorraine/clorraine/releases/tag/v1.1.2). The
annotated tag `v1.1.2` (tag object `5beec93`, commit `ed439ae`) points at the
evidence commit; the source tarball, Linux x86_64 tarball, `SHA256SUMS`, and
the release notes are published.

## Pre-release gates

Gated commit: `74d4380a540bf609f46f005c882dd4ee2c06f371` (`C1`). The evidence
commit is the tag commit; its diff from the gated commit is documentation and
plan records only. `make` targets run on the host and enter the pinned
container internally; `make vscode-check` needs Node 22 on `PATH`, and `gh`
runs on the host.

- [x] `make check` — specification check, registry, setup-check, Rust
      workspace suites, benchmark harness tests, fuzzer tests, and the fuzzer
      smoke matrix
- [x] `make asan` — every suite that references `RDC_ASAN`, one bounded process
      per suite
- [x] `make ubsan` — the UBSan leg with the active-leg overflow probe
- [x] `make corpus-amiga` — portable corpus on hosted-amiga plus the vbcc build
      and the FS-UAE-only programs
- [x] `make differential` — host and hosted-amiga output match for the three
      differential programs
- [x] `make bench-shape` — generated-C shape fixture with m68k GCC, vbcc, and
      the freestanding profile
- [x] `make bench-base COMMIT=v1.1.1` / `make bench` /
      `make bench-compare REQUIRE="lookup:instructions:20 lookup:text:20"` —
      the release-tree comparison; both lookup floors pass, no scored
      regression
- [x] `make bench-flags` — same-compiler old/GC/shipped audit; `runs: 2`;
      floors, constructors, and repeat pass
- [x] `make verify` — all acceptance legs; must print `verify: all legs passed`
- [x] `make vscode-check` — the extension type-check, tests, package audit,
      and packaged-client smoke

Gate evidence for the gated commit:

| Gate | Result |
| ---- | ------ |
| `make check` | exit 0; `spec check: OK (27 files, 114 compile-fail tests)`; fuzzer smoke `cases=28 host=28 sanitizer=16 target=16 failures=0` |
| `make asan` | 30/30 suites passed |
| `make ubsan` | 4/4 suites passed; the active-leg sanitizer check passed |
| `make corpus-amiga` | all portable programs match; vbcc builds are warning-free; FS-UAE-only programs built |
| `make differential` | 3/3 programs match |
| `make bench-shape` | m68k GCC, GCC nix13, vbcc, and freestanding shapes are warning-free |
| `make bench-base` / `make bench` / `make bench-compare` | base v1.1.1 (`2d1eb30`, `dirty: false`); head `74d4380` (`dirty: false`); lookup 13,653 → 9,470 instructions (−30.6%), `.text` 64,328 → 39,904 (−38.0%); both requirements PASS; other scored benchmarks unchanged |
| `make bench-flags` | `runs: 2`; base/head/flags identities clean; five scored benchmarks `.text` 57.1–69.3%, binaries 64.2–74.7%; host binaries 20.6–36.5%; 1.3 audit set 49.1–88.9%; calibration 3,372 → 2,928 instructions (2.0) / 3,348 → 2,947 (1.3); constructors and repeat pass |
| `make verify` | exit 0; `verify: all legs passed` |
| `make vscode-check` | exit 0; extension type-check, tests, `.vsix` package audit, and packaged-client smoke passed (Node 22.23.3) |

## Licence and metadata audit (v1.1.2, as published)

- [x] `LICENSE` contains the MIT text with the OpenLorraine copyright line
- [x] `compiler/Cargo.toml` declares `version = "1.1.2"`, `license = "MIT"`, a
      description, and the repository URL
- [x] `Cargo.lock` records `clc` version `1.1.2`
- [x] `tools/vscode/package.json` and its lock record `1.1.2`
- [x] `tools/demo/package.json` keeps its own package version and declares
      `"license": "MIT"`
- [x] `tools/demo/package-lock.json` records `"license": "MIT"` on the root
      package
- [x] `README.md` names MIT, links `LICENSE`, and names v1.1.2 as the current
      release
- [x] `THIRD_PARTY_NOTICES.md` preserves the upstream Hamurabi MIT notice
- [x] `docs/known-limitations.md` exists, is linked from `README.md` and
      `spec/README.md`, and is included in the binary tarball

| File | Expected | Observed |
| ---- | -------- | -------- |
| `LICENSE` | MIT text, OpenLorraine copyright | present |
| `compiler/Cargo.toml` | `1.1.2`, `MIT`, description, repository | present |
| `Cargo.lock` | `clc 1.1.2` | present |
| `tools/vscode/package.json` / lock | `1.1.2`, `MIT` | present |
| `tools/demo/package.json` / lock | own version, `MIT` | present |
| `README.md` | MIT section, `LICENSE`, v1.1.2 current release | present |
| `THIRD_PARTY_NOTICES.md` | upstream Hamurabi MIT text | present |
| `spec/README.md` | known-limitations link | present |

## Build and artefact packaging

Run from the repository root in the pinned SDK container; `gh` runs on the
host.

1. Create the annotated tag on the evidence commit and assert it:
   `git tag -a v1.1.2 -m "Clorraine v1.1.2"`, `git cat-file -t v1.1.2`,
   `git rev-parse v1.1.2^{commit}`.
2. Assert a clean tree and `HEAD == v1.1.2^{commit}`.
3. `cargo build --release --workspace`
4. Assert `target/release/clc --version` prints exactly `clc 1.1.2`.
5. Assert `target/release/clorraine --version` prints exactly
   `clorraine 1.1.2`.
6. Confirm both binaries are Linux x86_64 ELF files and are not stripped.
7. Compile and run `examples/hello/hello.clo` with both release binaries:
   `target/release/clc --profile host -o build/release/hello.c examples/hello/hello.clo`,
   `cc -std=c99 -O2 -o build/release/hello build/release/hello.c`,
   `build/release/hello`; and
   `target/release/clorraine build --release --profile host examples/hello/hello.clo`,
   `build/clorraine/hello/host/hello`. Both print
   `Hello, Amiga, from Clorraine!`.

Artefacts are written under `build/release/v1.1.2/` and stay gitignored:

| Artefact | Contents |
| -------- | -------- |
| `clorraine-1.1.2-src.tar.gz` | Source tree from the annotated tag, prefix `clorraine-1.1.2/` |
| `clorraine-1.1.2-linux-x86_64.tar.gz` | `clc`, `clorraine`, `LICENSE`, `README.md`, `RELEASE-NOTES.md`, `CHANGELOG.md`, `THIRD_PARTY_NOTICES.md`, and `docs/known-limitations.md` |
| `SHA256SUMS` | SHA-256 for both tarballs |

The source archive is the tag snapshot. Its runbook records the
pre-publication state; the publication record is added to the repository and
the release page after publication.

Before publication, verify that neither archive contains NDK content, ROMs,
Workbench media, `build/` output, `target/` output, `docker/.env`, or
`docker/compose.override.yaml`. The committed `docker/.env.example` and
`docker/compose.override.yaml.example` are allowed.

## Conditional publication

These steps run only after the user authorizes publication; the authorization
for v1.1.2 is recorded in the release decision.

- [x] Push `main` and the annotated tag `v1.1.2` after the archive audit
- [x] Create the release as a draft:
      `gh release create v1.1.2 --draft --title "Clorraine v1.1.2" --notes-file RELEASE-NOTES.md build/release/v1.1.2/clorraine-1.1.2-src.tar.gz build/release/v1.1.2/clorraine-1.1.2-linux-x86_64.tar.gz build/release/v1.1.2/SHA256SUMS`
- [x] Verify `gh release view v1.1.2 --json isDraft,assets`: `isDraft: true`
      and exactly three assets with the expected names and sizes
- [x] Publish: `gh release edit v1.1.2 --draft=false`
- [x] Confirm the published page lists the notes and all three checksummed
      assets; the URL and asset table are below

Published assets:

| Asset | Size (bytes) | SHA-256 |
| ----- | -----------: | ------- |
| `clorraine-1.1.2-src.tar.gz` | 5,135,518 | `44337730938d6dfa767370d325380aeae337fb350c4f6e476d286a6141139194` |
| `clorraine-1.1.2-linux-x86_64.tar.gz` | 4,489,640 | `52f9e36461e27d85f06bdd7242f4ab0a4d3fe9ff07aa5eccabde9d9d33d38fbe` |
| `SHA256SUMS` | 195 | covers both tarballs |

If publication is blocked, the tag and artifacts stay unpublished, the exact
names and SHA-256 values are reported, and the blocker is recorded here.

## Historical publication: v1.1.1

The v1.1.1 release was published on 2026-11-09 as
[v1.1.1](https://github.com/OpenLorraine/clorraine/releases/tag/v1.1.1). Its
assets were `clorraine-1.1.1-src.tar.gz`,
`clorraine-1.1.1-linux-x86_64.tar.gz`, and `SHA256SUMS`, with
`RELEASE-NOTES.md` and one `CHANGELOG.md` entry. The binary tarball contained
six files and omitted both `THIRD_PARTY_NOTICES.md` and
`docs/known-limitations.md`, which its README and notes linked; v1.1.2 fixes
that packaging gap. The published assets were:

| Asset | Size (bytes) | SHA-256 |
| ----- | -----------: | ------- |
| `clorraine-1.1.1-src.tar.gz` | 4,963,679 | `a292933d163f317a7f95bc1a2ad6b163621ac9a28aa9130382fc5c5bbc9f281e` |
| `clorraine-1.1.1-linux-x86_64.tar.gz` | 4,475,046 | `26847a3ed36f60c587f392fceb062e313973b7f5d287eecefeaef9c13acdd161` |
| `SHA256SUMS` | 195 | covers both tarballs |

## Historical publication: v1.1.0

The v1.1.0 release was published on 2026-11-08 as
[v1.1.0](https://github.com/OpenLorraine/clorraine/releases/tag/v1.1.0). Its
assets were `clorraine-1.1.0-src.tar.gz`,
`clorraine-1.1.0-linux-x86_64.tar.gz`, and `SHA256SUMS`, with
`RELEASE-NOTES.md` and one `CHANGELOG.md` entry.

## Historical publication: v1.0.0

The v1.0.0 release was published on 2026-11-04 as
[v1.0.0](https://github.com/OpenLorraine/clorraine/releases/tag/v1.0.0). Its
assets were `clorraine-1.0.0-src.tar.gz`,
`clorraine-1.0.0-linux-x86_64.tar.gz`, and `SHA256SUMS`, with
`RELEASE-NOTES.md` and one `CHANGELOG.md` entry.

## Failure handling

| Failure | Action |
| ------- | ------ |
| A verification gate fails | An environmental/tooling fix may be applied; a compiler or test failure stops the release and needs a user ruling |
| A flaky environmental leg | One rerun with the failing leg and output recorded; a second failure stops |
| The version does not match | Fix `compiler/Cargo.toml`, regenerate `Cargo.lock`, and rebuild |
| A licence or metadata field is missing | Add it and rerun `./spec/check.sh`, `make check`, and the affected gates |
| A dirty tree before measurement | Clean or commit; restore an unintended RTG capture; investigate a material difference; then rerun |
| `v1.1.2` exists locally only | Delete and recreate after confirming `origin` lacks it |
| `v1.1.2` exists on `origin` | Stop; never force-move a published tag |
| The release build fails after the local tag | Environmental/tooling fix only; recreate the local tag before it is pushed. After the push, rebuild from the same tag and never move it |
| The `bench-base` worktree race | Retry once with the retained `build/perf/base-clc`; the commit identity must be unchanged |
| A partial asset upload | Retry `gh release upload`; never replace an asset or delete the release without a recorded ruling |
| A push is rejected because `origin` advanced | Stop; do not rebase the measured commit |
| An artefact would contain NDK, ROM, or Workbench content | Do not publish; rebuild the artefact from allowlisted sources |
