/**
 * Isolated external VS Code host for the compiled extension's public lifecycle.
 * Only the VS Code module and observable filesystem/process boundaries are
 * replaced or observed. The client, discovery and actions run unchanged.
 */
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const childProcess = require("node:child_process");
const { registerHooks } = require("node:module");
const { fileURLToPath, pathToFileURL } = require("node:url");

const config = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const commands = new Map();
const messages = [];
const tasks = [];
const saves = [];
const discovery = [];
const providers = [];
const globalState = new Map();
const children = [];
const listeners = new Map();
let saveAllCalls = 0;

/** Read only fixture observations, never compiled extension source. */
function events() {
  return fs.readFileSync(config.events, "utf8").trim().split("\n")
    .filter(Boolean).map((line) => JSON.parse(line));
}

/** Wait for a bounded external protocol observation, not a guessed startup delay. */
async function until(predicate, description) {
  const end = Date.now() + 2000;
  while (!predicate()) {
    assert.ok(Date.now() < end, `fixture timed out: ${description}`);
    await new Promise((resolve) => setTimeout(resolve, 5));
  }
}

/** Minimal URI value with the public file and parsing operations. */
class Uri {
  constructor(value) {
    this.value = new URL(value);
    this.scheme = this.value.protocol.slice(0, -1);
    this.fsPath = this.scheme === "file" ? fileURLToPath(this.value) : "";
    this.path = decodeURIComponent(this.value.pathname);
  }
  static file(value) { return new Uri(pathToFileURL(value).href); }
  static parse(value) { return new Uri(value); }
  static joinPath(base, ...parts) { return Uri.file(path.join(base.fsPath, ...parts)); }
  toString() { return this.value.href; }
}

/** Subscribe and dispose external editor events without invoking extension internals. */
function event(name) {
  return (callback) => {
    const callbacks = listeners.get(name) ?? new Set();
    listeners.set(name, callbacks);
    callbacks.add(callback);
    return { dispose: () => callbacks.delete(callback) };
  };
}

/** Notify registered public event listeners about a fixture editor change. */
function emit(name, value) {
  for (const callback of listeners.get(name) ?? []) callback(value);
}

/** A saved document exposes only the editor operations used by these scenarios. */
function document(file, dirty = false) {
  const text = fs.readFileSync(file, "utf8") + (dirty ? "\n// Unsaved editor change.\n" : "");
  return {
    uri: Uri.file(file), fileName: file, languageId: "clorraine",
    isDirty: dirty, isUntitled: false, version: 1,
    getText: () => text,
    async save() {
      saves.push(file);
      fs.appendFileSync(config.events, `${JSON.stringify({ method: "fixture/save", file })}\n`);
      if (config.save === "throw") throw new Error("Save failed: fixture disk error");
      if (config.save === "cancel") return false;
      fs.writeFileSync(file, text);
      this.isDirty = false;
      emit("save", this);
      return true;
    },
  };
}

const entry = document(config.entry, config.dirtyEntry);
const dependency = document(config.files[1], config.dirtyDependency);
const unrelated = document(config.unrelated, config.dirtyUnrelated);
if (config.invalid === "untitled") {
  entry.uri = Uri.parse("untitled:main.clo");
  entry.isUntitled = true;
}
const folders = config.roots.map((root, index) => ({
  uri: Uri.file(root), name: path.basename(root), index,
}));
const settings = {
  "lsp.path": config.server, "driver.path": config.driver,
  ...(config.profile === undefined ? {} : { profile: config.profile }),
  ...(config.settings ?? {}),
};
const defaults = { profile: "hosted-amiga", "toolchain.path": "", "sdk.path": "", "fsUae.path": "", "kickstart.path": "" };

/** Preserve literal argv and options at the real task API boundary. */
class ProcessExecution {
  constructor(process, args = [], options) {
    this.process = process;
    this.args = Array.isArray(args) ? args : [];
    this.options = Array.isArray(args) ? options : args;
  }
}

/** Shell tasks remain distinguishable, so unsafe construction fails assertions. */
class ShellExecution {
  constructor(...args) { this.args = args; }
}

/** Model the documented task constructor, not an extension-specific task helper. */
class Task {
  constructor(definition, scope, name, source, execution, problemMatchers = []) {
    Object.assign(this, { definition, scope, name, source, execution, problemMatchers });
    this.presentationOptions = {};
  }
}

