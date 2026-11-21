/**
 * Trusted saved-source Build/Run tasks. The server selects files; this adapter
 * owns editor saving, dirty-buffer checks and literal process argument arrays.
 */
import * as fs from "node:fs";
import * as path from "node:path";

import * as vscode from "vscode";

import type { LspClient } from "./client";
import { machinePath, resolveTool } from "./discovery";

/** The initialized server state needed by an action, never a second client. */
export interface SelectionSession {
  /** Current shared stdio client. */
  readonly client: LspClient;
  /** Negotiated experimental source-selection version; only exactly one is supported. */
  readonly sourceFilesVersion: unknown;
}

/** One user-facing operation and its unchanged driver action. */
interface Action {
  /** Public command ID. */
  id: string;
  /** Task and diagnostic operation name. */
  title: string;
  /** Driver verb. */
  verb: "build" | "run";
  /** Whether to request a native FS-UAE process instead of vamos. */
  emulator: boolean;
}

/** The three registered command contracts. */
const ACTIONS: readonly Action[] = [
  { id: "clorraine.build", title: "Build", verb: "build", emulator: false },
  { id: "clorraine.run", title: "Run", verb: "run", emulator: false },
  { id: "clorraine.runInFsUae", title: "Run in FS-UAE", verb: "run", emulator: true },
];

/** Reject malformed server responses before their values can become process arguments. */
function selectedFiles(value: unknown, root: string, entry: string): string[] {
  if (typeof value !== "object" || value === null || !("files" in value) ||
      !Array.isArray(value.files) || value.files.length < 1 || value.files.length > 256) {
    throw new Error("Invalid source selection; update the language server and retry.");
  }
  const files: string[] = [];
  const canonicalRoot = fs.realpathSync(root);
  for (const item of value.files) {
    if (typeof item !== "string" || !path.isAbsolute(item) ||
        fs.realpathSync(item) !== item || !fs.statSync(item).isFile()) {
      throw new Error("Invalid saved source path; save the sources and retry selection.");
    }
    const relative = path.relative(canonicalRoot, item);
    if (relative === ".." || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
      throw new Error("Source selection escaped its folder; fix the source root and retry.");
    }
    files.push(item);
  }
  if (files[0] !== fs.realpathSync(entry) || new Set(files).size !== files.length) {
    throw new Error("Invalid entry or duplicate source selection; update the language server and retry.");
  }
  const tail = files.slice(1);
  if (tail.some((item, index) => index > 0 &&
      Buffer.compare(Buffer.from(tail[index - 1]), Buffer.from(item)) >= 0)) {
    throw new Error("Invalid source ordering; update the language server and retry.");
  }
  return files;
}

/** Recheck the entry across awaits so changed buffers are never compiled as saved bytes. */
function unchanged(document: vscode.TextDocument, version: number): void {
  if (vscode.window.activeTextEditor?.document !== document ||
      document.isDirty || document.version !== version) {
    throw new Error("The active entry changed; save it and retry.");
  }
  if (!vscode.workspace.isTrusted) {
    throw new Error("Grant workspace trust and retry.");
  }
}

/** Check only selected dirty overlays; never save dependencies or unrelated documents. */
function requireSavedDependencies(files: readonly string[]): void {
  const selected = new Set(files);
  for (const document of vscode.workspace.textDocuments) {
    if (!document.isDirty || document.uri.scheme !== "file") {
      continue;
    }
    let canonical = document.uri.fsPath;
    try {
      canonical = fs.realpathSync(canonical);
    } catch {
      // An unrelated missing file must not prevent a saved program from running.
    }
    if (selected.has(canonical)) {
      throw new Error(`Save selected source ${document.uri.fsPath}, then retry.`);
    }
  }
}

/**
 * Match source events against the same canonical root used by saved selection.
 * A newly created unsaved file may not exist yet; resolve its parent when
 * possible so directory aliases do not hide an intervening editor change.
 */
function sourceUnderRoot(document: vscode.TextDocument, canonicalRoot: string): boolean {
  if (document.uri.scheme !== "file" || path.extname(document.uri.fsPath) !== ".clo") {
    return false;
  }
  let canonical = path.resolve(document.uri.fsPath);
  try {
    canonical = fs.realpathSync(canonical);
  } catch {
    try {
      canonical = path.join(fs.realpathSync(path.dirname(canonical)), path.basename(canonical));
    } catch {
      // Missing parent directories cannot be indexed yet; retain the absolute
      // spelling so a new file under the ordinary source root still invalidates.
    }
  }
  const relative = path.relative(canonicalRoot, canonical);
  return relative !== ".." && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
}

/**
 * ProcessExecution avoids shell parsing, but VS Code still expands task
 * variables. Reject their introducer in every final input rather than trying
 * quotes or backslashes, which are not escapes at that later boundary.
 */
function requireLiteralTaskInputs(executable: string, args: readonly string[], cwd: string): void {
  if ([executable, ...args, cwd].some((value) => value.includes("${"))) {
    throw new Error("Task paths cannot contain ${ because VS Code expands task variables. Rename the file or folder, or change the tool path in User Settings, then retry.");
  }
}

/**
 * Register commands once per activation; only this extension's tasks are owned.
 * Command handlers query the current session after saving, so restarts never
 * retain a stale language-server connection.
 */
