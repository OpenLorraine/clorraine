# VS Code Extension: DX and Adoption Closeout

> **Document**: 03-05-vscode-extension.md
> **Parent**: [Index](00-index.md)

## Overview

A TypeScript package under `tools/vscode/` that launches `clorraine-lsp`,
routes the LSP features into the VS Code API, registers the canonical TextMate
grammar, and packages a `.vsix`. Runtime code has no npm dependency.
Requirements: R9-R10, AR-23 to AR-26.

## Architecture

### Current Architecture

`editors/clorraine.tmLanguage.json` is canonical and consumed by
`tools/demo`. `tools/demo` is the reference npm-only package: TypeScript, no
build step for tests, `node --test`, and a `package-lock.json`.

### Proposed Changes

```
tools/vscode/
├── package.json
├── package-lock.json
├── tsconfig.json
├── README.md
├── .vscodeignore
├── language-configuration.json
├── src/
│   ├── client.ts        # no vscode import
│   └── extension.ts     # vscode adapter
├── test/
│   ├── client.test.ts
│   └── packaged.test.ts
└── scripts/
    ├── stage.mjs        # copy grammar and build output into build/vscode
    └── audit_vsix.py    # archive manifest/grammar audit
```

## Implementation Details

### Manifest

```json
{
  "name": "clorraine",
  "displayName": "Clorraine",
  "description": "Clorraine language support",
  "version": "1.1.0",
  "publisher": "openlorraine",
  "license": "MIT",
  "repository": {"type": "git", "url": "https://github.com/OpenLorraine/clorraine"},
  "engines": {"vscode": "^1.85.0"},
  "activationEvents": ["onLanguage:clorraine"],
  "main": "./out/extension.js",
  "contributes": {
    "languages": [{
      "id": "clorraine",
      "aliases": ["Clorraine", "clorraine"],
      "extensions": [".clo"],
      "configuration": "./language-configuration.json"
    }],
    "grammars": [{
      "language": "clorraine",
      "scopeName": "source.clorraine",
      "path": "./syntaxes/clorraine.tmLanguage.json"
    }],
    "configuration": {
      "title": "Clorraine",
      "properties": {
        "clorraine.lsp.path": {"type": "string", "default": ""},
        "clorraine.profile": {
          "type": "string",
          "enum": ["hosted-amiga", "hosted-amiga-13", "host", "freestanding"],
          "default": "hosted-amiga"
        }
      }
    }
  },
  "scripts": {
    "typecheck": "tsc --noEmit",
    "test": "node --test test/*.test.ts",
    "stage": "node scripts/stage.mjs",
    "package": "npm run stage && cd ../../build/vscode/extension && npx @vscode/vsce package --out ../clorraine-1.1.0.vsix"
  },
  "devDependencies": {
    "@types/node": "^22.0.0",
    "@types/vscode": "^1.85.0",
    "@vscode/vsce": "^3.0.0",
    "typescript": "^5.9.0"
  }
}
```

The version is read from the workspace package at stage time; the literal in
`package.json` is kept in sync by `stage.mjs` and a test asserts it matches
`compiler/Cargo.toml`. This avoids a second hand-maintained version of record.

### Client (`src/client.ts`)

```ts
export interface LspClientOptions {
  command: string;
  args?: string[];
  cwd?: string;
  env?: NodeJS.ProcessEnv;
  initializationOptions?: unknown;
  onNotification?: (method: string, params: unknown) => void;
  onDiagnostics?: (params: PublishDiagnosticsParams) => void;
  onExit?: (code: number | null) => void;
}

export class LspClient {
  start(): Promise<InitializeResult>;
  request<T>(method: string, params: unknown): Promise<T>;
  notify(method: string, params: unknown): void;
  stop(): Promise<void>;
}
```

- Framing mirrors the server: `Content-Length` in bytes, `\r\n`.
- A 32 MiB read cap.
- Request ids are increasing integers; a pending map resolves or rejects.
- A response with an error rejects with `LspError { code, message }`.
- Notifications: `textDocument/publishDiagnostics` goes to `onDiagnostics`,
  everything else to `onNotification`.
