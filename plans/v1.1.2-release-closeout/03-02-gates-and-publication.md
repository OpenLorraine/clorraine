# Gates, Artifacts, Publication: v1.1.2 Release Closeout (PLAN-27)

> **Document**: 03-02-gates-and-publication.md
> **Parent**: [Index](00-index.md)

## Overview

This component runs the release acceptance on the gated commit, records the
measurements, packages the artifacts from the annotated tag, and publishes the
GitHub release. It changes no product code.

## Commit and Gate Sequence

```
C0  docs(plans): add the v1.1.2 release plan          (plan folder + roadmap row)
    |
C1  chore(release): bump the version to 1.1.2        (version surfaces + plan progress)
    |  clean tree; full gate; measurement JSON names C1; review
    v
C2  docs(release): record the v1.1.2 gates            (changelog, notes, runbook, findings)
    |  docs-only diff from C1; claim review; final `make check` + version assertions
    v
tag v1.1.2 (annotated, local, on C2)
    |  build release binaries; package and audit artifacts
    v
push main + tag -> draft release -> verify assets -> publish
    |
    v
C3  docs(release): record the v1.1.2 publication     (URL, sizes, hashes, status)
    |  close the plan and roadmap
```

The measurement JSON names C1 because the harness records `HEAD` at run time.
C2 changes no compiler source, so the measurement remains valid; the findings
record both hashes and assert C2's diff from C1 is documentation-only.

## Gate Run

Run from the repository root on a clean C1. The top-level `make` targets
enter the pinned container internally; `make vscode-check` needs Node on
`PATH`, and `gh` runs on the host.

```sh
rm -f build/perf/base.json build/perf/head.json build/perf/flags.json
git status --porcelain            # must be empty
make check
make asan
make ubsan
make corpus-amiga
make differential
make bench-shape
make bench-base COMMIT=v1.1.1
make bench
make bench-compare REQUIRE="lookup:instructions:20 lookup:text:20"
make bench-flags
make verify
make vscode-check
```

After `make verify`, check `git status --porcelain`. The RTG legs regenerate
`docs/amiga-sdk/a1200-rtg.png` and `a4000-rtg-piv.png`; restore an unintended
change before C2. A material capture difference is investigated, not committed
to hide an unexplained rendering change. Do not run `bench-flags` after an
uncommitted regeneration. `make vscode-check` needs Node 22 on `PATH`
(the v1.1.1 runbook used 22.23.3).

### Identity Assertions

| Document | Field | Expected |
|---|---|---|
| `build/perf/base.json` | `compiler.git` | `2d1eb3026fc6f75885785e6ff9601f99aaba4021` |
| `build/perf/base.json` | `compiler.dirty` | `false` |
| `build/perf/head.json` | `compiler.git` | the C1 full hash |
| `build/perf/head.json` | `compiler.dirty` | `false` |
| `build/perf/flags.json` | `clc.git` | the C1 full hash |
| `build/perf/flags.json` | `clc.dirty` | `false` |
| `build/perf/flags.json` | `clc.version` | `clc 1.1.2` |
| `build/perf/flags.json` | `clc.sha256` | equals `head.json.compiler.sha256` |
| `build/perf/flags.json` | `runs` | `2` |
| `build/perf/flags.json` | `checks` | floors, constructors, and repeat all `passed: true` |

`base.json` is built by `docker/scripts/bench-base.sh` from the `v1.1.1` tag
in a temporary worktree that is removed on exit; only `build/perf/base-clc`
is retained. A rerun after a worktree race is
`python3 tools/perf/bench.py run --clc build/perf/base-clc --compiler-git
2d1eb3026fc6f75885785e6ff9601f99aaba4021 --label base --out build/perf/base.json`.
That rerun is valid only if `base-clc` is the v1.1.1 build; `2.1.5` deletes it
so `make bench-base` rebuilds it. Otherwise rerun `make bench-base`.

## Packaging

### Release Build

Inside the pinned container:

```sh
cargo build --release --workspace
target/release/clc --version            # clc 1.1.2
target/release/clorraine --version      # clorraine 1.1.2
file target/release/clc target/release/clorraine   # ELF 64-bit LSB, not stripped
```

Compile and run `examples/hello/hello.clo` with the release `clc` and
`clorraine`; the output is `Hello, Amiga, from Clorraine!`:

```sh
mkdir -p build/release
target/release/clc --profile host -o build/release/hello.c examples/hello/hello.clo
cc -std=c99 -O2 -o build/release/hello build/release/hello.c
build/release/hello
target/release/clorraine build --release --profile host examples/hello/hello.clo
build/clorraine/hello/host/hello
```

