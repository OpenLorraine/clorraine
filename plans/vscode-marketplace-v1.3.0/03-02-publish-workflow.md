# Publish Workflow: VS Code Marketplace Extension and the v1.3.0 Release

> **Document**: 03-02-publish-workflow.md
> **Parent**: [Index](00-index.md)

## Overview

A manually dispatched GitHub Actions workflow builds the extension from the dispatched ref, checks
its version, audits it, and — only from a matching release tag with `dry_run` unticked — publishes
it to the Marketplace with Trusted Publishing (AR-5, AR-11, AR-12, AR-13, AR-29, AR-30, AR-41).

## Architecture

### Current Architecture

No publish workflow exists. `docs.yml` is the precedent for a build/deploy split with pinned
actions and a deploy-only `id-token` (`docs.yml:31-66`).

### Proposed Changes

**File:** `.github/workflows/publish-vscode.yml` (name fixed by the Trusted Publishing policy,
AR-22).

```yaml
name: publish-vscode
on:
  workflow_dispatch:
    inputs:
      dry_run:
        description: "Build and audit only; untick to publish (tag dispatch only)"
        type: boolean
        default: true
permissions:
  contents: read
concurrency:
  group: publish-vscode
  cancel-in-progress: false
jobs:
  build:
    runs-on: ubuntu-24.04
    timeout-minutes: 20
    outputs:
      sha256: ${{ steps.hash.outputs.sha256 }}
      version: ${{ steps.version.outputs.version }}
    steps:
      - checkout (pinned as in docs.yml), persist-credentials: false
      - setup-node (pinned as in docs.yml), node-version 22, no cache
      - version guard: node tools/vscode/scripts/release-version.mjs --ref "$REF" \
          --cargo compiler/Cargo.toml --manifest tools/vscode/package.json   # env: REF: ${{ github.ref }}
      - make vscode-package
      - python3 tools/vscode/scripts/audit_vsix.py build/vscode/clorraine-$VERSION.vsix
      - version guard again on the staged manifest (build/vscode/extension/package.json)
      - sha256sum → step output; write version, SHA-256, ref, and run ID to $GITHUB_STEP_SUMMARY
      - upload-artifact (pinned by SHA): name vsix, the one .vsix file, retention 90 days
  publish:
    needs: build
    if: ${{ !inputs.dry_run && startsWith(github.ref, 'refs/tags/v') }}
    runs-on: ubuntu-24.04
    timeout-minutes: 15
    environment: marketplace
    permissions:
      contents: read
      id-token: write
    steps:
      - checkout (pinned), persist-credentials: false
      - setup-node (pinned), node 22, no cache
      - download-artifact (pinned by SHA): vsix
      - verify: sha256sum of the downloaded file == needs.build.outputs.sha256 (via env:), else fail
      - version guard with --publish (refuses a non-tag ref) on the checked-out sources
      - (cd tools/vscode && npm ci --ignore-scripts)
      - tools/vscode/node_modules/.bin/vsce publish --packagePath "$VSIX" --oidc
      - append "published openlorraine.clorraine <version>" to $GITHUB_STEP_SUMMARY
```

Rules:

- Every `uses:` is pinned to a 40-hex commit SHA with a version comment; `checkout` and
  `setup-node` reuse the `docs.yml` pins; `upload-artifact` and `download-artifact` are pinned
  to current v4 release SHAs looked up at implementation time.
- No `${{ … }}` expression appears inside a `run:` script; refs, versions, and hashes reach
  scripts through `env:` (AR-29).
- No `cache:` on `setup-node`; no `actions/cache` (AR-30).
- `--skip-duplicate` is not used: re-publishing an existing version fails, and recovery is a
  manual listing check (AR-30).
- The `marketplace` environment's `v*` tag rule independently blocks the publish job on a
  branch ref (AR-12, AR-22).

### Version guard — `tools/vscode/scripts/release-version.mjs` (AR-42)

```
node release-version.mjs --ref <git ref> --cargo <Cargo.toml> --manifest <package.json> [--publish]
```

- Reads `version = "x.y.z"` from the Cargo manifest's `[package]` section and `"version"` from
  the JSON manifest.
- Cargo version ≠ manifest version → exit 1:
  `release-version: compiler version <c> does not match extension manifest <m>`.
  The source `tools/vscode/package.json` is kept equal to Cargo by the release bump; the staged
  manifest always equals Cargo (`stage.mjs`).
- `refs/tags/<name>`: `<name>` must equal `v<cargo>` exactly, else exit 1:
  `release-version: tag <name> does not match version <c>` (this refuses `-rN` tags).
- Any other ref: allowed without `--publish` (dry runs from `main`); with `--publish` exit 1:
  `release-version: publishing requires a release tag, got <ref>`.
- Missing file or unreadable version → exit 2 with the path.
- Success prints `release-version: <version>` and, when `GITHUB_OUTPUT` is set, appends
  `version=<version>`.

## Integration Points

- Consumes the extension package (03-1) through `make vscode-package` and `audit_vsix.py`.
- Phase 1 lands this file on `main` early so dry runs can be dispatched (AR-40); the real
  publish is dispatched from tag `v1.3.0` during the release (03-5).
- Dispatch is the approval: the executing agent asks the user before every dispatch with
  `dry_run=false` (AR-12).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Tag ≠ Cargo ≠ manifest | Build job fails before packaging; publish job fails again before `vsce` | AR-29 |
| Dispatch from a branch with `dry_run=false` | Publish job skipped by `if:`; environment rule blocks it; guard refuses | AR-12, AR-13 |
| Downloaded VSIX hash differs | Publish job fails before `npm ci` | AR-30 |
| Trusted Publishing exchange fails | Job fails; nothing published; fix the policy and re-dispatch | AR-5 |
| Version already published | `vsce` fails; check the listing manually | AR-30 |

## Testing Requirements

- Spec: `tools/vscode/test/publish-workflow.spec.test.ts` (static checks of the workflow text) and
  `tools/vscode/test/release-version.spec.test.ts` (guard behaviour) — ST-1..ST-10.
- Impl: `tools/vscode/test/release-version.impl.test.ts` (malformed inputs, `GITHUB_OUTPUT`).
- Live: one dry run from `main` (Phase 1) and one from the release tag before the real publish
  (Phase 7).
