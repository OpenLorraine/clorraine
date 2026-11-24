/**
 * Installed tools must work without terminal PATH, while explicit user choices
 * and trusted development/PATH fallbacks remain available. Workspace settings
 * cannot supply executable overrides. Public lifecycle/profile observations use
 * the unchanged staged extension and real subprocess LSP framing.
 * This is not desktop, compiler, emulator or real-machine acceptance.
 */
import { strict as assert } from "node:assert";
import { spawnSync } from "node:child_process";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = resolve(here, "..");
const staged = resolve(packageRoot, "../../build/vscode/extension");
const toolNames = ["clorraine-lsp", "clorraine"] as const;
const pathSettings = ["lsp.path", "driver.path", "toolchain.path", "fsUae.path", "kickstart.path"];
const profiles = ["hosted-amiga", "hosted-amiga-13", "host", "freestanding"];
const workspaceLevels = ["workspaceValue", "workspaceFolderValue",
  "workspaceLanguageValue", "workspaceFolderLanguageValue"];

/** Candidate directories are real, isolated executable locations. */
type Location = "installed" | "debug" | "release" | "path" | "custom" | "environment" | "workspace";
/** Public configuration inspection fields, retaining precedence independently. */
type Levels = Record<string, string>;
/** One fresh extension-host scenario; no extension-private inputs are accepted. */
interface Scenario {
  /** Which LSP candidates physically exist. */
  lsp?: Location[];
  /** Which driver candidates physically exist. */
  driver?: Location[];
  /** Global user path overrides, with optional hostile workspace levels. */
  setting?: { tool: "lsp.path" | "driver.path"; location?: Location; invalid?: "missing" | "directory" | "nonexecutable" };
  /** Environment LSP override points to a distinct candidate. */
  environment?: boolean;
  /** Each level is merged by the external facade, not by a discovery replacement. */
  workspaceLevel?: string;
  /** The hostile setting can target either executable or all optional paths. */
  workspaceTool?: "lsp.path" | "driver.path" | "optional";
  /** Optional user values coexist with hostile workspace values. */
  optionalUser?: boolean;
  /** Restricted mode is set before activation. */
  trusted?: boolean;
  /** Public action callback, when this scenario needs a driver selection. */
  command?: string;
  /** Initial and changed profile values are ordinary editor configuration. */
  profile?: string;
  nextProfile?: string;
  /** Dispose first context, then activate again before invoking one command. */
  lifecycle?: boolean;
}

/** Validated JSON observations contain external values only. */
interface Observation {
  /** Real protocol startup and registered language providers. */
  startup: { initialized: boolean; providers: string[] };
  /** Live command IDs and first-cycle IDs remaining after disposal. */
  commands: string[];
  disposedCommands?: string[];
  /** UI messages, diagnostic messages and workspace candidate probes. */
  messages: string[];
  diagnostics: string[];
  probes: string[];
  /** Task and process/protocol records. */
  tasks: Record<string, unknown>[];
  events: Record<string, unknown>[];
}

/** Narrow untrusted decoded process/manifest values without unsafe casts. */
function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
/** Validate string arrays before consuming external output. */
function strings(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}
/** A malformed observation is a harness error, never an expected behavioral red. */
function observation(value: unknown): value is Observation {
  return record(value) && record(value.startup) && typeof value.startup.initialized === "boolean" &&
    strings(value.startup.providers) && strings(value.commands) && strings(value.messages) &&
    strings(value.diagnostics) && strings(value.probes) &&
    (value.disposedCommands === undefined || strings(value.disposedCommands)) &&
    Array.isArray(value.tasks) && value.tasks.every(record) &&
    Array.isArray(value.events) && value.events.every(record);
}

/**
 * Create only owned HOME/PATH/workspace trees. Candidate wrappers use an absolute
 * Node shebang so desktop PATH can contain neither Clorraine tool nor Node.
 */
