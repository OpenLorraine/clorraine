# Packaging and Acceptance: VS Code Marketplace Extension and the v1.3.0 Release

> **Document**: 03-04-packaging-acceptance.md
> **Parent**: [Index](00-index.md)

## Overview

The candidate drops the VSIX (four assets plus `SHA256SUMS`), and acceptance proves the
Marketplace extension instead of hashing a candidate VSIX (AR-6, AR-28, AR-34, AR-44).

## Architecture

### Current Architecture

See 02 §What Exists (Packaging, Acceptance).

### Proposed Changes — Packaging

- `Makefile` `dist`: stop calling `vscode-package`, stop requiring Node, stop passing `--vsix`;
  keep the `VERSION` = `tools/vscode/package.json` version check (it keeps the source manifest
  in step with Cargo for the workflow's guard). Comment: "Build a local candidate: archives,
  installers, and checksums."
- `tools/setup/package.sh`: remove `--vsix` (an unknown option is an error, as today); remove the
  VSIX copy, the four VSIX placeholders from both installer assemblies, and the VSIX pin checks in
  `--verify`; `SHA256SUMS` covers exactly the Linux archive, the Windows zip, `install.sh`, and
  `install.ps1`. `--verify` fails when the output directory contains any other `.vsix`, archive,
  or installer file (a stale VSIX from an older candidate must not ride along).
- `make vscode-check` and `make vscode-package` are unchanged except for the 03-1 additions; they
  remain the local way to build and audit the VSIX.

### Proposed Changes — Acceptance evidence

Record schema (both platforms):

```json
"extension": {
  "id": "openlorraine.clorraine",
  "version": "1.3.0",
  "publish_run": "<GitHub Actions run ID, digits>",
  "vsix_sha256": "<64 lowercase hex from the publish run summary>"
},
"manual": { "editor_build_run": true }          // Linux
"manual": { "uac_prompt_seen": true, "editor_build_run": true }  // Windows (unchanged keys)
```

- `acceptance.py`: replace `vsix` with `extension` in the core keys; Linux top keys gain
  `manual` with `{editor_build_run}`; validate `id == "openlorraine.clorraine"`,
  `version ==` the candidate version, `publish_run` matches `^[0-9]+$`, `vsix_sha256` matches the
  64-hex pattern, and `manual.editor_build_run is True`. A record with a `vsix` key or without
  `extension` fails naming the key (AR-44). Error text for a version mismatch:
  `extension version <v> does not match candidate <c>`.
- `acceptance.sh` (`make acceptance`): new required inputs `PUBLISH_RUN` and `PUBLISH_SHA256`
  (Make variables passed through), optional `EDITOR_CONFIRMED=1`. It runs the candidate installer
  with `--vscode` inside the isolated HOME, then reads
  `code --list-extensions --show-versions` with the same HOME and records the
  `openlorraine.clorraine@<version>` line. No line, or a different version, fails the run.
  Without `EDITOR_CONFIRMED=1` the record is written with `editor_build_run: false` and the run
  ends with "acceptance: confirm the language server and Build/Run in VS Code, then rerun with
  EDITOR_CONFIRMED=1" (exit 1), matching the Windows `-EditorConfirmed` pattern.
- `acceptance.ps1`: new mandatory `-PublishRun` and `-PublishSha256`; the installer runs with
  `-VSCode` instead of `-NoVSCode`; the extension line is read the same way through the
  installer's editor-launch logic; `-EditorConfirmed` keeps its meaning.
- The candidate no longer contains a VSIX, so both scripts stop requiring and copying one.
- Precondition (Phase 7): the Marketplace serves `openlorraine.clorraine` at the candidate
  version before acceptance starts (AR-28); checked with the public gallery query.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Extension not installed after the installer | Acceptance fails with the installer summary | AR-28 |
| Installed version ≠ candidate | Acceptance fails; `acceptance-check` also rejects | AR-28 |
| Old record with `vsix` | `acceptance-check` rejects naming the key | AR-44 |
| Stale `.vsix` in the output dir | `dist-verify` fails naming the file | AR-6 |

## Testing Requirements

- Spec: `tools/setup/tests/dist.spec.test.sh`, `windows-dist.spec.test.sh` (four assets, no VSIX
  pins, `--vsix` rejected, stale VSIX rejected), `acceptance_spec.py`,
  `acceptance_windows_spec.py` (extension entry, manual key on Linux, legacy key rejected) —
  ST-29..ST-36.
- Impl: `windows-dist.impl.test.sh`, `acceptance_windows_impl.py`, fixture updates in
  `install_fixtures.sh`.
