# Release Runbook: Clorraine v1.1.1

This document is the release runbook for Clorraine v1.1.1. It records the
approved release decision, the licence and metadata audit, the verification
gates, the artefact build, and the conditional publication steps. Publication
is a separate, explicitly authorized step: the local release can be complete
while the tag and artefacts remain unpublished.

## Release decision

| Item | Decision |
| ---- | -------- |
| Version | `1.1.1` |
| Release date | 2026-11-09 |
| Licence | MIT |
| Source | Annotated Git tag `v1.1.1` on `main` |
| Binary artefact | Linux x86_64 tarball |
| Integrity | `SHA256SUMS` |
| Release notes | `RELEASE-NOTES.md` and one `CHANGELOG.md` entry |
| Specification | Frozen v1.0 chapters plus the approved v1.1 amendments |
| aarch64 artefact | Deferred; aarch64 host builds remain supported |

The version of record is set in `compiler/Cargo.toml` and mirrored in
`Cargo.lock`. The workspace extension version in `tools/vscode/package.json`
and its lock are bumped with the release. The release is manual; release
automation is out of scope.

**Publication status (2026-11-09):** the local release is prepared, the
annotated `v1.1.1` tag is created locally, and the artefacts are checksummed.
Publication awaits explicit user authorization.

## Pre-release gates

- [x] `make check` — specification check, Rust workspace suites, benchmark
      harness tests, fuzzer tests, and the fuzzer smoke matrix
- [x] `make asan` — every suite that references `RDC_ASAN`, one bounded process
      per suite
- [x] `make ubsan` — the UBSan leg with the active-leg overflow probe
- [x] `make corpus-amiga` — portable corpus on hosted-amiga plus the vbcc build
      and the FS-UAE-only programs
- [x] `make differential` — host and hosted-amiga output match for the three
      differential programs
- [x] `make bench-shape` — generated-C shape fixture with m68k GCC, vbcc, and
      the freestanding profile
- [x] `make bench-base COMMIT=v1.1.0` / `make bench` / `make bench-compare
      REQUIRE="matrix:instructions:5 matrix:text:3"` — the matrix win against
      the previous release, no scored regression
- [x] `make verify` — all acceptance legs; must print `verify: all legs passed`
- [x] `make vscode-check` — the extension type-check, tests, package audit,
      and packaged-client smoke

Gate evidence for the release tree:

| Gate | Result |
| ---- | ------ |
| `make check` | exit 0; `spec check: OK (27 files, 114 compile-fail tests)`; 128 workspace test groups; fuzzer smoke `cases=28 host=28 sanitizer=16 target=16 failures=0` |
| `make asan` | 29/29 suites passed |
| `make ubsan` | 3/3 suites passed; the active-leg sanitizer check passed |
| `make corpus-amiga` | 4/4 portable programs match; vbcc builds are warning-free; FS-UAE-only programs built |
| `make differential` | 3/3 programs match |
| `make bench-shape` | m68k GCC, GCC nix13, vbcc, and freestanding shapes are warning-free |
| `make bench-base COMMIT=v1.1.0` / `make bench` / `make bench-compare` | base 1.1.0 (c0bd6d0) matrix 10,959 instructions / 80,712 `.text`; head 1.1.1 (2bc279e) 9,908 / 76,676 (-9.6% / -5.0%); both requirements PASS; other scored benchmarks unchanged |
| `make verify` | exit 0; `verify: all legs passed` |
| `make vscode-check` | exit 0; extension type-check, tests, `.vsix` package audit, and packaged-client smoke passed (Node 22.23.3 on PATH) |

## Licence and metadata audit

- [x] `LICENSE` contains the MIT text with the OpenLorraine copyright line
- [x] `compiler/Cargo.toml` declares `version = "1.1.1"`, `license = "MIT"`, a
      description, and the repository URL
- [x] `Cargo.lock` records `clc` version `1.1.1`
- [x] `tools/vscode/package.json` and its lock record `1.1.1`
- [x] `tools/demo/package.json` keeps its own package version and declares
      `"license": "MIT"`
- [x] `tools/demo/package-lock.json` records `"license": "MIT"` on the root
      package
- [x] `README.md` names MIT, links `LICENSE`, and names v1.1.1 as the current
      release
- [x] `THIRD_PARTY_NOTICES.md` preserves the upstream Hamurabi MIT notice
- [x] `docs/known-limitations.md` exists and is linked from `README.md` and
      `spec/README.md`

| File | Expected | Observed |
| ---- | -------- | -------- |
| `LICENSE` | MIT text, OpenLorraine copyright | present |
| `compiler/Cargo.toml` | `1.1.1`, `MIT`, description, repository | present |
| `Cargo.lock` | `clc 1.1.1` | present |
| `tools/vscode/package.json` / lock | `1.1.1`, `MIT` | present |
| `tools/demo/package.json` / lock | own version, `MIT` | present |
| `README.md` | MIT section, `LICENSE`, v1.1.1 current release | present |
| `THIRD_PARTY_NOTICES.md` | upstream Hamurabi MIT text | present |
| `spec/README.md` | known-limitations link | present |

## Build and artefact packaging

Run the build inside the pinned SDK container:

1. `cargo build --release --workspace`
2. Assert `target/release/clc --version` prints exactly `clc 1.1.1`.
3. Assert `target/release/clorraine --version` prints exactly
   `clorraine 1.1.1`.
4. Compile and run a hello-world program with the release `clc` and
   `clorraine`.
5. Confirm both binaries are Linux x86_64 ELF files.

Artefacts are written under `build/release/v1.1.1/` and stay gitignored:

| Artefact | Contents |
| -------- | -------- |
| `clorraine-1.1.1-src.tar.gz` | Source tree from the annotated tag, prefix `clorraine-1.1.1/` |
| `clorraine-1.1.1-linux-x86_64.tar.gz` | `clc`, `clorraine`, `LICENSE`, `README.md`, `RELEASE-NOTES.md`, and `CHANGELOG.md` |
| `SHA256SUMS` | SHA-256 for both tarballs |

Before publication, verify that neither archive contains NDK content, ROMs,
Workbench media, `build/` output, `target/` output, or a local environment file.

## Conditional publication

These steps run only after the user explicitly authorizes publication.

- [ ] Push the annotated tag: `git push origin v1.1.1`
- [ ] Create the release:
      `gh release create v1.1.1 --title "Clorraine v1.1.1" --notes-file RELEASE-NOTES.md`
      with the source tarball, Linux x86_64 tarball, and `SHA256SUMS` attached
- [ ] Confirm the release page lists the notes, changelog reference, and all
      three checksummed assets

Published assets:

| Asset | Size (bytes) | SHA-256 |
| ----- | -----------: | ------- |
| `clorraine-1.1.1-src.tar.gz` | TBD | TBD |
| `clorraine-1.1.1-linux-x86_64.tar.gz` | TBD | TBD |
| `SHA256SUMS` | TBD | covers both tarballs |

If publication is not authorized, the tag stays local, the exact artefact
names and SHA-256 values are reported, and the blocker is recorded here. The
local release remains complete in that case; only publication is blocked.

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
| A verification gate fails | Stop; fix the implementation or the environment, then rerun the full gate |
| The version does not match | Fix `compiler/Cargo.toml`, regenerate `Cargo.lock`, and rebuild |
| A licence or metadata field is missing | Add it and rerun `./spec/check.sh` and `make check` |
| Publication is not authorized | Do not push the tag or upload; record the blocker and keep the checklist open |
| An artefact would contain NDK, ROM, or Workbench content | Do not publish; rebuild the artefact from allowlisted sources only |