/** Capture genuine UI messages without selecting a trust or save action for the user. */
async function message(text) {
  messages.push(String(text));
  return undefined;
}

/** Register a language feature as a startup observation; do not fake its result. */
function provider(kind) {
  return () => {
    providers.push(kind);
    return { dispose() {} };
  };
}

const diagnostics = { set() {}, delete() {}, clear() {}, dispose() {} };
const output = { append() {}, appendLine() {}, show() {}, hide() {}, dispose() {} };
const vscode = {
  Uri, ProcessExecution, ShellExecution, Task,
  TaskScope: { Global: 1, Workspace: 2 },
  TaskRevealKind: { Always: 1, Silent: 2, Never: 3 },
  TaskPanelKind: { Shared: 1, Dedicated: 2, New: 3 },
  TaskGroup: { Build: { id: "build" } },
  DiagnosticSeverity: { Error: 0, Warning: 1, Information: 2, Hint: 3 },
  workspace: {
    isTrusted: config.trusted !== false,
    workspaceFolders: folders,
    textDocuments: [entry, dependency, unrelated],
    getWorkspaceFolder(uri) {
      return folders.filter((folder) => uri.fsPath === folder.uri.fsPath ||
        uri.fsPath.startsWith(`${folder.uri.fsPath}${path.sep}`))
        .sort((a, b) => b.uri.fsPath.length - a.uri.fsPath.length)[0];
    },
    getConfiguration(section) {
      const key = (name) => (section ? `${section}.${name}` : name).replace(/^clorraine\./, "");
      return {
        get: (name, fallback) => settings[key(name)] ?? defaults[key(name)] ?? fallback,
        inspect: (name) => ({
          key: `clorraine.${key(name)}`, globalValue: settings[key(name)],
          defaultValue: defaults[key(name)],
          workspaceValue: config.workspaceOverrides?.[key(name)],
        }),
      };
    },
    async saveAll() { saveAllCalls += 1; return true; },
    fs: {
      stat: (uri) => fs.promises.stat(uri.fsPath),
      readFile: (uri) => fs.promises.readFile(uri.fsPath),
    },
    onDidOpenTextDocument: event("open"), onDidChangeTextDocument: event("change"),
    onDidCloseTextDocument: event("close"), onDidSaveTextDocument: event("save"),
    onDidChangeConfiguration: event("configuration"),
    onDidChangeWorkspaceFolders: event("folders"),
    onDidGrantWorkspaceTrust: event("trust"),
  },
  window: {
    activeTextEditor: config.invalid === "no-editor" ? undefined : { document: entry },
    showErrorMessage: message, showWarningMessage: message, showInformationMessage: message,
    createOutputChannel: () => output,
    onDidChangeActiveTextEditor: event("editor"),
  },
  languages: {
    createDiagnosticCollection: () => diagnostics,
    registerHoverProvider: provider("hover"),
    registerDefinitionProvider: provider("definition"),
    registerCompletionItemProvider: provider("completion"),
    registerDocumentSymbolProvider: provider("symbols"),
  },
  commands: {
    registerCommand(id, callback) {
      assert.ok(!commands.has(id), `duplicate command ${id}`);
      commands.set(id, callback);
      return { dispose: () => commands.delete(id) };
    },
    async executeCommand(id, ...args) { return commands.get(id)?.(...args); },
  },
  tasks: {
    async executeTask(task) {
      tasks.push({
        name: task.name, detail: task.detail,
        kind: task.execution instanceof ProcessExecution ? "process" : "shell",
        process: task.execution?.process, args: task.execution?.args,
        cwd: task.execution?.options?.cwd, presentation: task.presentationOptions,
      });
      return { task, terminate() {} };
    },
    onDidEndTaskProcess: event("taskProcessEnd"), onDidEndTask: event("taskEnd"),
  },
};

/**
 * Observe executable lookups at filesystem/process boundaries. Safe explicit
 * non-workspace LSP startup is permitted, but workspace candidate probes are not
 * hidden by the fixture. Wrappers always delegate to the real external API.
 */
