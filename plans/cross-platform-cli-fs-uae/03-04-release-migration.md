# Release and migration

> **Document**: 03-04-release-migration.md
> **Parent**: [Index](00-index.md)
> **AR**: AR-8/13/15/16/22/27/28/30

## Overview

v1.2.0 ships Linux x64 only. The release is built on GitHub-hosted runners,
published as a draft, reviewed, then explicitly promoted. No self-hosted runner
or Docker is in the user or producer path. macOS/Windows follow-up releases use
the same workflow after their evidence.

## v1.2.0 assets

| Asset | Contents |
|---|---|
| `clorraine-1.2.0-linux-x64.tar.gz` | `bin/clc`, `bin/clorraine`, `bin/clorraine-lsp`, licence/notices |
| `SHA256SUMS` | archive + `install.sh` + VSIX hashes |
| `install.sh` | Linux bootstrap |
| `clorraine-1.2.0.vsix` | extension client |

`install.ps1` and macOS/Windows archives are follow-up release assets, not v1.2.0.
No toolchain asset is published (AR-26).

## Candidate staging before acceptance

The release workflow creates a draft. Before acceptance, download the draft
assets into a private directory and install with `install.sh --archive PATH
--checksums PATH` (no network), record the SHA-256 of every asset tested, and
use those exact bytes for ST-34 and the publication approval.

## Release workflow

`.github/workflows/release.yml`, `workflow_dispatch` only:

1. Validate version/commit as literal data.
2. `ubuntu-22.04` job: install Rust 1.99.0, `cargo build --release --workspace`,
   package the Linux archive, build/audit the VSIX, write `SHA256SUMS`.
3. Upload artifacts; a single upload job with `contents: write` creates a
   **draft** release. Build/test jobs are `contents: read`.
4. Do not publish, mark latest or overwrite an existing asset.
5. A separate read-only promotion check downloads the draft, verifies the
   approved hashes/evidence, and only then a manual approval job publishes.

The release workflow never runs on a self-hosted runner or requires Docker.

## Version and docs

- Keep `1.2.0` in `compiler/Cargo.toml`, `Cargo.lock`, extension manifest/lock.
- Rewrite `README.md`, `site/guide/quick-start.md`, `RELEASE.md`,
  `RELEASE-NOTES.md`, `site/status.md`, `site/project/brief.md`,
  `site/guide/known-limitations.md`, `site/guide/roadmap.md` and
  `docs/known-limitations.md` for the new Linux CLI/setup/FS-UAE path.
- Document the manual upstream toolchain download and `--toolchain`; no Python
  installer, support lock or `curl | sh`.
- State Linux-only v1.2.0; macOS/Windows are follow-up releases, not claimed.
- Document unsigned binaries and browser-download warnings.
- Publish as a non-latest prerelease, run the anonymous install, then promote
  latest; never overwrite an existing asset.

## Migration/removal

After the Linux installer and FS-UAE path pass their gates, remove:

| Path | Reason |
|---|---|
| `tools/setup/install_native.py`, `native_archive.py`, `native_state.py` | Python installer |
| `tools/setup/tests/native_*`, `test_native_*` | Old installer tests |
| `tools/distribution/` code/tests/producers | Support pipeline; keep `tools/distribution/inputs/` as referenced licence/origin evidence |
| `.github/workflows/support-release.yml`, `distribution-release.yml` | Old release path |
| Old `RELEASE.md` support/manifest sections | Replaced runbook |

Update `Makefile`: remove `distribution-check` and its `check` dependency; add a
`release-check` target for `tools/release/tests` and wire it into `make check`.
Update `.github/workflows/check.yml` if it references removed paths. Update
`THIRD_PARTY_NOTICES.md` to remove dangling `tools/distribution/inputs` paths or
keep the referenced inputs deliberately.

Keep `tools/setup/install_sdk.py` and the Docker SDK: they remain the
development/verification environment.

## Error handling

| Case | Behavior | AR |
|---|---|---|
| Matrix build/test fails | Stop; no draft | AR-22 |
| Archive/VSIX missing or hash mismatch | Stop; no draft | AR-27 |
| Existing draft with different bytes | Do not overwrite; use a new version | AR-15 |
| Approval hash/evidence mismatch | Do not publish | AR-22 |
| Old path still referenced by Makefile/CI | Removal task fails | AR-16 |

## Testing requirements

- `release-check` is hermetic: workflow structure/archive/removal tests; it is
  wired into `make check` and the host CI workflow.
- `acceptance-check` is explicit and reads candidate/evidence paths; it is not
  part of `make check` and fails when evidence is absent.
