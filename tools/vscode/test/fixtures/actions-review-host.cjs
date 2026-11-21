/**
 * Public editor-event and process-task observation only. The unchanged framed
 * peer controls when selection returns; no compiler outcome is substituted.
 */
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { registerHooks } = require("node:module");
const { pathToFileURL } = require("node:url");

const config = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const commands = new Map();
const changeListeners = new Set();
const saveListeners = new Set();
const endListeners = new Set();
const messages = [];
const tasks = [];
const emitted = [];
const context = { subscriptions: [] };

/** Match VS Code's file URI boundary without interpreting literal path bytes. */
function fileUri(file) {
  return { scheme: "file", fsPath: file, toString: () => pathToFileURL(file).href };
}

/** Observe both registration and disposal through the public event contract. */
function subscribe(listeners, callback) {
  listeners.add(callback);
  return { dispose: () => listeners.delete(callback) };
}

/** A real edit changes version; a save cleans it before delivering the event. */
function document(file) {
  return {
    uri: fileUri(file), fileName: file, isUntitled: false, isDirty: false, version: 1,
    async save() {
      this.isDirty = false;
      for (const callback of saveListeners) callback(this);
      return true;
    },
  };
}

const entry = document(config.entry);
const dependency = document(config.files[1]);
const folder = { uri: fileUri(config.root), name: "selected", index: 0 };
const vscode = {
  Uri: { file: fileUri },
  workspace: {
    isTrusted: true, textDocuments: [entry, dependency],
    getWorkspaceFolder: () => folder,
    getConfiguration: () => ({
      get: (key) => key === "profile" ? config.profile : (config.settings[key] ?? ""),
      inspect: (key) => ({ globalValue: config.settings[key] ?? "", defaultValue: "" }),
    }),
    onDidChangeTextDocument: (callback) => subscribe(changeListeners, callback),
    onDidSaveTextDocument: (callback) => subscribe(saveListeners, callback),
  },
  window: {
    activeTextEditor: { document: entry },
    showErrorMessage: (message) => { messages.push(message); },
    showWarningMessage: (message) => { messages.push(message); },
    showInformationMessage: (message) => { messages.push(message); },
  },
  commands: {
    registerCommand(id, callback) {
      assert.ok(!commands.has(id));
      commands.set(id, callback);
      return { dispose: () => commands.delete(id) };
    },
  },
  tasks: {
    onDidEndTask: (callback) => subscribe(endListeners, callback),
    async executeTask(task) {
      tasks.push({
        process: task.execution.process, args: task.execution.args,
        cwd: task.execution.options.cwd,
      });
      return { terminate() {} };
    },
  },
  Task: class {
    constructor(definition, scope, name, source, execution) {
      Object.assign(this, { definition, scope, name, source, execution });
    }
  },
  ProcessExecution: class {
    constructor(process, args, options) { Object.assign(this, { process, args, options }); }
  },
  TaskScope: { Global: 1 }, TaskRevealKind: { Always: 1 }, TaskPanelKind: { Dedicated: 2 },
};

globalThis.__reviewEditor = vscode;
const hook = registerHooks({
  resolve(specifier, context, next) {
    return specifier === "vscode" ? { url: "review:vscode", shortCircuit: true } :
      next(specifier, context);
  },
  load(url, context, next) {
    return url === "review:vscode" ? {
      format: "commonjs", source: "module.exports = globalThis.__reviewEditor;",
      shortCircuit: true,
    } : next(url, context);
  },
});

/** Remove only registrations owned by this activation. */
function dispose() {
  for (const item of context.subscriptions.splice(0)) item.dispose();
}

/** Observe real peer traffic with a finite readiness deadline. */
async function selectionArrived() {
  const deadline = Date.now() + 2500;
  for (;;) {
    const events = fs.readFileSync(config.events, "utf8").split("\n").filter(Boolean).map(JSON.parse);
    if (events.some((event) => event.method === "clorraine/sourceFiles")) return;
    assert.ok(Date.now() < deadline, "real selection request did not arrive");
    await new Promise((done) => setTimeout(done, 5));
  }
}

/** Deliver actual documented event shapes while the server reply is withheld. */
async function mutate() {
  const target = config.event === "entry" ? entry :
    config.event === "alias" ? document(config.aliasDependency) :
    config.event === "outside" ? document(config.outside) :
    config.event === "nonclo" ? document(config.nonclo) : dependency;
  if (config.event !== "save-only") {
    target.version += 1;
    target.isDirty = true;
    fs.appendFileSync(target.uri.fsPath, "\n");
    for (const callback of changeListeners) callback({ document: target, contentChanges: [] });
    emitted.push("change");
  }
  await target.save();
  emitted.push("save");
}

/** Exercise only exported action registration and real client transport. */
async function main() {
  const peerConfig = path.join(config.scratch, "peer.json");
  fs.writeFileSync(config.events, "");
  fs.writeFileSync(peerConfig, JSON.stringify({
    events: config.events, release: config.release, files: config.files,
    pendingChange: true, selectionFailure: config.failure === true,
  }));
  const { LspClient } = require(path.join(config.staged, "client.js"));
  const client = new LspClient({
    command: process.execPath, args: [path.join(__dirname, "action-lsp.cjs")],
    env: { ...process.env, ACTION_LSP_CONFIG: peerConfig },
  });
  let connection = { client, sourceFilesVersion: 1 };
  try {
    await client.start();
    const { registerActions } = require(path.join(config.staged, "actions.js"));
    registerActions(context, () => connection);
    const action = commands.get(config.command);
    assert.equal(typeof action, "function", "public command must be registered");
    const running = action();
    let during;
    // Task-input rejection may precede selection, so do not demand a request
    // in those scenarios. Ordinary event cases always reach the framed barrier.
    if (config.input) {
      fs.writeFileSync(config.release, "");
    } else {
      await selectionArrived();
      during = { change: changeListeners.size, save: saveListeners.size };
      if (config.event) await mutate();
      if (config.dirtyDependency) dependency.isDirty = true;
      if (config.dirtyEntry) entry.isDirty = true;
      if (config.sessionChange) connection = { client, sourceFilesVersion: 1 };
      if (config.disposePending) dispose();
      fs.writeFileSync(config.release, "");
    }
    await running;
    const after = { change: changeListeners.size, save: saveListeners.size };
    const finalDependency = { dirty: dependency.isDirty, version: dependency.version };
    dispose();
    return { tasks, messages, emitted, during, after, finalDependency,
      disposed: { change: changeListeners.size, save: saveListeners.size },
      commandsAfterDispose: commands.size };
  } finally {
    dispose();
    await client.stop();
  }
}

main().then((value) => process.stdout.write(JSON.stringify(value) + "\n"), (error) => {
  console.error(error);
  process.exitCode = 1;
}).finally(() => hook.deregister());
