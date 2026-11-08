//! VS Code adapter for the Clorraine language server.
//!
//! The adapter owns the editor-facing pieces: server discovery, document
//! synchronization, diagnostics, and the four navigation providers. All
//! protocol work lives in `client.ts`, so the same client is exercised by the
//! headless tests.

import * as fs from "node:fs";
import * as path from "node:path";
import { pathToFileURL } from "node:url";

import * as vscode from "vscode";

import {
  CompletionItem,
  Diagnostic as LspDiagnostic,
  DocumentSymbol as LspDocumentSymbol,
  Hover,
  Location as LspLocation,
  LspClient,
} from "./client";

let session: Session | undefined;

/** Everything one running server owns. */
class Session implements vscode.Disposable {
  private readonly diagnostics: vscode.DiagnosticCollection;
  private readonly disposables: vscode.Disposable[] = [];
  private stopping = false;
  private exitedOnce = false;

  readonly client: LspClient;
  private readonly context: vscode.ExtensionContext;

  constructor(client: LspClient, context: vscode.ExtensionContext) {
    this.client = client;
    this.context = context;
    this.diagnostics = vscode.languages.createDiagnosticCollection("clorraine");
    this.context.subscriptions.push(this.diagnostics);
  }

  /** Registers the providers and document listeners. */
  register(): void {
    this.disposables.push(
      vscode.languages.registerHoverProvider("clorraine", {
        provideHover: async (document, position) => {
          if (!this.client.ready) {
            return undefined;
          }
          const result = await this.client.request<Hover | null>(
            "textDocument/hover",
            documentPosition(document, position),
          );
          if (!result) {
            return undefined;
          }
          const value =
            typeof result.contents === "string"
              ? result.contents
              : result.contents.value;
          return new vscode.Hover(
            new vscode.MarkdownString(value),
            result.range ? toRange(result.range) : undefined,
          );
        },
      }),
      vscode.languages.registerDefinitionProvider("clorraine", {
        provideDefinition: async (document, position) => {
          if (!this.client.ready) {
            return undefined;
          }
          const result = await this.client.request<LspLocation | LspLocation[] | null>(
            "textDocument/definition",
            documentPosition(document, position),
          );
          if (!result) {
            return undefined;
          }
          const locations = Array.isArray(result) ? result : [result];
          return locations.map(
            (location) =>
              new vscode.Location(
                vscode.Uri.parse(location.uri),
                toRange(location.range),
              ),
          );
        },
      }),
      vscode.languages.registerCompletionItemProvider(
        "clorraine",
        {
          provideCompletionItems: async (document, position) => {
            if (!this.client.ready) {
              return [];
            }
            const result = await this.client.request<CompletionItem[] | null>(
              "textDocument/completion",
              documentPosition(document, position),
            );
            return (result ?? []).map((item) => {
              const completion = new vscode.CompletionItem(
                item.label,
                toCompletionKind(item.kind),
              );
              completion.detail = item.detail;
              return completion;
            });
          },
        },
        ".",
        ":",
      ),
      vscode.languages.registerDocumentSymbolProvider("clorraine", {
        provideDocumentSymbols: async (document) => {
          if (!this.client.ready) {
            return [];
          }
          const result = await this.client.request<LspDocumentSymbol[] | null>(
            "textDocument/documentSymbol",
            {
              textDocument: { uri: document.uri.toString() },
            },
          );
          return (result ?? []).map((symbol) => toDocumentSymbol(symbol));
        },
      }),
      vscode.workspace.onDidOpenTextDocument((document) =>
        this.didOpen(document),
      ),
      vscode.workspace.onDidChangeTextDocument((event) =>
        this.didChange(event.document),
      ),
      vscode.workspace.onDidCloseTextDocument((document) =>
        this.didClose(document),
      ),
      vscode.workspace.onDidSaveTextDocument((document) =>
        this.didSave(document),
      ),
      vscode.workspace.onDidChangeConfiguration((event) => {
        if (
          event.affectsConfiguration("clorraine.lsp.path") ||
          event.affectsConfiguration("clorraine.profile")
        ) {
          void restart(this.context);
        }
      }),
    );
    for (const document of vscode.workspace.textDocuments) {
      this.didOpen(document);
    }
  }

