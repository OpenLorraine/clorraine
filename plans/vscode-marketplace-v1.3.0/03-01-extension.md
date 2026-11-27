# Extension: VS Code Marketplace Extension and the v1.3.0 Release

> **Document**: 03-01-extension.md
> **Parent**: [Index](00-index.md)

## Overview

Make `tools/vscode/` listing-ready, add the `clorraine.driverPath` command used by the starter
tasks, add the "Open install guide" action and the version-drift notice, and extend packaging and
the audit to cover the new files. One universal VSIX; version from `compiler/Cargo.toml` (AR-3,
AR-4).

## Architecture

### Current Architecture

See 02 §What Exists (Extension, Missing-tool errors).

### Proposed Changes

1. **Manifest** (`package.json`), per AR-18 and AR-38:
   - `"icon": "images/icon.png"`, `"homepage": "https://openlorraine.github.io/clorraine/"`,
     `"bugs": {"url": "https://github.com/OpenLorraine/clorraine/issues"}`,
     `"keywords"` (at most 30; e.g. `amiga`, `m68k`, `clorraine`, `retro`, `compiler`,
     `language server`), `"galleryBanner": {"color": "#1b2a4a", "theme": "dark"}`,
     `"preview": true`, `"extensionKind": ["workspace"]`,
     `"capabilities": {"untrustedWorkspaces": {"supported": "limited", "description": "In Restricted Mode Clorraine does not start the language server, discover tools, save files, or run tasks."}, "virtualWorkspaces": false}`.
     `virtualWorkspaces: false` states the existing file-only behaviour (`actions.ts:204` requires
     `file` URIs) and is not a new capability.
   - `contributes.commands` gains `{ "command": "clorraine.driverPath", "title": "Clorraine: Driver Path" }`;
     `contributes.menus.commandPalette` gains `{ "command": "clorraine.driverPath", "when": "false" }`.
     `activationEvents` stays `onLanguage:clorraine`; implicit activation on contributed commands
     (VS Code ≥1.85) activates the extension when a task resolves the variable (AR-32).
2. **New module `src/guide.ts`** (pure, no `vscode` import, testable in plain Node like `paths.ts`):
   - `installGuideUrl(platform: NodeJS.Platform): string` — AR-35 URLs.
   - `serverOlderThan(server: string | undefined, extension: string): boolean` — numeric
     `x.y.z` comparison; `undefined`, non-matching (`/^\d+\.\d+\.\d+$/`), equal, or newer server
     → `false` (AR-35).
   - `driftMessage(server: string, extension: string): string` →
     `Clorraine CLI ${server} is older than the extension ${extension}. Run the installer to update.`
3. **Install-guide error** — one helper in `extension.ts`,
   `showMissingTool(message: string): Promise<void>`, shows
   `vscode.window.showErrorMessage(\`Clorraine: ${message}\`, "Open install guide")` and, when the
   action is chosen, `vscode.env.openExternal(vscode.Uri.parse(installGuideUrl(process.platform)))`.
   Used at the three missing-tool sites (AR-35):
   - language-server start (`extension.ts:268-274`) — replaces the plain error;
   - Build/Run (`actions.ts:303-307`) — only when the failure is the driver lookup (the
     `resolveTool("clorraine")` call at `:208`); other action errors keep today's text;
   - `clorraine.driverPath` (below).
   A "missing tool" is the `No <tool> found…` error from `resolveTool`; an explicit invalid
   setting keeps its own message and also gets the action (the fix is still to install or point
   at a valid tool).
4. **`clorraine.driverPath` command** (registered in `extension.ts` `activate`):
   returns `resolveTool("clorraine")`; on any thrown error it calls `showMissingTool` with the
   error text and returns `undefined`, which cancels the task without a second error (AR-32).
   Tasks never run in Restricted Mode, so the trust branch of `resolveTool` is not reached through
   tasks; if invoked anyway it follows the same error path.
5. **Drift notice** — after `client.start()` succeeds (`extension.ts:296-303`), read
   `initialization.serverInfo?.version`; if `serverOlderThan(server, context.extension.packageJSON.version)`
   and the notice has not been shown in this activation (module-level flag), show
   `vscode.window.showInformationMessage(driftMessage(...), "Open install guide")` (AR-16,
   AR-35). Restarts after configuration changes do not repeat it.
6. **Listing files**:
   - `README.md` rewritten as the listing (AR-38): what it does; requirements (the Clorraine CLI
     with install-guide links for Linux and Windows); platforms line "Linux x86_64 and Windows 11
     x64; macOS is not supported yet"; settings table; Restricted Mode; the standard trademark and
     independence line (as in `site/.vitepress/config.ts` footer); license. Absolute `https`
     links only. No screenshot.
   - `DEVELOPMENT.md` (new, not staged): the packaging and test notes moved out of the README.
   - `CHANGELOG.md` (new): `## 1.3.0` (Marketplace listing, driver command for portable starter
     tasks, install-guide action, version notice, Windows 11 support listed) and `## 1.2.0`
     (first release: diagnostics, hover, definition, completion, symbols, Build/Run/Run in
     FS-UAE) (AR-36).
   - `images/icon.png`: 256×256 PNG rendered once from `site/public/logo.svg` (ImageMagick or
     headless Chrome on the maintainer host) and committed (AR-38).
7. **Staging and audit**: `stage.mjs` also copies `CHANGELOG.md` and `images/icon.png`;
   `audit_vsix.py` adds both to `REQUIRED`, rejects `extension/DEVELOPMENT.md`, checks the PNG
   signature and that width and height are ≥128 (IHDR read with the standard library), and checks
   the manifest fields of item 1 (`preview` true, `extensionKind == ["workspace"]`, `icon`,
   `untrustedWorkspaces.supported == "limited"`, `clorraine.driverPath` contributed and hidden).
   `.vscodeignore` keeps excluding sources.

## Implementation Details

### New Functions/Methods

```ts
// src/guide.ts
export function installGuideUrl(platform: NodeJS.Platform): string;
export function serverOlderThan(server: string | undefined, extension: string): boolean;
export function driftMessage(server: string, extension: string): string;
```

### Integration Points

- The starter `tasks.json` (03-3) calls `${command:clorraine.driverPath}`.
- The publish workflow (03-2) runs `make vscode-package` and `audit_vsix.py` on this package.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| No language server found at start | Error with "Open install guide"; no session | AR-9, AR-35 |
| No driver for Build/Run | Same action on the driver-lookup failure | AR-35 |
| No driver for a task | Error with action; command returns `undefined`; task cancelled | AR-32 |
| Server older than extension | One informational notice per activation | AR-16, AR-35 |
| Missing or unparsable server version | No notice | AR-35 |

## Testing Requirements

- Spec: `test/marketplace.spec.test.ts` (manifest, staging, audit), `test/guide.spec.test.ts`
  (pure helpers), `test/driver-command.spec.test.ts` (command and guide action through the
  existing fake-host fixtures) — ST-11..ST-19.
- Impl: `test/guide.impl.test.ts` (edge versions), audit edge cases.