function observeDiscovery() {
  const observe = (value) => {
    if (value instanceof URL) value = fileURLToPath(value);
    if (Buffer.isBuffer(value)) value = value.toString();
    if (typeof value === "string" && config.roots.some((root) =>
      value.includes(`${root}${path.sep}`)) &&
      /(?:target(?:[\\/]|$)|workspace-tools|clorraine(?:-lsp)?$)/.test(value)) {
      discovery.push(value);
    }
  };
  for (const name of ["existsSync", "accessSync", "statSync", "lstatSync", "realpathSync",
    "readdirSync", "readFileSync", "openSync", "access", "stat", "lstat", "realpath", "readdir", "readFile", "open"]) {
    const original = fs[name];
    fs[name] = (...args) => { observe(args[0]); return original(...args); };
    if (original.native) {
      fs[name].native = (...args) => { observe(args[0]); return original.native(...args); };
    }
  }
  for (const name of ["access", "stat", "lstat", "realpath", "readdir", "readFile", "open"]) {
    const original = fs.promises[name];
    fs.promises[name] = (...args) => { observe(args[0]); return original(...args); };
  }
  for (const name of ["spawn", "execFile", "spawnSync", "execFileSync", "exec", "execSync"]) {
    const original = childProcess[name];
    childProcess[name] = (...args) => {
      observe(args[0]);
      if (Array.isArray(args[1])) args[1].forEach(observe);
      const child = original(...args);
      if (name === "spawn" || name === "execFile" || name === "exec") children.push(child);
      return child;
    };
  }
}

/** Run one scenario; a missing command is returned as a contract failure, not a fixture error. */
async function main() {
  observeDiscovery();
  globalThis.__actionTestVscode = vscode;
  const hook = registerHooks({
    resolve(specifier, context, next) {
      if (specifier === "vscode") return { url: "action-fixture:vscode", shortCircuit: true };
      return next(specifier, context);
    },
    load(url, context, next) {
      if (url === "action-fixture:vscode") return {
        format: "commonjs", source: "module.exports = globalThis.__actionTestVscode;",
        shortCircuit: true,
      };
      return next(url, context);
    },
  });
  const extension = require(config.extension);
  const context = {
    subscriptions: [], extensionPath: path.dirname(path.dirname(config.extension)),
    extensionUri: Uri.file(path.dirname(path.dirname(config.extension))),
    globalState: {
      get(key, fallback) { return globalState.has(key) ? globalState.get(key) : fallback; },
      async update(key, value) { globalState.set(key, value); },
    },
  };
  let startup;
  try {
    await extension.activate(context);
    // Restricted mode may omit even safe LSP startup. Only trusted activation
    // requires this control; no workspace executable lookup is exempted.
    if (config.trusted !== false) {
      await until(() => events().some((item) => item.method === "initialized"), "LSP initialized");
    }
    startup = { initialized: events().some((item) => item.method === "initialized"), providers: [...providers] };
    const registered = [...commands.keys()];
    const callback = commands.get(config.command);
    if (callback) {
      const running = Promise.resolve().then(() => callback());
      if (config.pendingChange) {
        await until(() => events().some((item) => item.method === "clorraine/sourceFiles"),
          "selection pending");
        if (config.pendingChange === "dependency") dependency.isDirty = true;
        if (config.pendingChange === "entry-dirty") entry.isDirty = true;
        if (config.pendingChange === "editor") {
          vscode.window.activeTextEditor = { document: unrelated };
          emit("editor", vscode.window.activeTextEditor);
        }
        fs.writeFileSync(config.release, "");
      }
      await running;
      if (config.repeat) {
        await Promise.resolve().then(() => callback());
      }
    }
    return { startup, commands: registered, messages, tasks, saves, saveAllCalls,
      discovery, events: events(), unrelatedDirty: unrelated.isDirty };
  } finally {
    await extension.deactivate();
    for (const disposable of context.subscriptions) disposable.dispose();
    hook.deregister();
  }
}

main().then((result) => {
  process.stdout.write(`ACTION_OBSERVATION=${JSON.stringify(result)}\n`);
}, (error) => {
  console.error(error);
  process.exitCode = 1;
}).finally(() => {
  for (const child of children) {
    if (child.exitCode === null && child.signalCode === null) child.kill("SIGKILL");
  }
});
