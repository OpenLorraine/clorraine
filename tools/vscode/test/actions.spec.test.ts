/**
 * Editor actions must use a trusted, saved active entry and an explicit saved
 * dependency selection. These tests observe the compiled extension's registered
 * callbacks, real LSP transport and the external VS Code process-task boundary.
 * They do not prove compiler, emulator, desktop or Remote-WSL acceptance.
 */
import { strict as assert } from "node:assert";
import { spawnSync } from "node:child_process";
import { chmodSync, copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = resolve(here, "..");
const extension = resolve(packageRoot, "../../build/vscode/extension/out/extension.js");
const commandIds = ["clorraine.build", "clorraine.run", "clorraine.runInFsUae"];
const profiles = ["host", "freestanding", "hosted-amiga", "hosted-amiga-13"];

/** Inputs configure external editor/server state, never extension internals. */
interface Scenario {
  /** Registered user-facing command to invoke. */
  command?: string;
  /** Absent means the existing default profile. */
  profile?: string;
  /** False models restricted workspace mode at activation and invocation. */
  trusted?: boolean;
  /** Invalid active-document variants. */
  invalid?: "no-editor" | "untitled" | "extension";
  /** Dirty editor overlays are independent of the saved files. */
  dirtyEntry?: boolean;
  /** Selected open dependency overlay state. */
  dirtyDependency?: boolean;
  /** An unrelated dirty document must never be saved or block launch. */
  dirtyUnrelated?: boolean;
  /** External save cancellation or I/O failure. */
  save?: "cancel" | "throw";
  /** Server advertises the version, no capability, or an incompatible version. */
  capability?: number | "missing";
  /** External read-only request failure. */
  selectionFailure?: boolean;
  /** Mutation occurs after the request arrives and before its reply. */
  pendingChange?: "dependency" | "entry-dirty" | "editor";
  /** The active entry belongs to the second root or no workspace root. */
  location?: "second" | "standalone";
  /** Supply existing machine/user SDK, emulator and media fixture paths. */
  configuredPaths?: boolean;
}

/** JSON observations contain no private extension objects. */
interface Observation {
  /** Successful real protocol initialization and provider registrations. */
  startup: { initialized: boolean; providers: string[] };
  /** Live command registrations, captured before disposal. */
  commands: string[];
  /** User-facing error, warning and information messages. */
  messages: string[];
  /** Process or shell tasks submitted to VS Code. */
  tasks: Record<string, unknown>[];
  /** Exactly those documents whose save method was called. */
  saves: string[];
  /** Workspace-wide saving is forbidden. */
  saveAllCalls: number;
  /** Workspace executable filesystem/process probes. */
  discovery: string[];
  /** Framed server messages and editor-save observations in arrival order. */
  events: Record<string, unknown>[];
  /** The unrelated overlay's final dirty state. */
  unrelatedDirty: boolean;
}

/** Narrow decoded child output before assertions consume it. */
function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

/** Validate strings returned across the process boundary. */
function strings(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}

/** Reject collection/fixture failures separately from action-contract assertions. */
function observation(value: unknown): value is Observation {
  return record(value) && record(value.startup) &&
    typeof value.startup.initialized === "boolean" && strings(value.startup.providers) &&
    strings(value.commands) && strings(value.messages) && strings(value.saves) &&
    strings(value.discovery) && typeof value.saveAllCalls === "number" &&
    typeof value.unrelatedDirty === "boolean" &&
    Array.isArray(value.tasks) && value.tasks.every(record) &&
    Array.isArray(value.events) && value.events.every(record);
}

/**
 * Activate the staged CommonJS extension in a fresh bounded host process.
 * The fake driver cannot compile successfully; tasks are observed, not executed.
 */
function run(scenario: Scenario = {}) {
  const base = process.env.CODEOPS_TMPDIR ?? tmpdir();
  mkdirSync(base, { recursive: true });
  const scratch = mkdtempSync(join(base, "editor-actions-"));
  try {
    const first = join(scratch, "first root");
    const second = join(scratch, "second root ; $ literal");
    const standalone = join(scratch, "standalone & literal");
    const root = scenario.location === "second" ? second :
      scenario.location === "standalone" ? standalone : first;
    const external = join(scratch, "external-tools");
    for (const dir of [first, second, standalone, external]) mkdirSync(dir);
    const entry = join(root, scenario.invalid === "extension" ? "main.txt" : "-main ; $(literal).clo");
    const files = [entry, join(root, "a dependency & literal.clo"), join(root, "z-dependency.clo")];
    const unrelated = join(root, "unrelated.clo");
    writeFileSync(entry, "module app.main;\nimport { dependency } from app.dependency;\nfunc main(): i32 { return dependency(); }\n");
    writeFileSync(files[1], "module app.dependency;\nimport { value } from app.value;\nfunc dependency(): i32 { return value(); }\n");
    writeFileSync(files[2], "module app.value;\nfunc value(): i32 { return 7; }\n");
    writeFileSync(unrelated, "module app.unrelated;\nfunc main(): i32 { return 0; }\n");
    const server = join(external, "clorraine-lsp");
    copyFileSync(join(here, "fixtures/action-lsp.cjs"), server);
    chmodSync(server, 0o755);
    const driver = join(external, "clorraine");
    writeFileSync(driver, "#!/usr/bin/env node\nif (process.argv[2] === '--version') console.log('clorraine 1.1.2'); else process.exitCode = 97;\n");
    chmodSync(driver, 0o755);
    const settings = {
      "sdk.path": scenario.configuredPaths ? join(external, "SDK ; literal", "amiga") : "",
      "fsUae.path": scenario.configuredPaths ? join(external, "fs-uae") : "",
      "kickstart.path": scenario.configuredPaths ? join(external, "Kick ; literal.rom") : "",
    };
    if (scenario.configuredPaths) {
      mkdirSync(settings["sdk.path"], { recursive: true });
      copyFileSync(driver, settings["fsUae.path"]);
      writeFileSync(settings["kickstart.path"], "Test-only bytes, not licensed media.\n");
    }
    for (const workspace of [first, second]) {
      const candidate = join(workspace, "target/debug");
      mkdirSync(candidate, { recursive: true });
      copyFileSync(driver, join(candidate, "clorraine"));
      copyFileSync(server, join(candidate, "clorraine-lsp"));
    }
    const events = join(scratch, "events.jsonl");
    writeFileSync(events, "");
    const configPath = join(scratch, "scenario.json");
    const config = {
      ...scenario, command: scenario.command ?? commandIds[0], extension,
      roots: scenario.location === "standalone" ? [] : [first, second],
      entry, files, unrelated, driver, server, settings, events, release: join(scratch, "release"),
      workspaceOverrides: { "driver.path": join(first, "target/debug/clorraine") },
    };
    writeFileSync(configPath, JSON.stringify(config));
    const child = spawnSync(process.execPath, [join(here, "fixtures/action-host.cjs"), configPath], {
      encoding: "utf8", timeout: 12000, maxBuffer: 1024 * 1024,
      env: { ...process.env, ACTION_LSP_CONFIG: configPath, CLORRAINE_LSP: server,
        TMPDIR: scratch, CODEOPS_TMPDIR: scratch },
    });
    assert.equal(child.error, undefined, `fixture process error: ${child.error?.message}`);
    assert.equal(child.status, 0, `fixture startup/runtime error:\n${child.stderr}\n${child.stdout}`);
    const line = child.stdout.split("\n").find((item) => item.startsWith("ACTION_OBSERVATION="));
    assert.ok(line, "fixture must return observations");
    const result: unknown = JSON.parse(line.slice("ACTION_OBSERVATION=".length));
    assert.ok(observation(result), "fixture output must have the documented observation shape");
    if (scenario.trusted !== false) {
      assert.equal(result.startup.initialized, true, "real LSP initialization is the baseline control");
      assert.ok(result.startup.providers.includes("hover"), "existing language registration must work");
    }
    return { result, entry, files, root, driver, settings };
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

/** Every behavior assertion requires the actual registered public callback. */
function invoke(scenario: Scenario = {}) {
  const actual = run(scenario);
  const id = scenario.command ?? commandIds[0];
  assert.ok(actual.result.commands.includes(id), `missing registered command: ${id}`);
  return actual;
}

/** No rejected action may leak into execution or workspace-wide saving. */
function rejected(scenario: Scenario, remedy: RegExp) {
  const actual = invoke(scenario);
  assert.equal(actual.result.tasks.length, 0, "rejected action must not launch a task");
  assert.equal(actual.result.saveAllCalls, 0, "never save all workspace documents");
  const message = actual.result.messages.join("\n");
  assert.match(message, remedy);
  assert.match(message, scenario.command === commandIds[1] || scenario.command === commandIds[2]
    ? /run/i : /build/i, "failure identifies the operation");
  if (scenario.command === commandIds[2]) assert.match(message, /fs-uae/i);
  assert.ok(message.split(/[^A-Za-z0-9-]+/).includes(scenario.profile ?? "hosted-amiga"),
    "failure identifies the selected profile, not only a suggested replacement");
  return actual;
}

test("should activate the compiled extension and initialize its real LSP client under the facade", () => {
  const { result } = run();
  assert.ok(result.events.some((event) => event.method === "initialize"));
  assert.ok(result.events.some((event) => event.method === "initialized"));
});

test("should register and declare all three labelled editor commands", () => {
  const { result } = run();
  for (const id of commandIds) assert.ok(result.commands.includes(id), `missing registered command: ${id}`);
  const manifest: unknown = JSON.parse(readFileSync(join(packageRoot, "package.json"), "utf8"));
  assert.ok(record(manifest) && record(manifest.contributes));
  const contributions = manifest.contributes.commands;
  assert.ok(Array.isArray(contributions), "commands must be visible in the command palette");
  for (const [index, title] of ["Clorraine: Build", "Clorraine: Run", "Clorraine: Run in FS-UAE"].entries()) {
    assert.ok(contributions.some((item: unknown) =>
      record(item) && item.command === commandIds[index] && item.title === title), title);
  }
});

// Each action must pass saved sources as literal argv; settings apply only to supported modes.
for (const profile of profiles) {
  for (const command of commandIds) {
    const emulator = command === commandIds[2];
    if (emulator && !profile.startsWith("hosted-amiga")) continue;
    test(`should launch ${command} with explicit saved dependencies and applicable settings for ${profile}`, () => {
      const { result, files, root, driver, entry, settings } = invoke({
        command, profile, configuredPaths: true,
        location: "second", dirtyEntry: true, dirtyUnrelated: true });
      assert.equal(result.tasks.length, 1);
      const task = result.tasks[0];
      assert.equal(task.kind, "process", "shell command strings are forbidden");
      assert.equal(task.process, driver);
      assert.equal(task.cwd, root, "use the containing second root, not the initial LSP root");
      assert.ok(strings(task.args), "ProcessExecution requires a string argument array");
      assert.deepEqual(task.args.slice(0, 3), [command === commandIds[0] ? "build" : "run", "--profile", profile]);
      assert.deepEqual(task.args.slice(-files.length), files, "entry first, dependencies lexical, one absolute path per argument");
      assert.ok(!task.args.includes("--"), "the driver does not support a -- delimiter");
      assert.equal(task.args.filter((arg) => arg === "--emulator").length, emulator ? 1 : 0);
      for (const [flag, value, applies] of [
        ["--sdk-root", settings["sdk.path"], profile !== "host"],
        ["--fs-uae", settings["fsUae.path"], emulator],
        ["--kickstart", settings["kickstart.path"], emulator],
      ] as const) {
        assert.equal(task.args.filter((arg) => arg === flag).length, applies ? 1 : 0, flag);
        if (applies) assert.equal(task.args[task.args.indexOf(flag) + 1], value);
        else assert.ok(!task.args.includes(value), `inapplicable ${flag} value must not leak`);
      }
      assert.equal(task.args.length, 3 + files.length + (profile === "host" ? 0 : 2) +
        (emulator ? 5 : 0), "no hidden shell fragments or unsupported extra arguments");
      assert.deepEqual(result.saves, [entry], "save only the active entry");
      assert.equal(result.saveAllCalls, 0);
      assert.equal(result.unrelatedDirty, true);
      const selected = result.events.findIndex((event) => event.method === "clorraine/sourceFiles");
      const saved = result.events.findIndex((event) => event.method === "fixture/save");
      assert.ok(saved >= 0 && selected > saved, "request selection only after successful entry save");
      assert.deepEqual(result.events[selected].params,
        { entryUri: pathToFileURL(entry).href, rootUri: pathToFileURL(root).href });
      assert.ok(record(task.presentation));
      assert.equal(task.presentation.reveal ?? 1, 1, "task output must be visible; VS Code defaults to Always");
      assert.match(String(task.name), emulator ? /run in fs-uae/i : command === commandIds[0] ? /build/i : /run/i);
      assert.ok(String(task.name).split(/[^A-Za-z0-9-]+/).includes(profile),
        "task title identifies the selected profile");
      assert.doesNotMatch(`${task.name}\n${task.detail ?? ""}\n${result.messages.join("\n")}`,
        /guest.*(?:succe|exit status|exit code)/i);
    });
  }
}

test("should use the existing default profile and omit empty optional path flags", () => {
  const { result, files } = invoke();
  assert.equal(result.tasks.length, 1);
  assert.deepEqual(result.tasks[0].args, ["build", "--profile", "hosted-amiga", ...files]);
  assert.deepEqual(result.saves, [], "a clean entry needs no save");
});

test("should select the standalone entry parent for both request root and process CWD", () => {
  const { result, root, entry } = invoke({ location: "standalone" });
  assert.equal(result.tasks.length, 1);
  assert.equal(result.tasks[0].cwd, root);
  assert.deepEqual(result.events.find((event) => event.method === "clorraine/sourceFiles")?.params,
    { entryUri: pathToFileURL(entry).href, rootUri: pathToFileURL(root).href });
});

for (const command of commandIds) {
  test(`should reject untrusted ${command} before executable discovery, saving or selection`, () => {
    const { result } = rejected({ command, trusted: false, dirtyEntry: true }, /trust.*(?:workspace|folder)|(?:workspace|folder).*trust/i);
    assert.deepEqual(result.discovery, [], "workspace executable lookup is not safe initialization");
    assert.deepEqual(result.saves, []);
    assert.ok(!result.events.some((event) => event.method === "clorraine/sourceFiles"));
    assert.match(result.messages.join("\n"), /enable|grant|manage|trust (?:this|the|your)|workspace trust/i,
      "trust rejection must explain how to proceed");
  });
}

for (const profile of ["host", "freestanding"]) {
  test(`should reject FS-UAE for ${profile} with an actionable profile remedy`, () => {
    rejected({ command: commandIds[2], profile }, /(?:select|choose|switch|use).*hosted-amiga/i);
  });
}

for (const invalid of ["no-editor", "untitled", "extension"] as const) {
  test(`should reject ${invalid} instead of selecting or saving a stale entry`, () => {
    const { result } = rejected({ invalid, dirtyEntry: true }, /(?:open|select|save).*\.clo/i);
    assert.deepEqual(result.saves, []);
    assert.ok(!result.events.some((event) => event.method === "clorraine/sourceFiles"));
  });
}

for (const save of ["cancel", "throw"] as const) {
  test(`should stop after entry save ${save} without requesting a stale disk selection`, () => {
    const { result, entry } = rejected({ dirtyEntry: true, save }, /save|sav(?:ed|ing)/i);
    assert.deepEqual(result.saves, [entry]);
    assert.ok(!result.events.some((event) => event.method === "clorraine/sourceFiles"));
  });
}

test("should block a dirty selected dependency with save and retry guidance without auto-saving it", () => {
  const { result } = rejected({ command: commandIds[1], dirtyEntry: true,
    dirtyDependency: true, dirtyUnrelated: true }, /save/i);
  assert.match(result.messages.join("\n"), /retry|try again/i);
  assert.match(result.messages.join("\n"), /a dependency/);
  assert.equal(result.saves.length, 1, "only active entry may be saved");
  assert.equal(result.unrelatedDirty, true);
});

for (const [index, pendingChange] of (["dependency", "entry-dirty", "editor"] as const).entries()) {
  test(`should block ${pendingChange} changes while saved-source selection is pending`, () => {
    const { result } = rejected({ command: commandIds[index], pendingChange }, /save|retry|try again|changed/i);
    assert.ok(result.events.some((event) => event.method === "clorraine/sourceFiles"));
    assert.deepEqual(result.saves, [], "no automatic dependency or unrelated saves");
  });
}

for (const [index, capability] of (["missing", 0, 2] as const).entries()) {
  test(`should reject selection capability ${capability} rather than silently launch entry-only`, () => {
    const { result } = rejected({ command: commandIds[index], capability }, /(?:update|upgrade|install|restart|compatible|version)/i);
    assert.ok(!result.events.some((event) => event.method === "clorraine/sourceFiles"),
      "do not call an unavailable or incompatible selection protocol");
  });
}

test("should report read-only selection failure without launching an entry-only fallback", () => {
  const { result } = rejected({ selectionFailure: true }, /selection|source|import/i);
  assert.match(result.messages.join("\n"), /retry|try again|fix/i);
  assert.ok(result.events.some((event) => event.method === "clorraine/sourceFiles"));
});