- `stop()` sends `shutdown`, waits for the response, sends `exit`, and kills
  the child after a 2-second grace period.
- A crash rejects every pending request and calls `onExit`.
- `start()` sends `initialize` with `processId`, `rootUri`, `capabilities`
  (empty object), and `initializationOptions`, then `initialized`.

Server path resolution lives in `extension.ts`, not the client, so the client
stays testable with an explicit command.

### Extension (`src/extension.ts`)

- `activate`:
  - resolve the server path (`clorraine.lsp.path`, `CLORRAINE_LSP`,
    `target/debug/clorraine-lsp`, `target/release/clorraine-lsp`, `PATH`);
  - create the client with `rootUri` = first workspace folder;
  - create a `DiagnosticCollection("clorraine")`;
  - register hover, definition, completion, and document-symbol providers;
  - send `didOpen` for every visible `.clo` document and wire
    `onDidOpenTextDocument`, `onDidChangeTextDocument`,
    `onDidCloseTextDocument`, `onDidSaveTextDocument`.
- A configuration change to `clorraine.lsp.path` or `clorraine.profile`
  stops and restarts the client.
- A server start failure shows one error message naming the setting and the
  path; it does not retry in a loop.
- Diagnostics convert 0-based ranges directly to `vscode.Range`.
- Hover converts `{kind:"markdown", value}` to `MarkdownString`.
- Definition accepts one location or an array.
- Completion maps LSP kinds to `vscode.CompletionItemKind`.
- Document symbols map to `vscode.DocumentSymbol`.

### Grammar and staging

`scripts/stage.mjs`:

1. Reads the workspace version from `compiler/Cargo.toml`.
2. Creates `build/vscode/extension/`.
3. Copies `package.json`, `README.md`, `language-configuration.json`, and
   `out/`.
4. Copies `editors/clorraine.tmLanguage.json` to
   `build/vscode/extension/syntaxes/clorraine.tmLanguage.json`.
5. Writes a staged `package.json` with the workspace version and
   `main: "./out/extension.js"`.

`make vscode-package` runs `tsc`, `stage.mjs`, and `vsce package` from the
staged directory. The `.vsix` lands in `build/vscode/`. Nothing is written
under `tools/vscode/` except `out/` and `node_modules/`, both gitignored.

`make vscode-check` first runs `cargo build --workspace` inside the pinned SDK
container; the host has no Rust toolchain. It then runs the Node checks with
`CLORRAINE_LSP` pointing at the container-built `target/debug/clorraine-lsp`.

### Headless tests

`test/client.test.ts`:

- frames a request split across writes against a fake in-process server;
- rejects a malformed response;
- runs the real `clorraine-lsp` from `CLORRAINE_LSP` over a fixture workspace
  and asserts diagnostics, hover, and clean shutdown.

`test/packaged.test.ts`:

- reads `CLORRAINE_VSIX` (set by `make vscode-check`);
- runs `python3 scripts/audit_vsix.py --vsix ... --extract ...`;
- imports the extracted `extension/out/client.js`;
- starts the real server, initializes, and asserts one diagnostic;
- compares the extracted grammar bytes with `editors/`.

### Error handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Server binary absent | One error message with the resolution order | AR-23 |
| Server crashes | Reject pending requests; show once; no loop | AR-23 |
| Malformed frame from server | Reject pending request; stop the client | AR-24 |
| Diagnostics for an unopened file | Ignore | AR-26 |
| Grammar drift | Packaging test fails | AR-25 |
| Node absent | `make vscode-check` exits 2 with the requirement | AR-24 |

## Testing Requirements

- `npm run typecheck` clean.
- `npm test` headless.
- `make vscode-check` packages and audits the `.vsix`.
- The audit asserts `extension/package.json`, `extension/out/extension.js`,
  `extension/out/client.js`, `extension/syntaxes/clorraine.tmLanguage.json`,
  and `extension/README.md`; it rejects `node_modules`, `src`, `test`, and
  `*.ts`.
