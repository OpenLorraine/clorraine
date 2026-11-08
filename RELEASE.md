# Release Runbook: Clorraine v1.1.0

This document is the release runbook for Clorraine v1.1.0. It records the
approved release decision, the licence and metadata audit, the verification
gates, the artifact build, and the conditional publication steps. Publication is
a separate, explicitly authorized step: the local release can be complete while
the tag and artifacts remain unpublished.

## Release decision

| Item | Decision |
| ---- | -------- |
| Version | `1.1.0` |
| Release date | 2026-11-08 |
| Licence | MIT |
| Source | Annotated Git tag `v1.1.0` on `main` |
| Binary artifact | Linux x86_64 tarball |
| Integrity | `SHA256SUMS` |
| Release notes | `RELEASE-NOTES.md` and one `CHANGELOG.md` entry |
| Specification | Frozen v1.0 chapters plus the approved v1.1 amendments |
| aarch64 artifact | Deferred; aarch64 host builds remain supported |

The version of record is set in `compiler/Cargo.toml` and mirrored in
`Cargo.lock`. The release is manual; release automation is out of scope.

**Publication status (2026-11-08):** **published** as
[v1.1.0](https://github.com/OpenLorraine/clorraine/releases/tag/v1.1.0). The
annotated tag, source tarball, Linux x86_64 tarball, `SHA256SUMS`, release
notes, and changelog are attached. The conditional publish steps below are
complete.

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
- [x] `make verify` — all acceptance legs; must print `verify: all legs passed`

Gate evidence for the release tree:

| Gate | Result |
| ---- | ------ |
| `make check` | exit 0; `spec check: OK (27 files, 114 compile-fail tests)`; 121 test groups passed; fuzzer smoke `cases=28 host=28 sanitizer=16 target=16 failures=0` |
| `make asan` | 28/28 suites passed |
| `make ubsan` | 2/2 suites passed; the active-leg sanitizer check passed |
| `make corpus-amiga` | 4/4 portable programs match; vbcc builds are warning-free; FS-UAE-only programs built |
| `make differential` | 3/3 programs match |
| `make bench-shape` | m68k GCC, GCC nix13, vbcc, and freestanding shapes are warning-free |
| `make verify` | exit 0; `verify: all legs passed` |

## Licence and metadata audit

- [x] `LICENSE` contains the MIT text with the OpenLorraine copyright line
- [x] `compiler/Cargo.toml` declares `version = "1.1.0"`, `license = "MIT"`, a
      description, and the repository URL
- [x] `Cargo.lock` records `clc` version `1.1.0`
- [x] `tools/demo/package.json` keeps its own package version and declares
      `"license": "MIT"`
- [x] `tools/demo/package-lock.json` records `"license": "MIT"` on the root
      package
- [x] `README.md` names MIT and links `LICENSE`
- [x] `THIRD_PARTY_NOTICES.md` preserves the upstream Hamurabi MIT notice
- [x] `docs/known-limitations.md` exists and is linked from `README.md` and
      `spec/README.md`

| File | Expected | Observed |
| ---- | -------- | -------- |
| `LICENSE` | MIT text, OpenLorraine copyright | present |
| `compiler/Cargo.toml` | `1.1.0`, `MIT`, description, repository | present |
| `Cargo.lock` | `clc 1.1.0` | present |
| `tools/demo/package.json` | `MIT` | present |
| `tools/demo/package-lock.json` | root package `MIT` | present |
| `README.md` | MIT section, `LICENSE`, and notices link | present |
| `THIRD_PARTY_NOTICES.md` | upstream Hamurabi MIT text | present |
| `spec/README.md` | known-limitations link | present |

## Build and artifact packaging

Run the build inside the pinned SDK container:

1. `cargo build --release --workspace`
2. Assert `target/release/clc --version` prints exactly `clc 1.1.0`.
3. Assert `target/release/clorraine --version` prints exactly
   `clorraine 1.1.0`.
4. Compile and run a hello-world program with the release `clc` and
   `clorraine`.
5. Confirm both binaries are Linux x86_64 ELF files.

Artifacts are written under `build/release/v1.1.0/` and stay gitignored:

| Artifact | Contents |
| -------- | -------- |
| `clorraine-1.1.0-src.tar.gz` | Source tree from the annotated tag, prefix `clorraine-1.1.0/` |
| `clorraine-1.1.0-linux-x86_64.tar.gz` | `clc`, `clorraine`, `LICENSE`, `README.md`, `RELEASE-NOTES.md`, and `CHANGELOG.md` |
| `SHA256SUMS` | SHA-256 for both tarballs |

Before publication, verify that neither archive contains NDK content, ROMs,
Workbench media, `build/` output, `target/` output, or a local environment file.

## Conditional publication

These steps ran after the user explicitly authorized publication on
2026-11-08.

- [x] Push the annotated tag: `git push origin v1.1.0`
- [x] Create the release:
      `gh release create v1.1.0 --title "Clorraine v1.1.0" --notes-file RELEASE-NOTES.md`
      with the source tarball, Linux x86_64 tarball, and `SHA256SUMS` attached
- [x] Confirm the release page lists the notes, changelog reference, and all
      three checksummed assets

Published assets:

| Asset | Size (bytes) | SHA-256 |
| ----- | -----------: | ------- |
| `clorraine-1.1.0-src.tar.gz` | 4,742,187 | `dbae3bc2ad5c57841a722b12dbe9fda3b6c41ab5acf1d0d9bf9209d1ffb4855c` |
| `clorraine-1.1.0-linux-x86_64.tar.gz` | 4,435,694 | `d4a556ff09db28b86b68fbdc708512da6f0e51b506839f81aa1598a3ce662ad5` |
| `SHA256SUMS` | 195 | covers both tarballs |

If publication had not been authorized, the tag would have stayed local, the
exact artifact names and SHA-256 values would be reported, and the blocker
would be recorded here. The local release remains complete in that case; only
publication is blocked.

## Historical publication: v1.0.0

The v1.0.0 release was published on 2026-11-04 as
[v1.0.0](https://github.com/OpenLorraine/clorraine/releases/tag/v1.0.0). Its
assets were `clorraine-1.0.0-src.tar.gz`,
`clorraine-1.0.0-linux-x86_64.tar.gz`, and `SHA256SUMS`, with
`RELEASE-NOTES.md` and one `CHANGELOG.md` entry. It used the MIT licence, the
`v1.0.0` source tag, and the same manual release process.

## Failure handling

| Failure | Action |
| ------- | ------ |
| A verification gate fails | Stop; fix the implementation or the environment, then rerun the full gate |
| The version does not match | Fix `compiler/Cargo.toml`, regenerate `Cargo.lock`, and rebuild |
| A licence or metadata field is missing | Add it and rerun `./spec/check.sh` and `make check` |
| Publication is not authorized | Do not push the tag or upload; record the blocker and keep the checklist open |
| An artifact would contain NDK, ROM, or Workbench content | Do not publish; rebuild the artifact from allowlisted sources only |
