/**
 * A minimal editor facade for the install-guide, driver-command, and version
 * notice scenarios. It loads the staged extension unchanged, records every
 * message with its action items, answers a message with the configured
 * action, and records URLs the extension asks the editor to open.
 *
 * Scenario (JSON file in argv[2]): extension, packageJson, workspace, entry,
 * configuration (setting → value), choose (action label or null),
 * restart (emit one profile change after startup), commands (ids to execute
 * in order).
 */
const fs = require("node:fs");
const path = require("node:path");
const { registerHooks } = require("node:module");
const { fileURLToPath, pathToFileURL } = require("node:url");

const scenario = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const commands = new Map();
const listeners = new Map();
const messages = [];
const opened = [];
const results = {};

/** Waits until `predicate` holds or a short deadline passes. */
async function settle(predicate, ms = 1500) {
  const end = Date.now() + ms;
  while (!predicate() && Date.now() < end) await new Promise((resolve) => setTimeout(resolve, 10));
  return predicate();
}

/** File and parsed URIs as the extension uses them. */
class Uri {
  constructor(value) {
    this.value = new URL(value);
    this.scheme = this.value.protocol.slice(0, -1);
    this.fsPath = this.scheme === "file" ? fileURLToPath(this.value) : "";
  }
  static file(value) { return new Uri(pathToFileURL(value).href); }
  static parse(value) { return new Uri(value); }
  toString() { return this.value.href; }
}

/** An event registration whose callbacks the host can fire. */
function event(name) {
  return (callback) => {
    const set = listeners.get(name) ?? new Set();
    listeners.set(name, set);
    set.add(callback);
    return { dispose: () => set.delete(callback) };
  };
}

/** Records a message and answers it with the configured action when offered. */
function message(kind) {
  return async (text, ...items) => {
    const labels = items.filter((item) => typeof item === "string");
    messages.push({ kind, text: String(text), items: labels });
    return scenario.choose && labels.includes(scenario.choose) ? scenario.choose : undefined;
  };
}

const document = {
  uri: Uri.file(scenario.entry), fileName: scenario.entry, languageId: "clorraine",
  isUntitled: false, isDirty: false, version: 1,
  getText: () => fs.readFileSync(scenario.entry, "utf8"),
  async save() { return true; },
};
const folder = { uri: Uri.file(scenario.workspace), name: "sources", index: 0 };
const noop = () => ({ dispose() {} });
class Task { constructor(definition, scope, name, source, execution) { Object.assign(this, { definition, scope, name, source, execution }); } }
class ProcessExecution { constructor(process, args, options) { Object.assign(this, { process, args, options }); } }
const vscode = {
  Uri, Task, ProcessExecution,
  TaskScope: { Global: 1, Workspace: 2 }, TaskRevealKind: { Always: 1 }, TaskPanelKind: { Dedicated: 2 },
  DiagnosticSeverity: { Error: 0, Warning: 1, Information: 2, Hint: 3 },
  Position: class { constructor(line, character) { Object.assign(this, { line, character }); } },
  Range: class { constructor(...args) { this.args = args; } },
  Diagnostic: class { constructor(range, text, severity) { Object.assign(this, { range, message: text, severity }); } },
  workspace: {
    isTrusted: true, workspaceFolders: [folder], textDocuments: [document],
    getWorkspaceFolder: () => folder,
    getConfiguration: () => ({
      inspect: (key) => ({ key: `clorraine.${key}`, defaultValue: "",
        ...(scenario.configuration[key] !== undefined ? { globalValue: scenario.configuration[key] } : {}) }),
      get: (key, fallback) => scenario.configuration[key] ?? fallback,
    }),
    onDidChangeConfiguration: event("configuration"), onDidGrantWorkspaceTrust: event("trust"),
    onDidOpenTextDocument: event("open"), onDidChangeTextDocument: event("change"),
    onDidCloseTextDocument: event("close"), onDidSaveTextDocument: event("save"),
  },
  window: {
    activeTextEditor: { document },
    showErrorMessage: message("error"), showWarningMessage: message("warning"),
    showInformationMessage: message("information"),
  },
  env: { async openExternal(uri) { opened.push(uri.toString()); return true; } },
  languages: {
    createDiagnosticCollection: () => ({ set() {}, clear() {}, delete() {}, dispose() {} }),
    registerHoverProvider: noop, registerDefinitionProvider: noop,
    registerCompletionItemProvider: noop, registerDocumentSymbolProvider: noop,
  },
  commands: {
    registerCommand(id, callback) { commands.set(id, callback); return { dispose: () => commands.delete(id) }; },
    async executeCommand(id, ...args) { return commands.get(id)?.(...args); },
  },
  tasks: {
    async executeTask(task) { return { task, terminate() {} }; },
    onDidEndTask: event("taskEnd"), onDidEndTaskProcess: event("taskProcessEnd"),
  },
};

/** Number of initialize requests the peer server has seen so far. */
function starts() {
  return fs.existsSync(process.env.GUIDE_EVENTS) ?
    fs.readFileSync(process.env.GUIDE_EVENTS, "utf8").split("\n").filter(Boolean).length : 0;
}

async function main() {
  globalThis.__guideVscode = vscode;
  const hook = registerHooks({
    resolve: (specifier, context, next) =>
      specifier === "vscode" ? { url: "guide:vscode", shortCircuit: true } : next(specifier, context),
    load: (url, context, next) => url === "guide:vscode" ?
      { format: "commonjs", source: "module.exports = globalThis.__guideVscode;", shortCircuit: true } :
      next(url, context),
  });
  const extension = require(scenario.extension);
  const root = path.dirname(path.dirname(scenario.extension));
  const context = {
    subscriptions: [], extensionPath: root, extensionUri: Uri.file(root),
    extension: { id: "openlorraine.clorraine", packageJSON: JSON.parse(fs.readFileSync(scenario.packageJson, "utf8")) },
  };
  try {
    await extension.activate(context);
    await settle(() => starts() >= 1 || messages.length > 0);
    if (scenario.restart) {
      for (const callback of listeners.get("configuration") ?? []) {
        await callback({ affectsConfiguration: (section) => section === "clorraine" || section === "clorraine.profile" });
      }
      await settle(() => starts() >= 2);
    }
    for (const id of scenario.commands ?? []) {
      const callback = commands.get(id);
      results[id] = callback ? { registered: true, value: (await callback()) ?? null } : { registered: false };
    }
    await settle(() => false, 200);
    return { messages, opened, results, starts: starts(), commands: [...commands.keys()] };
  } finally {
    await extension.deactivate();
    for (const disposable of context.subscriptions) disposable.dispose();
    hook.deregister();
  }
}

main().then((result) => {
  process.stdout.write(`GUIDE_OBSERVATION=${JSON.stringify(result)}\n`);
  process.exit(0);
}, (error) => {
  console.error(error);
  process.exit(1);
});