### Source Tarball

```sh
rm -rf build/release/v1.1.2
mkdir -p build/release/v1.1.2
git archive --format=tar.gz --prefix=clorraine-1.1.2/ \
    -o build/release/v1.1.2/clorraine-1.1.2-src.tar.gz v1.1.2
```

The archive is the tag snapshot. Its `RELEASE.md` records the pre-publication
state; the publication record is added to the repository and the release page
after publication (AR-17).

### Binary Tarball

```sh
stage=build/release/v1.1.2/stage
rm -rf "$stage"
mkdir -p "$stage/clorraine-1.1.2/docs"
cp target/release/clc target/release/clorraine \
   LICENSE README.md RELEASE-NOTES.md CHANGELOG.md THIRD_PARTY_NOTICES.md \
   "$stage/clorraine-1.1.2/"
cp docs/known-limitations.md "$stage/clorraine-1.1.2/docs/"
tar -C "$stage" -czf build/release/v1.1.2/clorraine-1.1.2-linux-x86_64.tar.gz \
    clorraine-1.1.2
```

The staging tree contains eight files under `clorraine-1.1.2/`; the `clc` and
`clorraine` binaries are Linux x86_64 ELF and not stripped.

### Checksums and Audit

```sh
cd build/release/v1.1.2
sha256sum *.tar.gz > SHA256SUMS
sha256sum -c SHA256SUMS
```

Audit both archives:

- the source archive has the `clorraine-1.1.2/` prefix and no `.git/`,
  `build/`, `target/`, `docker/.env`, `docker/compose.override.yaml`, NDK,
  ROM, or Workbench media;
- the binary archive contains only the eight staged files;
- the committed `docker/.env.example` and
  `docker/compose.override.yaml.example` are allowed in the source archive.

Record the artifact sizes and SHA-256 values in `RELEASE.md` (post-tag commit)
and in `05-findings.md`.

## Tag

Before building, assert the working tree is clean and `HEAD` is the tag commit:

```sh
git status --porcelain
git rev-parse HEAD
git rev-parse v1.1.2^{commit}
```

```sh
git tag -a v1.1.2 -m "Clorraine v1.1.2" <C2>
git cat-file -t v1.1.2          # tag
git rev-parse v1.1.2^{commit}   # C2
```

Create the tag locally after the full gate is green, then build and audit the
artifacts. Push `main` and the tag only after the release build and archive
audit pass. If the local tag exists but `origin` does not, delete and recreate
it after confirming the remote is absent. Never force-move a published tag.

## Publication

```sh
gh release create v1.1.2 --draft --title "Clorraine v1.1.2" \
    --notes-file RELEASE-NOTES.md \
    build/release/v1.1.2/clorraine-1.1.2-src.tar.gz \
    build/release/v1.1.2/clorraine-1.1.2-linux-x86_64.tar.gz \
    build/release/v1.1.2/SHA256SUMS
gh release view v1.1.2 --json isDraft,assets  # draft; three assets, sizes correct
gh release edit v1.1.2 --draft=false
gh release view v1.1.2                 # notes and all three assets visible
```

A partial upload is retried with `gh release upload v1.1.2 <missing-asset>`.
Never replace an existing asset or delete the release without a recorded user
ruling. Record the URL and asset table in the post-tag commit.

## Failure Handling

| Failure | Action |
|---|---|
| A gate fails | An environmental/tooling fix may be applied; a compiler or test failure stops the release and needs a user ruling (AR-11) |
| A flaky environmental leg | One rerun with the failing leg and output recorded; a second failure stops |
| A version mismatch | Fix `compiler/Cargo.toml`, regenerate `Cargo.lock`, rebuild |
| A dirty tree before measurement | Clean or commit, then rerun the measurement |
| `v1.1.2` exists locally only | Delete and recreate after confirming `origin` lacks it |
| `v1.1.2` exists on `origin` | Stop; never force-move a published tag |
| Release build fails after the local tag | Environmental/tooling fix only; recreate the local tag before it is pushed. A compiler/test failure stops for a user ruling. After the push, rebuild from the same tag and never move it |
| `bench-base` worktree race | Retry once with the retained `build/perf/base-clc`; identity must be unchanged |
| Partial asset upload | Retry `gh release upload`; record the failure |
| Existing draft release | Resume with `gh release upload` and `gh release edit` |
| Existing published release | Stop; never overwrite a published release |
| `gh auth`/permission failure | Record the blocker; stop; no anonymous upload |
| Push rejected (`origin` advanced) | Stop; do not rebase the measured commit |
| Forbidden archive content | Do not publish; rebuild from allowlisted sources |
