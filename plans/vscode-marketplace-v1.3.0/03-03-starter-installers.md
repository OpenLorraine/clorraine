# Starter and Installers: VS Code Marketplace Extension and the v1.3.0 Release

> **Document**: 03-03-starter-installers.md
> **Parent**: [Index](00-index.md)

## Overview

Make the starter tasks portable through the extension's driver command, and make both installers
install or update the extension from the Marketplace by ID (AR-7, AR-8, AR-14, AR-15, AR-25,
AR-26, AR-27, AR-33, AR-43).

## Architecture

### Current Architecture

See 02 §What Exists (Starter, Installers).

### Proposed Changes — Starter

- `compiler/templates/starter/tasks.json`: both tasks use
  `"command": "${command:clorraine.driverPath}"`; the `@CLORRAINE@` placeholder is removed; args,
  labels, groups, and presentation are unchanged.
- `compiler/src/clorraine/new.rs`: write `TASKS` verbatim; delete `launcher_for` and its helpers
  that only it uses, its re-export in `compiler/src/clorraine/mod.rs:28`, its unit tests in
  `new.rs`, its Windows spec test (`compiler/tests/windows_cli_spec_test.rs:614-625`), and any
  impl test in `windows_cli_impl_test.rs` that calls it (AR-25). The "Run in FS-UAE as the default
  build task (Ctrl+Shift+B)" hint text stays.
- `compiler/templates/starter/README.md`: if it mentions the launcher path, say the tasks need the
  Clorraine extension (recommended by `extensions.json`).
- Existing starter projects are not migrated; their absolute paths keep working.

### Proposed Changes — Installers

Applies to `tools/setup/install.sh` and `tools/setup/install.ps1` alike; names below are the
Linux ones with the Windows equivalent in brackets.

1. **Detection** (AR-14): `code`, `code-insiders` only (`code.cmd`, `code-insiders.cmd`, then the
   default per-user `code.cmd`); `codium` is no longer detected.
2. **Removed**: `VSIX_NAME`, `VSIX_URL`, `VSIX_BYTES`, `VSIX_SHA256` and their placeholders,
   `download_vsix` [the `Invoke-Download` call for the VSIX and the `$script:Vsix*` pins] (AR-6).
3. **Editor calls** go through one bounded helper (AR-33, AR-43):
   - Linux: `timeout --kill-after=5 "$EDITOR_TIMEOUT" "$editor" ARGS…`; exit 124/137 is a timeout.
   - Windows: `Invoke-Native` gains an optional timeout; on expiry it kills the process tree
     (`taskkill /T /F /PID`) and reports a timeout.
   - `EDITOR_TIMEOUT` comes from `CLORRAINE_EDITOR_TIMEOUT` when it is a positive integer, else 300.
4. **Flow** (AR-7, AR-26, AR-27):

   | Situation | Action | State |
   | --------- | ------ | ----- |
   | `--no-vscode` [`-NoVSCode`] | nothing | `skipped` |
   | No editor found | if `--vscode`, warn "VS Code was not found; install VS Code, then run: code --install-extension openlorraine.clorraine" | `unavailable` |
   | Not installed, interactive, no flag | prompt `Install the Clorraine extension in VS Code? [Y/n]`; no → `skipped` | — |
   | Not installed, non-interactive, no `--vscode` | print `VS Code detected; run with --vscode to install the extension` [`-VSCode`] | `available` |
   | Not installed, consent given | `--install-extension openlorraine.clorraine` | `installed` / `failed` |
   | Installed (any mode, no `--no-vscode`) | `--uninstall-extension openlorraine.clorraine`, then `--install-extension openlorraine.clorraine` | `updated` / `failed` |

   - Fresh install failure (non-zero exit or timeout): warn
     `could not install the VS Code extension from the Marketplace; run: code --install-extension openlorraine.clorraine`.
   - Update failure after a successful uninstall: warn
     `the Clorraine extension was removed but could not be reinstalled from the Marketplace; run: code --install-extension openlorraine.clorraine`.
   - Uninstall failure: warn
     `could not update the VS Code extension; run: code --install-extension openlorraine.clorraine --force`
     and stop the VS Code step (state `failed`).
   - Every failure continues the install; the installer's exit status is unaffected (AR-15).
5. **Summary lines**: `installed` → `extension installed`; `updated` → `extension updated from the
   Marketplace`; `available` → `VS Code detected; rerun with --vscode to install the extension`;
   `skipped` → `skipped`; `unavailable` → `VS Code not found`; `failed` → `extension not installed;
   run: code --install-extension openlorraine.clorraine`. The `already-installed` state is removed.
6. **Help text**: `--vscode` [`-VSCode`] → "install or update the VS Code extension from the
   Marketplace without asking"; `--no-vscode` [`-NoVSCode`] → "leave VS Code and its extensions
   unchanged". Next-steps text no longer says "from the release".

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Marketplace unreachable / extension not listed | Warn with the manual command; continue | AR-15 |
| Editor call hangs | Killed after the bound; `failed`; continue | AR-33 |
| Uninstall ok, install fails | Explicit "removed" warning with the reinstall command | AR-26 |
| Driver missing when a starter task runs | Extension error with install guide; task cancelled (03-1) | AR-32 |

## Testing Requirements

- Spec: `compiler/tests/first_run_spec_test.rs` (rewritten tasks assertion, ST-20);
  `tools/setup/tests/install.spec.test.sh` and `install.spec.test.ps1` (ST-21..ST-28), rewritten
  per AR-25.
- Impl: `install.impl.test.sh` / `install.impl.test.ps1` (argument order, timeout override parsing,
  summary text).
