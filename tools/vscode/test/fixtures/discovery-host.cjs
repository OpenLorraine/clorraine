/**
 * External editor facade for the staged extension's public lifecycle. Configuration
 * inspection retains each VS Code level; merged get() values intentionally expose
 * hostile workspace values so safe executable selection must use its real policy.
 * No extension modules or functions are replaced.
 */
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const cp = require("node:child_process");
const { registerHooks } = require("node:module");
const { fileURLToPath, pathToFileURL } = require("node:url");
const config = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const commands = new Map();
const listeners = new Map();
const children = [];
const messages = [];
const tasks = [];
const diagnostics = [];
const probes = [];
const providers = [];
const configuration = config.configuration;

/** Read observation records only, never generated production bodies. */
function events() {
  return fs.readFileSync(config.events, "utf8").trim().split("\n").filter(Boolean).map(JSON.parse);
}

/**
 * Wait for a public observation, but return false on absence. Missing product
 * behavior belongs in specification assertions, not a fixture-timeout error.
 */
async function settle(predicate) {
  const end = Date.now() + 900;
  while (!predicate() && Date.now() < end) await new Promise((resolve) => setTimeout(resolve, 5));
  return predicate();
}

/** Public file URI operations used by the editor and task APIs. */
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

/** Keep subscriptions independently disposable across activation cycles. */
function event(name) {
  return (callback) => {
    const callbacks = listeners.get(name) ?? new Set();
    listeners.set(name, callbacks);
    callbacks.add(callback);
    return { dispose: () => callbacks.delete(callback) };
  };
}

/** Dispatch the same public events an editor configuration change would emit. */
async function emit(name, value) {
  await Promise.all([...(listeners.get(name) ?? [])].map((callback) => callback(value)));
}

/** Capture a direct process task without executing an editor-specific runner. */
class ProcessExecution {
  constructor(process, args = [], options) {
    this.process = process;
    this.args = Array.isArray(args) ? args : [];
    this.options = Array.isArray(args) ? options : args;
  }
}

/** Unsafe shell execution remains visible at the true external task boundary. */
class ShellExecution {
  constructor(...args) { this.args = args; }
}

/** Public VS Code task value; presentation defaults remain the editor's defaults. */
class Task {
  constructor(definition, scope, name, source, execution, problemMatchers = []) {
    Object.assign(this, { definition, scope, name, source, execution, problemMatchers });
    this.presentationOptions = {};
  }
}

/** Return inspect fields and normal documented configuration precedence. */
function getConfiguration(section) {
  const key = (name) => (section ? `${section}.${name}` : name).replace(/^clorraine\./, "");
  return {
    inspect(name) { return { key: `clorraine.${key(name)}`, ...configuration[key(name)] }; },
    get(name, fallback) {
      const values = configuration[key(name)] ?? {};
      for (const level of ["workspaceFolderLanguageValue", "workspaceLanguageValue", "globalLanguageValue",
        "workspaceFolderValue", "workspaceValue", "globalValue", "defaultLanguageValue", "defaultValue"]) {
        if (values[level] !== undefined) return values[level];
      }
      return fallback;
    },
  };
}

/** Capture UI output without accepting user actions or changing trust. */
async function message(value) { messages.push(String(value)); return undefined; }

/** Register a language provider as an external startup control. */
function provider(kind) {
  return () => { providers.push(kind); return { dispose() {} }; };
}

const document = {
  uri: Uri.file(config.entry), fileName: config.entry, languageId: "clorraine",
  isUntitled: false, isDirty: false, version: 1,
  getText: () => fs.readFileSync(config.entry, "utf8"),
  async save() { return true; },
};
const folder = { uri: Uri.file(config.workspace), name: "sources", index: 0 };
const output = { append() {}, appendLine() {}, show() {}, hide() {}, dispose() {} };
const vscode = {
  Uri, ProcessExecution, ShellExecution, Task,
  TaskScope: { Global: 1, Workspace: 2 },
  TaskRevealKind: { Always: 1, Silent: 2, Never: 3 },
  TaskPanelKind: { Shared: 1, Dedicated: 2, New: 3 },
  TaskGroup: { Build: { id: "build" } },
  DiagnosticSeverity: { Error: 0, Warning: 1, Information: 2, Hint: 3 },
  Position: class Position { constructor(line, character) { Object.assign(this, { line, character }); } },
  Range: class Range { constructor(...args) { this.args = args; } },
  Diagnostic: class Diagnostic {
    constructor(range, message, severity) { Object.assign(this, { range, message, severity }); }
  },
  workspace: {
    isTrusted: config.trusted !== false, workspaceFolders: [folder], textDocuments: [document],
    getWorkspaceFolder(uri) { return uri.fsPath.startsWith(`${config.workspace}${path.sep}`) ? folder : undefined; },
    getConfiguration,
    fs: { stat: (uri) => fs.promises.stat(uri.fsPath), readFile: (uri) => fs.promises.readFile(uri.fsPath) },
    onDidChangeConfiguration: event("configuration"), onDidGrantWorkspaceTrust: event("trust"),
    onDidOpenTextDocument: event("open"), onDidChangeTextDocument: event("change"),
    onDidCloseTextDocument: event("close"), onDidSaveTextDocument: event("save"),
    onDidChangeWorkspaceFolders: event("folders"),
  },
  window: {
    activeTextEditor: { document }, createOutputChannel: () => output,
    showErrorMessage: message, showWarningMessage: message, showInformationMessage: message,
    onDidChangeActiveTextEditor: event("editor"),
  },
  languages: {
    createDiagnosticCollection: () => ({
      set(uri, values) { diagnostics.push(...values.map((value) => value.message)); },
      clear() {}, delete() {}, dispose() {},
    }),
    registerHoverProvider: provider("hover"), registerDefinitionProvider: provider("definition"),
    registerCompletionItemProvider: provider("completion"), registerDocumentSymbolProvider: provider("symbols"),
  },
  commands: {
    registerCommand(id, callback) {
      assert.ok(!commands.has(id), `duplicate live command registration: ${id}`);
      commands.set(id, callback);
      return { dispose: () => commands.delete(id) };
    },
    async executeCommand(id, ...args) { return commands.get(id)?.(...args); },
  },
  tasks: {
    async executeTask(task) {
      const execution = task.execution;
      // A version probe at the task boundary proves executable identity, not a
      // successful build/run. Every candidate has the same version output.
      const before = events().length;
      const probe = execution instanceof ProcessExecution ? cp.spawnSync(execution.process, ["--version"], {
        encoding: "utf8", timeout: 1000, maxBuffer: 65536, env: process.env,
      }) : undefined;
      tasks.push({ kind: execution instanceof ProcessExecution ? "process" : "shell",
        process: execution?.process, args: execution?.args, probeStatus: probe?.status,
        identities: events().slice(before).filter((item) => item.kind === "exec").map((item) => item.executable) });
      return { task, terminate() {} };
    },
    onDidEndTaskProcess: event("taskProcessEnd"), onDidEndTask: event("taskEnd"),
  },
};