  /** Sends `didOpen` for a Clorraine document. */
  private didOpen(document: vscode.TextDocument): void {
    if (document.languageId !== "clorraine") {
      return;
    }
    this.client.notify("textDocument/didOpen", {
      textDocument: {
        uri: document.uri.toString(),
        languageId: document.languageId,
        version: document.version,
        text: document.getText(),
      },
    });
  }

  /** Sends a full-text `didChange`. */
  private didChange(document: vscode.TextDocument): void {
    if (document.languageId !== "clorraine") {
      return;
    }
    this.client.notify("textDocument/didChange", {
      textDocument: {
        uri: document.uri.toString(),
        version: document.version,
      },
      contentChanges: [{ text: document.getText() }],
    });
  }

  /** Sends `didClose` and clears the diagnostics it owned. */
  private didClose(document: vscode.TextDocument): void {
    if (document.languageId !== "clorraine") {
      return;
    }
    this.client.notify("textDocument/didClose", {
      textDocument: { uri: document.uri.toString() },
    });
    this.diagnostics.delete(document.uri);
  }

  /** Sends `didSave`. */
  private didSave(document: vscode.TextDocument): void {
    if (document.languageId !== "clorraine") {
      return;
    }
    this.client.notify("textDocument/didSave", {
      textDocument: { uri: document.uri.toString() },
      text: document.getText(),
    });
  }

  /** Applies one diagnostics notification. */
  setDiagnostics(params: { uri: string; diagnostics: LspDiagnostic[] }): void {
    const values = params.diagnostics.map((diagnostic) => {
      const mapped = new vscode.Diagnostic(
        toRange(diagnostic.range),
        diagnostic.message,
        toSeverity(diagnostic.severity),
      );
      mapped.code = diagnostic.code;
      mapped.source = diagnostic.source ?? "clorraine";
      return mapped;
    });
    this.diagnostics.set(vscode.Uri.parse(params.uri), values);
  }

  /** Marks the session as stopping so an exit is not reported as a crash. */
  beginStop(): void {
    this.stopping = true;
  }

  /** Handles an unexpected server exit once. */
  unexpectedExit(message: string): void {
    if (this.stopping || this.exitedOnce) {
      return;
    }
    this.exitedOnce = true;
    void vscode.window.showErrorMessage(`Clorraine language server stopped: ${message}`);
  }

  dispose(): void {
    for (const disposable of this.disposables) {
      disposable.dispose();
    }
    this.diagnostics.dispose();
  }
}

/** Activates the extension. */
export async function activate(
  context: vscode.ExtensionContext,
): Promise<void> {
  await start(context);
}

/** Deactivates the extension. */
export async function deactivate(): Promise<void> {
  await stop();
}

/** Starts one server session. */
async function start(context: vscode.ExtensionContext): Promise<void> {
  const configuration = vscode.workspace.getConfiguration("clorraine");
  const profile = configuration.get<string>("profile") ?? "hosted-amiga";
  let command: string;
  try {
    command = resolveServer();
  } catch (error) {
    void vscode.window.showErrorMessage(
      `Clorraine: ${error instanceof Error ? error.message : String(error)}`,
    );
    return;
  }
  const root = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  let starting = true;
  const client = new LspClient({
    command,
    rootUri: root ? pathToFileURL(root).toString() : undefined,
    initializationOptions: { profile },
    onDiagnostics: (params) => session?.setDiagnostics(params),
    onExit: (code) => {
      if (!starting) {
        session?.unexpectedExit(`exit code ${code ?? "null"}`);
      }
    },
  });
  const created = new Session(client, context);
  session = created;
  try {
    await client.start();
    starting = false;
    created.register();
  } catch (error) {
    starting = false;
    await client.stop();
    created.dispose();
    session = undefined;
    void vscode.window.showErrorMessage(
      `Clorraine: could not start ${command}: ${
        error instanceof Error ? error.message : String(error)
      }`,
    );
    return;
  }
  context.subscriptions.push(created);
}

/** Stops the current session. */
async function stop(): Promise<void> {
  const current = session;
  session = undefined;
  if (!current) {
    return;
  }
  current.beginStop();
  current.dispose();
  await current.client.stop();
}

