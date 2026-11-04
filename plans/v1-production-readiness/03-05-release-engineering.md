# Release Engineering: Clorraine v1.0 Production Readiness

> **Document**: 03-05-release-engineering.md
> **Parent**: [Index](00-index.md)

## Overview

The repository has no licence, no release checklist, no tags, and no release artifact. This
component adds the MIT licence and package metadata, records the accepted residual risks, writes
the release checklist, and records the AR-12 decision (v1.0.0, source + Linux x86_64 tarball);
publication is the separate conditional sub-plan. The format and version are fixed by AR-12
(v1.0.0, source tag + Linux x86_64 tarball); only publication can remain blocked, and the main
plan can close production-ready without publishing an artifact.

## Architecture

### Current Architecture

- `README.md` says the licence is "To be determined"; there is no `LICENSE` file.
- `compiler/Cargo.toml` declares the package name, version `0.1.0`, edition, and binaries, but no
  `license`, `description`, or `repository` field. The release version is set from the AR-12
  decision before the artifact is built.
- There are no git tags and no release scripts.
- `plans/00-roadmap.md:599` records the licence decision as a hard blocker for any public
  release.
- The release target is a v1.0.0 source tag plus a Linux x86_64 binary tarball with
  `SHA256SUMS`; aarch64 release artifacts are deferred to PLAN-07 (AR-3, AR-12). Publication is
  the conditional publish sub-plan, not part of the 45-task main plan.

### Proposed Changes

1. **MIT licence.** Add the standard MIT `LICENSE` text with the OpenLorraine copyright line.
   Set `license = "MIT"` in `compiler/Cargo.toml` and `tools/demo/package.json`; add
   `description = "The Clorraine compiler and build/run driver"` and
   `repository = "https://github.com/OpenLorraine/clorraine"`. Update the README licence section
   from "To be determined" to MIT.
2. **Known limitations.** Add `docs/known-limitations.md` with the accepted residuals and the ASan
   build-environment requirement:
   - the SDK image's GCC 12 ASan runtime needs a non-PIE (`-no-pie`) link for its child programs;
     `make asan` applies the flag through a `cc` wrapper, so no open product residual remains;
   - the top-level-`while`-only loop split;
   - the host-only CI limitation (m68k tests skip without the cross toolchain).
   The multi-element view-array leak is in the AR-4 fix list and is not an accepted residual
   unless the fix is explicitly descoped. There is no open-ended catch-all: any new accepted
   residual must be added to this list and AR-14 before release, and the release checklist fails
   if an unlisted residual is found. `03-05` is the sole owner of this document; `03-01` supplies
   the verified technical facts.
3. **Release checklist.** Add `RELEASE.md` with:
   - the AR-12 decision: v1.0.0, source tag + Linux x86_64 binary tarball, `SHA256SUMS`;
   - the `make check`, `make verify`, `make asan`, and `make differential` evidence;
   - the MIT licence and metadata check, including a manifest audit of `compiler/Cargo.toml`,
     `Cargo.lock`, `tools/demo/package.json`, and `tools/demo/package-lock.json` (both manifests
     MIT; `Cargo.lock` records version 1.0.0 and the demo lockfile root records MIT);
   - the source tag, x86_64 build, and smoke-test steps in the conditional publish sub-plan;
   - checksums, release notes, a `CHANGELOG.md` entry, and the no-NDK-redistribution rule.
4. **Release notes and changelog.** In the conditional publish sub-plan, add a concise v1.0
   release note describing the supported profiles, the v1.0 boundary, the known limitations, and
   the licence, plus one `CHANGELOG.md` entry.

The release workflow is manual. Automation stays out of scope per AR-2; the first release is
small enough to prepare by hand, and a workflow can be added after the first tag.

## Implementation Details

### New Types/Interfaces

None.

### New Functions/Methods

None.

### Integration Points

- `LICENSE` and `compiler/Cargo.toml` are independent of publication; the package `version` is set
  to v1.0.0 in Phase 6 task 6.1.2 and verified in conditional task C.1.1.
- `RELEASE.md` records the AR-12 decision; the artifact steps are the conditional publish
  sub-plan.
- `docs/known-limitations.md` is linked from the root `README.md` and `spec/README.md` here in
  Phase 6 after the document exists, then `./spec/check.sh` is rerun.
- The final production gate checks the licence, metadata, checklist, release notes, changelog, and
  the AR-12 decision; the conditional publish sub-plan is complete or recorded as blocked.

## Code Examples

### Cargo metadata

```toml
[package]
name = "clc"
version = "1.0.0"   # set in Phase 6 task 6.1.2; verified in C.1.1
edition = "2021"
publish = false
license = "MIT"
description = "The Clorraine compiler and build/run driver"
repository = "https://github.com/OpenLorraine/clorraine"
```

### Release checklist shape

```markdown
## v1.0 release checklist

- [ ] AR-12 recorded: v1.0.0, source + x86_64 tarball; Cargo `version` matches
- [ ] `make check` green
- [ ] `make verify` all legs passed
- [ ] `make asan` green (serial suites)
- [ ] `make differential` green
- [ ] MIT LICENSE, Cargo metadata, and demo manifest/lockfile MIT present
- [ ] `docs/known-limitations.md` current and linked from `README.md` and `spec/README.md`
- [ ] source tag created
- [ ] Linux x86_64 binary built and smoke-tested; exact `clc 1.0.0`; aarch64 deferred to PLAN-07
- [ ] SHA256SUMS, release notes, and `CHANGELOG.md` entry attached
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Conditional publish sub-plan cannot run | Do not publish; record the blocker; the 45-task main plan still closes production-ready | AR-12 |
| Missing licence metadata | Fail the release checklist, including the manifest audit | AR-5 |
| `--version` does not report `clc 1.0.0` | Fail the smoke test; set the Cargo version before building | AR-12 |
| A host binary fails its smoke test | Do not publish; file the defect and rerun the gate | AR-3 |
| An aarch64 artifact is requested | Record the PLAN-07 deferral; do not add an unapproved cross-build surface | AR-3 |
| A known limitation changes | Update `docs/known-limitations.md` before publishing | AR-14 |
| The release would redistribute NDK content | Stop; the NDK is fetched or detected, never redistributed | AR-3 |
| A release note claims more than v1.0 | Correct it against the roadmap's scope decision | AR-1 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity
> Register entry that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- ST-11 – ST-14 in `07-testing-strategy.md`.
- `LICENSE` exists and contains the MIT text; `compiler/Cargo.toml`, `Cargo.lock`,
  `tools/demo/package.json`, and `tools/demo/package-lock.json` carry or record the MIT metadata.
- `RELEASE.md` contains the AR-12 decision (v1.0.0, x86_64) and the four verification commands.
- `docs/known-limitations.md` lists the accepted residuals, the ASan `-no-pie` build-environment
  requirement, and the host-only CI limitation, and both the `README.md` and `spec/README.md` links
  resolve.
- `RELEASE-NOTES.md` and a `CHANGELOG.md` entry exist in the conditional publish sub-plan.
- In the conditional publish sub-plan: build the x86_64 binary, run `--version` and assert
  the exact output `clc 1.0.0`, run a hello-world compile, and attach checksums.