export function registerActions(
  context: vscode.ExtensionContext,
  currentSession: () => SelectionSession | undefined,
): void {
  const owned = new Set<vscode.TaskExecution>();
  const pending = new Set<vscode.Disposable>();
  let disposed = false;
  context.subscriptions.push(
    vscode.tasks.onDidEndTask((event) => owned.delete(event.execution)),
    { dispose: () => {
      disposed = true;
      for (const execution of owned) {
        execution.terminate();
      }
      owned.clear();
      for (const listener of pending) {
        listener.dispose();
      }
      pending.clear();
    } },
  );
  for (const action of ACTIONS) {
    context.subscriptions.push(vscode.commands.registerCommand(action.id, async () => {
      if (disposed) {
        return;
      }
      const profile = vscode.workspace.getConfiguration("clorraine").get<string>("profile") ?? "hosted-amiga";
      const title = `Clorraine: ${action.title} (${profile})`;
      const selectionListeners: vscode.Disposable[] = [];
      try {
        if (!vscode.workspace.isTrusted) {
          throw new Error("Grant workspace trust in Manage Workspace Trust before building or running.");
        }
        if (!["host", "hosted-amiga", "hosted-amiga-13", "freestanding"].includes(profile)) {
          throw new Error("Select a supported clorraine.profile in Settings and retry.");
        }
        if (action.emulator && profile !== "hosted-amiga" && profile !== "hosted-amiga-13") {
          throw new Error("Select hosted-amiga or hosted-amiga-13 for Run in FS-UAE.");
        }
        const document = vscode.window.activeTextEditor?.document;
        if (!document || document.isUntitled || document.uri.scheme !== "file" ||
            path.extname(document.uri.fsPath) !== ".clo") {
          throw new Error("Open and save a file-backed .clo entry, then retry.");
        }
        const driver = resolveTool("clorraine");
        if (document.isDirty && !await document.save()) {
          throw new Error("Entry save was cancelled; save the .clo file and retry.");
        }
        if (disposed) {
          return;
        }
        const version = document.version;
        unchanged(document, version);
        const connection = currentSession();
        if (!connection?.client.ready || connection.sourceFilesVersion !== 1) {
          throw new Error("Update the Clorraine language server to a compatible source-selection version and restart the extension.");
        }
        const folder = vscode.workspace.getWorkspaceFolder(document.uri);
        const root = folder?.uri.fsPath ?? path.dirname(document.uri.fsPath);
        const canonicalRoot = fs.realpathSync(root);
        let sourcesChanged = false;
        const changed = (source: vscode.TextDocument) => {
          if (sourceUnderRoot(source, canonicalRoot)) {
            sourcesChanged = true;
          }
        };
        // Observe before the request, not after its reply: an edited dependency
        // may already be clean again by the time the old closure returns.
        selectionListeners.push(
          vscode.workspace.onDidChangeTextDocument((event) => changed(event.document)),
        );
        selectionListeners.push(vscode.workspace.onDidSaveTextDocument(changed));
        for (const listener of selectionListeners) {
          pending.add(listener);
        }
        /** A clean final buffer cannot prove that the saved closure is current. */
        const requireCurrentSources = () => {
          if (sourcesChanged) {
            throw new Error("Sources changed during selection; save them and retry.");
          }
        };
        let response: unknown;
        try {
          response = await connection.client.request("clorraine/sourceFiles", {
            entryUri: document.uri.toString(), rootUri: vscode.Uri.file(root).toString(),
          });
        } catch (error) {
          throw new Error(`Source selection failed: ${error instanceof Error ? error.message : String(error)}. Fix imports and retry.`);
        }
        if (disposed) {
          return;
        }
        unchanged(document, version);
        requireCurrentSources();
        if (currentSession() !== connection) {
          throw new Error("The language server changed; retry source selection.");
        }
        const files = selectedFiles(response, root, document.uri.fsPath);
        requireSavedDependencies(files);
        const args = [action.verb, "--profile", profile];
        if (profile !== "host") {
          const sdk = machinePath("sdk.path");
          if (sdk) {
            args.push("--sdk-root", sdk);
          }
        }
        if (action.emulator) {
          args.push("--emulator");
          for (const [key, flag] of [["fsUae.path", "--fs-uae"], ["kickstart.path", "--kickstart"]] as const) {
            const value = machinePath(key);
            if (value) {
              args.push(flag, value);
            }
          }
        }
        args.push(...files);
        requireLiteralTaskInputs(driver, args, root);
        const task = new vscode.Task(
          { type: "clorraine", action: action.verb, profile },
          folder ?? vscode.TaskScope.Global, title, "Clorraine",
          new vscode.ProcessExecution(driver, args, { cwd: root }), [],
        );
        task.presentationOptions = {
          reveal: vscode.TaskRevealKind.Always, panel: vscode.TaskPanelKind.Dedicated,
        };
        requireCurrentSources();
        const execution = await vscode.tasks.executeTask(task);
        // Task submission is asynchronous. Disposal can happen before VS Code
        // returns the execution, so that late task must be stopped immediately.
        if (disposed) {
          execution.terminate();
        } else {
          owned.add(execution);
        }
      } catch (error) {
        if (!disposed) {
          void vscode.window.showErrorMessage(
            `${title}: ${error instanceof Error ? error.message : String(error)} Check settings/saved sources and retry.`,
          );
        }
      } finally {
        for (const listener of selectionListeners) {
          listener.dispose();
          pending.delete(listener);
        }
      }
    }));
  }
}