/** Restarts the session after a configuration change. */
async function restart(context: vscode.ExtensionContext): Promise<void> {
  await stop();
  await start(context);
}

/** Resolves the server executable from settings, environment, build, or PATH. */
function resolveServer(): string {
  const configured = vscode.workspace
    .getConfiguration("clorraine")
    .get<string>("lsp.path")
    ?.trim();
  if (configured) {
    return configured;
  }
  if (process.env.CLORRAINE_LSP) {
    return process.env.CLORRAINE_LSP;
  }
  for (const folder of vscode.workspace.workspaceFolders ?? []) {
    for (const profile of ["debug", "release"]) {
      const candidate = path.join(
        folder.uri.fsPath,
        "target",
        profile,
        "clorraine-lsp",
      );
      if (fs.existsSync(candidate)) {
        return candidate;
      }
    }
  }
  const onPath = findOnPath("clorraine-lsp");
  if (onPath) {
    return onPath;
  }
  throw new Error(
    "no clorraine-lsp found; set clorraine.lsp.path, CLORRAINE_LSP, or build target/debug/clorraine-lsp",
  );
}

/** Finds an executable on PATH without spawning a shell. */
function findOnPath(name: string): string | undefined {
  const pathValue = process.env.PATH ?? "";
  const separator = process.platform === "win32" ? ";" : ":";
  for (const directory of pathValue.split(separator)) {
    if (!directory) {
      continue;
    }
    const candidate = path.join(directory, name);
    try {
      fs.accessSync(candidate, fs.constants.X_OK);
      return candidate;
    } catch {
      // Not here; keep looking.
    }
  }
  return undefined;
}

/** Builds LSP text-document and position params. */
function documentPosition(
  document: vscode.TextDocument,
  position: vscode.Position,
): Record<string, unknown> {
  return {
    textDocument: { uri: document.uri.toString() },
    position: { line: position.line, character: position.character },
  };
}

/** Converts an LSP range to a VS Code range. */
function toRange(range: {
  start: { line: number; character: number };
  end: { line: number; character: number };
}): vscode.Range {
  return new vscode.Range(
    range.start.line,
    range.start.character,
    range.end.line,
    range.end.character,
  );
}

/** Converts an LSP severity. */
function toSeverity(severity: number | undefined): vscode.DiagnosticSeverity {
  switch (severity) {
    case 1:
      return vscode.DiagnosticSeverity.Error;
    case 2:
      return vscode.DiagnosticSeverity.Warning;
    case 3:
      return vscode.DiagnosticSeverity.Information;
    default:
      return vscode.DiagnosticSeverity.Hint;
  }
}

/** Converts an LSP completion kind. */
function toCompletionKind(kind: number | undefined): vscode.CompletionItemKind {
  switch (kind) {
    case 3:
      return vscode.CompletionItemKind.Function;
    case 12:
      return vscode.CompletionItemKind.Function;
    case 14:
      return vscode.CompletionItemKind.Keyword;
    case 23:
      return vscode.CompletionItemKind.Struct;
    case 10:
      return vscode.CompletionItemKind.Enum;
    case 5:
      return vscode.CompletionItemKind.Class;
    case 9:
      return vscode.CompletionItemKind.Module;
    case 25:
      return vscode.CompletionItemKind.TypeParameter;
    default:
      return vscode.CompletionItemKind.Text;
  }
}

/** Converts an LSP document symbol. */
function toDocumentSymbol(symbol: LspDocumentSymbol): vscode.DocumentSymbol {
  return new vscode.DocumentSymbol(
    symbol.name,
    "",
    toSymbolKind(symbol.kind),
    toRange(symbol.range),
    toRange(symbol.selectionRange),
  );
}

/** Converts an LSP symbol kind. */
function toSymbolKind(kind: number): vscode.SymbolKind {
  switch (kind) {
    case 12:
      return vscode.SymbolKind.Function;
    case 23:
      return vscode.SymbolKind.Struct;
    case 10:
      return vscode.SymbolKind.Enum;
    case 5:
      return vscode.SymbolKind.Class;
    case 3:
      return vscode.SymbolKind.Namespace;
    case 14:
      return vscode.SymbolKind.Constant;
    default:
      return vscode.SymbolKind.Variable;
  }
}