function run(scenario: Scenario = {}) {
  const base = process.env.CODEOPS_TMPDIR ?? tmpdir();
  mkdirSync(base, { recursive: true });
  const scratch = mkdtempSync(join(base, "editor-discovery-"));
  try {
    const home = join(scratch, "home");
    const workspace = join(scratch, "source workspace");
    const dirs: Record<Location, string> = {
      installed: join(home, ".local/bin"), debug: join(workspace, "target/debug"),
      release: join(workspace, "target/release"), path: join(scratch, "desktop-path"),
      custom: join(scratch, "custom root ; $ literal/bin"),
      environment: join(scratch, "environment/bin"), workspace: join(workspace, "workspace-tools"),
    };
    for (const dir of [home, workspace, ...Object.values(dirs)]) mkdirSync(dir, { recursive: true });
    const entry = join(workspace, "main.clo");
    writeFileSync(entry, "module app.main;\nfunc main(): i32 { return 0; }\n");
    const executable = (location: Location, tool: typeof toolNames[number]) => join(dirs[location], tool);
    const peer = readFileSync(join(here, "fixtures/discovery-tool.cjs"), "utf8")
      .replace(/^#![^\n]*/, `#!${process.execPath}`);
    for (const [tool, candidates] of [
      [toolNames[0], scenario.lsp ?? ["custom"]],
      [toolNames[1], scenario.driver ?? ["custom"]],
    ] as const) {
      for (const candidate of candidates) {
        writeFileSync(executable(candidate, tool), peer, { mode: 0o755 });
      }
    }
    const configuration: Record<string, Levels> = Object.fromEntries(pathSettings.map((key) => [key, { defaultValue: "" }]));
    configuration.profile = { defaultValue: "hosted-amiga" };
    if (scenario.profile) configuration.profile.globalValue = scenario.profile;
    // Driver-only tests use a safe explicit external LSP as their startup control.
    if (scenario.lsp === undefined) configuration["lsp.path"].globalValue = executable("custom", toolNames[0]);
    if (scenario.driver === undefined) configuration["driver.path"].globalValue = executable("custom", toolNames[1]);
    if (scenario.setting) {
      const { tool, location = "custom", invalid } = scenario.setting;
      const selected = invalid ? join(scratch, `invalid-${invalid}`) :
        executable(location, tool === "lsp.path" ? toolNames[0] : toolNames[1]);
      if (invalid === "directory") mkdirSync(selected);
      if (invalid === "nonexecutable") { writeFileSync(selected, peer); chmodSync(selected, 0o644); }
      configuration[tool].globalValue = selected;
    }
    const optional = {
      "toolchain.path": join(scratch, "user toolchain"),
      "fsUae.path": join(scratch, "user emulator"),
      "kickstart.path": join(scratch, "user media.rom"),
    };
    mkdirSync(optional["toolchain.path"], { recursive: true });
    writeFileSync(optional["fsUae.path"], peer, { mode: 0o755 });
    writeFileSync(optional["kickstart.path"], "Not licensed media.\n");
    const hostile: Record<string, string> = {
      "lsp.path": executable("workspace", toolNames[0]),
      "driver.path": executable("workspace", toolNames[1]),
      "toolchain.path": join(dirs.workspace, "toolchain"),
      "fsUae.path": join(dirs.workspace, "fs-uae"),
      "kickstart.path": join(dirs.workspace, "kickstart.rom"),
    };
    mkdirSync(hostile["toolchain.path"], { recursive: true });
    writeFileSync(hostile["fsUae.path"], peer, { mode: 0o755 });
    writeFileSync(hostile["kickstart.path"], "Hostile test-only media bytes.\n");
    if (scenario.optionalUser) {
      for (const [key, value] of Object.entries(optional)) configuration[key].globalValue = value;
    }
    if (scenario.workspaceTool && scenario.workspaceLevel) {
      const keys = scenario.workspaceTool === "optional" ? Object.keys(optional) : [scenario.workspaceTool];
      for (const key of keys) configuration[key][scenario.workspaceLevel] = hostile[key];
    }
    const events = join(scratch, "events.jsonl");
    writeFileSync(events, "");
    const configPath = join(scratch, "scenario.json");
    writeFileSync(configPath, JSON.stringify({
      ...scenario, configuration, workspace, entry, events, extension: join(staged, "out/extension.js"),
    }));
    const env: NodeJS.ProcessEnv = { ...process.env, HOME: home, USERPROFILE: home, PATH: dirs.path,
      TMPDIR: scratch, CODEOPS_TMPDIR: scratch, DISCOVERY_CONFIG: configPath };
    // Never inherit a developer's override or a real user installation.
    delete env.CLORRAINE_LSP;
    if (scenario.environment) env.CLORRAINE_LSP = executable("environment", toolNames[0]);
    const child = spawnSync(process.execPath, [join(here, "fixtures/discovery-host.cjs"), configPath], {
      encoding: "utf8", env, timeout: 14000, maxBuffer: 1024 * 1024,
    });
    assert.equal(child.error, undefined, `fixture process error: ${child.error?.message}`);
    assert.equal(child.status, 0, `fixture runtime error:\n${child.stderr}\n${child.stdout}`);
    const line = child.stdout.split("\n").find((item) => item.startsWith("DISCOVERY_OBSERVATION="));
    assert.ok(line, "fixture must return observations");
    const result: unknown = JSON.parse(line.slice("DISCOVERY_OBSERVATION=".length));
    assert.ok(observation(result), "fixture must return valid external observations");
    return { result, executable, configuration, optional, hostile };
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

/** Identity assertions require actual protocol traffic, not a --version banner. */
function selectedLsp(actual: ReturnType<typeof run>, location: Location) {
  assert.equal(actual.result.startup.initialized, true, "selected LSP must initialize through the real client");
  const sessions = actual.result.events.filter((event) => event.method === "initialize");
  assert.ok(sessions.length > 0, "real initialize request must reach an executable");
  assert.ok(sessions.every((event) => event.executable === actual.executable(location, "clorraine-lsp")),
    `LSP selection must execute ${location}, not a same-version lower-priority candidate`);
}

/** Only an actual registered action may establish driver discovery behavior. */
function selectedDriver(actual: ReturnType<typeof run>, location: Location, command = "clorraine.build") {
  assert.equal(actual.result.startup.initialized, true, "safe explicit LSP is the action startup control");
  assert.ok(actual.result.commands.includes(command), `missing registered command: ${command}`);
  assert.equal(actual.result.tasks.length, 1, "one invocation must submit exactly one task");
  assert.equal(actual.result.tasks[0].kind, "process");
  assert.equal(actual.result.tasks[0].probeStatus, 0, "selected task executable must actually launch");
  assert.deepEqual(actual.result.tasks[0].identities, [actual.executable(location, "clorraine")],
    `the task itself must select ${location}; discovery-time version probes are not proof`);
}

test("should initialize explicit external tools with the default diagnostic profile", () => {
  const actual = run();
  selectedLsp(actual, "custom");
  assert.ok(actual.result.startup.providers.includes("hover"));
  assert.ok(actual.result.diagnostics.includes("profile:hosted-amiga"));
});

// The installed location precedes development builds; existing fallback order remains.
for (const [name, scenario, expected] of [
  ["explicit user override", { lsp: ["custom", "environment", "installed", "debug", "release", "path"],
    setting: { tool: "lsp.path" }, environment: true }, "custom"],
  ["environment override", { lsp: ["environment", "installed", "debug", "release", "path"], environment: true }, "environment"],
  ["installed as the only candidate", { lsp: ["installed"] }, "installed"],
  ["desktop default without terminal PATH", { lsp: ["installed", "debug", "release"] }, "installed"],
  ["installed before PATH", { lsp: ["installed", "path"] }, "installed"],
  ["trusted debug before release and PATH", { lsp: ["debug", "release", "path"] }, "debug"],
  ["trusted release before PATH", { lsp: ["release", "path"] }, "release"],
  ["PATH fallback", { lsp: ["path"] }, "path"],
] satisfies [string, Scenario, Location][]) {
  test(`should select LSP ${name} by its actual executable identity`, () => selectedLsp(run(scenario), expected));
}

for (const [name, driver, setting, expected] of [
  ["explicit custom-root user override", ["custom", "installed", "debug", "release", "path"], true, "custom"],
  ["installed as the only candidate", ["installed"], false, "installed"],
  ["desktop default without terminal PATH", ["installed", "debug", "release"], false, "installed"],
  ["installed before PATH", ["installed", "path"], false, "installed"],
  ["trusted debug before release and PATH", ["debug", "release", "path"], false, "debug"],
  ["trusted release before PATH", ["release", "path"], false, "release"],
  ["PATH fallback", ["path"], false, "path"],
] satisfies [string, Location[], boolean, Location][]) {
  test(`should select driver ${name} at the public process-task boundary`, () => {
    selectedDriver(run({ driver, command: "clorraine.build",
      ...(setting ? { setting: { tool: "driver.path" } as const } : {}) }), expected);
  });
}

test("should discover both default-installed tools together without PATH or explicit overrides", () => {
  const actual = run({ lsp: ["installed"], driver: ["installed"], command: "clorraine.build" });
  selectedLsp(actual, "installed");
  selectedDriver(actual, "installed");
});

for (const tool of ["lsp.path", "driver.path"] as const) {
  for (const invalid of ["missing", "directory", "nonexecutable"] as const) {
    test(`should reject explicit ${invalid} ${tool} with a configuration remedy and no fallback`, () => {
      const actual = run({ ...(tool === "lsp.path" ? { lsp: ["installed", "path"] as Location[] } :
        { driver: ["installed", "path"] as Location[], command: "clorraine.build" }),
        setting: { tool, invalid } });
      if (tool === "driver.path") {
        assert.equal(actual.result.startup.initialized, true);
        assert.ok(actual.result.commands.includes("clorraine.build"), "missing registered command: clorraine.build");
      }
      assert.equal(actual.result.tasks.length, 0);
      if (tool === "lsp.path") assert.equal(actual.result.startup.initialized, false);
      const message = actual.result.messages.join("\n");
      assert.match(message, new RegExp(`clorraine\\.${tool.replace(".", "\\.")}`));
      assert.match(message, /set|configur|install|correct|executable|valid/i, "explain how to fix the path");
      assert.ok(!actual.result.events.some((event) => event.kind === "exec" &&
        [actual.executable("installed", tool === "lsp.path" ? "clorraine-lsp" : "clorraine"),
          actual.executable("path", tool === "lsp.path" ? "clorraine-lsp" : "clorraine")].includes(String(event.executable))),
      "invalid explicit settings must not execute lower-priority tools");
    });
  }
}

for (const workspaceLevel of workspaceLevels) {
  for (const user of [false, true]) {
    for (const tool of ["lsp.path", "driver.path"] as const) {
      test(`should ignore ${workspaceLevel} ${tool} with ${user ? "a user override" : "only installed fallback"}`, () => {
        const actual = run({
          ...(tool === "lsp.path" ? { lsp: ["custom", "installed", "workspace"] as Location[] } :
            { driver: ["custom", "installed", "workspace"] as Location[], command: "clorraine.build" }),
          workspaceTool: tool, workspaceLevel,
          ...(user ? { setting: { tool } } : {}),
        });
        if (tool === "lsp.path") selectedLsp(actual, user ? "custom" : "installed");
        else selectedDriver(actual, user ? "custom" : "installed");
        assert.ok(!actual.result.events.some((event) => event.kind === "exec" &&
          event.executable === actual.executable("workspace", tool === "lsp.path" ? "clorraine-lsp" : "clorraine")));
      });
    }
  }
}

for (const workspaceLevel of workspaceLevels) {
  for (const optionalUser of [false, true]) {
    test(`should ignore ${workspaceLevel} toolchain emulator and ROM overrides with user paths ${optionalUser}`, () => {
      const actual = run({ workspaceTool: "optional", workspaceLevel, optionalUser, command: "clorraine.runInFsUae" });
      selectedDriver(actual, "custom", "clorraine.runInFsUae");
      const args = actual.result.tasks[0].args;
      assert.ok(strings(args));
      for (const [key, flag] of [["toolchain.path", "--toolchain"], ["fsUae.path", "--fs-uae"], ["kickstart.path", "--kickstart"]]) {
        assert.ok(!args.includes(actual.hostile[key]), "workspace path cannot silently supply a CLI override");
        assert.equal(args.includes(flag), optionalUser);
        if (optionalUser) assert.equal(args[args.indexOf(flag) + 1], actual.configuration[key].globalValue);
      }
    });
  }
}

test("should not probe or execute workspace LSP candidates during restricted activation", () => {
  const { result } = run({ trusted: false, lsp: ["debug", "release", "workspace"],
    workspaceTool: "lsp.path", workspaceLevel: "workspaceValue" });
  assert.deepEqual(result.probes, [], "trust must precede workspace executable lookup");
  assert.ok(!result.events.some((event) => event.kind === "exec"), "no workspace process may start");
});

for (const nextProfile of profiles) {
  test(`should apply a live profile change to ${nextProfile} in LSP diagnostics and the next action`, () => {
    const actual = run({ profile: nextProfile === "host" ? "hosted-amiga" : "host",
      nextProfile, command: "clorraine.run" });
    assert.ok(actual.result.diagnostics.includes(`profile:${nextProfile}`),
      "the real LSP connection must receive the changed profile");
    selectedDriver(actual, "custom", "clorraine.run");
    const args = actual.result.tasks[0].args;
    assert.ok(strings(args));
    assert.equal(args[args.indexOf("--profile") + 1], nextProfile);
  });
}

test("should dispose commands on deactivation and register one callback per command on reactivation", () => {
  const actual = run({ lifecycle: true, command: "clorraine.build" });
  selectedDriver(actual, "custom");
  assert.deepEqual(actual.result.disposedCommands, [], "old extension-context registrations must be disposed");
  for (const command of ["clorraine.build", "clorraine.run", "clorraine.runInFsUae"]) {
    assert.equal(actual.result.commands.filter((id) => id === command).length, 1, command);
  }
  assert.equal(actual.result.events.filter((event) => event.method === "initialize").length, 2,
    "each public activation starts one LSP session");
});

for (const key of pathSettings) {
  test(`should contribute machine-only clorraine.${key} metadata in the staged package`, () => {
    const manifest: unknown = JSON.parse(readFileSync(join(staged, "package.json"), "utf8"));
    assert.ok(record(manifest) && record(manifest.contributes) &&
      record(manifest.contributes.configuration) && record(manifest.contributes.configuration.properties));
    const property = manifest.contributes.configuration.properties[`clorraine.${key}`];
    assert.ok(record(property), `missing setting clorraine.${key}`);
    assert.equal(property.type, "string");
    assert.equal(property.default, "");
    assert.equal(property.scope, "machine", "workspace-overridable scopes violate executable path isolation");
  });
}

test("should preserve the existing shared profile default and all four profile choices in staged metadata", () => {
  const manifest: unknown = JSON.parse(readFileSync(join(staged, "package.json"), "utf8"));
  assert.ok(record(manifest) && record(manifest.contributes) &&
    record(manifest.contributes.configuration) && record(manifest.contributes.configuration.properties));
  const property = manifest.contributes.configuration.properties["clorraine.profile"];
  assert.ok(record(property));
  assert.equal(property.default, "hosted-amiga");
  assert.ok(strings(property.enum));
  assert.deepEqual([...property.enum].sort(), [...profiles].sort());
});
