# Language Server and Editor: Windows CLI and installer

> **Document**: 03-03-lsp-editor.md
> **Parent**: [Index](00-index.md)

## Overview

On Windows the language server and the VS Code extension must spell the same file the same way,
and the extension must find the installed CLI. Today `file:///c%3A/x.clo` parses to `/c:/x.clo`,
`sourceFiles` returns verbatim `\\?\C:\…` paths, and the extension compares strings strictly, so
every Build/Run would fail or skip the unsaved-dependency guard (AR-38).

## Architecture

### Current Architecture
- `lsp/uri.rs:48-90`: `parse` keeps the URI path; `from_path` percent-encodes every byte except
  unreserved characters and `/`.
- Document and index keys are `path.display()` strings (`uri.rs:24-29`, `workspace.rs`).
- `tools/vscode/src/actions.ts:67-115` compares `fs.realpathSync` results and server paths with
  `===`; `discovery.ts:97` looks only in `~/.local/bin/<name>`.

### Proposed Changes

#### Server (Rust)
- `uri::parse_for(os, uri)` and `uri::from_path_for(os, path)`; `parse`/`from_path` call them with
  `HostOs::current()`.
- Windows parse: decode `%XX`; `file:///c:/a/b.clo` and `file:///C%3A/a/b.clo` both become
  `C:\a\b.clo` (uppercase drive, backslashes). A host other than empty or `localhost` (UNC) is an
  error: `network-share paths are not supported`.
- Windows format: `C:\a b\x.clo` → `file:///c%3A/a%20b/x.clo` (lowercase drive, `%3A`, forward
  slashes), the VS Code shape.
- Every key and every `sourceFiles` result passes through one normalizer,
  `lsp::uri::normalize_for(os, path)`: `display_path`, then uppercase drive. Unix keys are
  unchanged.

#### Extension (TypeScript)
- A `samePath(a, b, platform = process.platform)` helper: on `win32` compare after
  `path.win32.normalize` and lowercasing; elsewhere `===`. `actions.ts` uses it for the
  saved-source and dirty-buffer checks.
- `resolveTool(name)` on `win32`: `%LOCALAPPDATA%\Programs\Clorraine\current\bin\<name>.exe`
  before the workspace build outputs and PATH (`<name>.exe` on PATH). Linux order unchanged.
- The `package.json` setting descriptions stop calling FS-UAE "native Linux" and mention
  `clorraine.exe`.
- The extension's version follows the release version chosen at release time (AR-3).

## Implementation Details

### New Functions/Methods

```rust
pub fn parse_for(os: HostOs, uri: &str) -> Result<DocumentUri, String>;
pub fn from_path_for(os: HostOs, path: &str) -> String;
pub fn normalize_for(os: HostOs, path: &str) -> String;
```

```ts
export function samePath(a: string, b: string, platform?: NodeJS.Platform): boolean;
export function installedToolPath(name: string, platform: NodeJS.Platform, env: NodeJS.ProcessEnv, home: string): string;
```

### Integration Points
- `platform::display_path` (03-02) is the first normalization step.
- The installer installs the VSIX from the same release (03-04).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| UNC `file://server/share/x.clo` on Windows | Request error `network-share paths are not supported` | AR-38 |
| Malformed percent escape | Unchanged existing error | AR-38 |
| Installed CLI missing on Windows | Existing "No clorraine found" message, unchanged | AR-38 |

## Testing Requirements
- ST-31 – ST-38 in [07-testing-strategy.md](07-testing-strategy.md).