/**
 * Observe real workspace candidate access and subprocesses. Calls always reach
 * the original external API; the observer never chooses a tool for production.
 */
function observe() {
  const inspect = (value) => {
    if (value instanceof URL) value = fileURLToPath(value);
    if (Buffer.isBuffer(value)) value = value.toString();
    if (typeof value === "string" && value.startsWith(`${config.workspace}${path.sep}`) &&
      /target|workspace-tools/.test(value)) probes.push(value);
  };
  for (const name of ["existsSync", "accessSync", "statSync", "lstatSync", "realpathSync",
    "readdirSync", "readFileSync", "openSync", "access", "stat", "lstat", "realpath", "readdir", "readFile", "open"]) {
    const original = fs[name];
    fs[name] = (...args) => { inspect(args[0]); return original(...args); };
    if (original.native) fs[name].native = (...args) => { inspect(args[0]); return original.native(...args); };
  }
  for (const name of ["access", "stat", "lstat", "realpath", "readdir", "readFile", "open"]) {
    const original = fs.promises[name];
    fs.promises[name] = (...args) => { inspect(args[0]); return original(...args); };
  }
  for (const name of ["spawn", "execFile", "spawnSync", "execFileSync", "exec", "execSync"]) {
    const original = cp[name];
    cp[name] = (...args) => {
      inspect(args[0]);
      if (Array.isArray(args[1])) args[1].forEach(inspect);
      const child = original(...args);
      if (["spawn", "execFile", "exec"].includes(name)) children.push(child);
      return child;
    };
  }
}

/** Activate only the public entry point and collect externally observable state. */
async function main() {
  observe();
  globalThis.__discoveryVscode = vscode;
  const hook = registerHooks({
    resolve(specifier, context, next) {
      return specifier === "vscode" ? { url: "discovery:vscode", shortCircuit: true } : next(specifier, context);
    },
    load(url, context, next) {
      return url === "discovery:vscode" ? {
        format: "commonjs", source: "module.exports = globalThis.__discoveryVscode;", shortCircuit: true,
      } : next(url, context);
    },
  });
  const extension = require(config.extension);
  const contexts = [];
  const activate = async () => {
    const context = { subscriptions: [], extensionPath: path.dirname(path.dirname(config.extension)),
      extensionUri: Uri.file(path.dirname(path.dirname(config.extension))) };
    contexts.push(context);
    await extension.activate(context);
    await settle(() => events().filter((item) => item.method === "initialized").length >= contexts.length);
    return context;
  };
  let disposedCommands;
  try {
    const first = await activate();
    const startup = { initialized: events().some((item) => item.method === "initialized"), providers: [...providers] };
    if (config.nextProfile !== undefined) {
      configuration.profile.globalValue = config.nextProfile;
      await emit("configuration", { affectsConfiguration: (section) =>
        section === "clorraine" || section === "clorraine.profile" });
      await settle(() => diagnostics.includes(`profile:${config.nextProfile}`));
    }
    if (config.lifecycle) {
      await extension.deactivate();
      for (const disposable of first.subscriptions) disposable.dispose();
      disposedCommands = [...commands.keys()];
      await activate();
    }
    const registered = [...commands.keys()];
    if (config.command && commands.has(config.command)) await commands.get(config.command)();
    await settle(() => diagnostics.length > 0 || !startup.initialized);
    return { startup, commands: registered, disposedCommands, messages, tasks,
      diagnostics, probes, events: events() };
  } finally {
    await extension.deactivate();
    for (const context of contexts) for (const disposable of context.subscriptions) disposable.dispose();
    hook.deregister();
  }
}

main().then((result) => {
  process.stdout.write(`DISCOVERY_OBSERVATION=${JSON.stringify(result)}\n`);
}, (error) => {
  console.error(error);
  process.exitCode = 1;
}).finally(async () => {
  for (const child of children) {
    if (child.exitCode === null && child.signalCode === null) {
      child.kill("SIGKILL");
      await new Promise((resolve) => child.once("exit", resolve));
    }
  }
});
