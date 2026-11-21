/**
 * Isolated implementation-test editor boundary. Real client/peer framing is
 * retained; only VS Code events and task-submission completion are controlled.
 */
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { registerHooks } = require("node:module");
const { pathToFileURL } = require("node:url");

const config = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));

/** A manually released API promise makes disposal races deterministic. */
function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

/** Bound observation waits by a deadline instead of assuming a child is ready. */
async function until(predicate) {
  const deadline = Date.now() + 2000;
  while (!predicate()) {
    assert.ok(Date.now() < deadline, "implementation fixture observation timed out");
    await new Promise((done) => setTimeout(done, 5));
  }
}

/** Match the small URI surface used by the unchanged action adapter. */
function fileUri(file) {
  return { scheme: "file", fsPath: file, toString: () => pathToFileURL(file).href };
}

const commands = new Map();
const endListeners = new Set();
const executionGate = deferred();
const saveGate = deferred();
const errors = [];
let tasks = 0;
let terminated = 0;
let unrelatedTerminated = 0;
let saveStarted = false;
const execution = { terminate: () => { terminated += 1; } };
const unrelated = { terminate: () => { unrelatedTerminated += 1; } };
const document = {
  uri: fileUri(config.entry), isUntitled: false, version: 1,
  isDirty: config.scenario === "pending-save",
  async save() {
    saveStarted = true;
    await saveGate.promise;
    this.isDirty = false;
    return true;
  },
};
const folder = { uri: fileUri(config.scratch), name: "owned", index: 0 };
const vscode = {
  Uri: { file: fileUri },
  workspace: {
    isTrusted: true, textDocuments: [document],
    onDidChangeTextDocument: () => ({ dispose() {} }),
    onDidSaveTextDocument: () => ({ dispose() {} }),
    getWorkspaceFolder: () => folder,
    getConfiguration: () => ({
      get: () => "host",
      inspect: () => config.mode === "setting" ? config.fields : {
        globalValue: process.execPath, defaultValue: "",
      },
    }),
  },
  window: {
    activeTextEditor: { document },
    showErrorMessage: (message) => { errors.push(message); },
  },
  commands: {
    registerCommand(id, callback) {
      assert.ok(!commands.has(id), `duplicate live command ${id}`);
      commands.set(id, callback);
      return { dispose: () => commands.delete(id) };
    },
  },
  tasks: {
    onDidEndTask(callback) {
      endListeners.add(callback);
      return { dispose: () => endListeners.delete(callback) };
    },
    async executeTask() {
      tasks += 1;
      if (config.scenario.startsWith("pending-execution")) await executionGate.promise;
      return execution;
    },
  },
  Task: class { constructor(...args) { this.arguments = args; } },
  ProcessExecution: class { constructor(...args) { this.arguments = args; } },
  TaskScope: { Global: 1 },
  TaskRevealKind: { Always: 1 },
  TaskPanelKind: { Dedicated: 2 },
};

globalThis.__ownershipEditor = vscode;
const hook = registerHooks({
  resolve(specifier, context, next) {
    return specifier === "vscode" ? { url: "ownership:vscode", shortCircuit: true } :
      next(specifier, context);
  },
  load(url, context, next) {
    return url === "ownership:vscode" ? {
      format: "commonjs", source: "module.exports = globalThis.__ownershipEditor;",
      shortCircuit: true,
    } : next(url, context);
  },
});

/** Dispose the real registered callbacks, not a private extension cleanup method. */
function dispose(context) {
  for (const item of context.subscriptions.splice(0)) item.dispose();
}

/** Read only the real peer's protocol observations. */
function events() {
  return fs.readFileSync(config.events, "utf8").split("\n").filter(Boolean).map(JSON.parse);
}

/** Exercise the exported adapter against actual framing and controlled editor events. */
async function main() {
  if (config.mode === "setting") {
    const { machinePath } = require(path.join(config.staged, "discovery.js"));
    try {
      return { value: machinePath("driver.path") };
    } catch (error) {
      return { error: error.message };
    }
  }
  config.events = path.join(config.scratch, "events.jsonl");
  config.release = path.join(config.scratch, "release");
  fs.writeFileSync(config.events, "");
  const peerConfig = path.join(config.scratch, "peer.json");
  fs.writeFileSync(peerConfig, JSON.stringify({
    events: config.events, release: config.release, files: [config.entry],
    pendingChange: ["pending-selection", "rejected-selection"].includes(config.scenario),
    selectionFailure: config.scenario === "rejected-selection",
  }));
  const { LspClient } = require(path.join(config.staged, "client.js"));
  const client = new LspClient({
    command: process.execPath, args: [path.join(__dirname, "action-lsp.cjs")],
    env: { ...process.env, ACTION_LSP_CONFIG: peerConfig },
  });
  let context = { subscriptions: [] };
  try {
    await client.start();
    const connection = { client, sourceFilesVersion: 1 };
    const { registerActions } = require(path.join(config.staged, "actions.js"));
    registerActions(context, () => connection);
    if (config.scenario === "reactivate") {
      dispose(context);
      assert.equal(commands.size, 0);
      context = { subscriptions: [] };
      registerActions(context, () => connection);
    }
    const action = commands.get("clorraine.build");
    assert.equal(typeof action, "function");
    const running = action();
    if (config.scenario.startsWith("pending-execution")) {
      await until(() => tasks === 1);
      dispose(context);
      if (config.scenario === "pending-execution-error") {
        executionGate.reject(new Error("Task submission ended during disposal"));
      } else {
        executionGate.resolve();
      }
    } else if (["pending-selection", "rejected-selection"].includes(config.scenario)) {
      await until(() => events().some((event) => event.method === "clorraine/sourceFiles"));
      dispose(context);
      fs.writeFileSync(config.release, "");
    } else if (config.scenario === "pending-save") {
      await until(() => saveStarted);
      dispose(context);
      saveGate.resolve();
    }
    await running;
    if (config.scenario === "completed" || config.scenario === "unrelated") {
      for (const listener of endListeners) listener({
        execution: config.scenario === "completed" ? execution : unrelated,
      });
    }
    dispose(context);
    return {
      tasks, terminated, unrelatedTerminated, errors: errors.length,
      commandsAfterDispose: commands.size,
      selectionRequests: events().filter((event) => event.method === "clorraine/sourceFiles").length,
    };
  } finally {
    dispose(context);
    await client.stop();
  }
}

main().then((value) => process.stdout.write(JSON.stringify(value) + "\n"), (error) => {
  console.error(error);
  process.exitCode = 1;
}).finally(() => hook.deregister());
