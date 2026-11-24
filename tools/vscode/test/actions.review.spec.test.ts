/**
 * Saved-source actions reject editor changes in flight and task-variable
 * introducers at the public process-task boundary. No task is actually run.
 */
import { strict as assert } from "node:assert";
import { spawnSync } from "node:child_process";
import { chmodSync, mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const staged = resolve(here, "../../../build/vscode/extension/out");
const commands = ["clorraine.build", "clorraine.run", "clorraine.runInFsUae"];

/** Scenario fields describe external editor state or literal user path settings. */
interface Scenario {
  /** Public command ID; build is the default. */
  command?: string;
  /** Editor events delivered while the real selection reply is gated. */
  event?: "dependency" | "entry" | "save-only" | "alias" | "outside" | "nonclo";
  /** Boundary slot containing literal task-variable syntax. */
  input?: "executable" | "entry" | "dependency" | "toolchain" | "emulator" | "rom" | "cwd";
  /** Literal bytes inserted into the selected input slot. */
  token?: string;
  /** The external peer returns a selection error after the barrier. */
  failure?: boolean;
  /** Dispose the public registration while selection is pending. */
  disposePending?: boolean;
  /** Replace the selected public session while selection is pending. */
  sessionChange?: boolean;
  /** Dirty-buffer compatibility controls, independent of public event emission. */
  dirtyDependency?: boolean;
  /** The active entry becomes dirty while selection is pending. */
  dirtyEntry?: boolean;
  /** Use ordinary punctuation in every submitted path. */
  punctuation?: boolean;
}

/** A JSON object crossing the isolated host boundary. */
function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Launch an isolated editor facade; always remove its private files. */
function observe(scenario: Scenario = {}) {
  const base = process.env.CODEOPS_TMPDIR ?? tmpdir();
  mkdirSync(base, { recursive: true });
  const scratch = mkdtempSync(join(base, "editor-review-"));
  try {
    const ordinary = scenario.punctuation ? " ; & # λ $literal " : "";
    const literal = (slot: Scenario["input"], name: string) =>
      `${name}${scenario.input === slot ? scenario.token : ordinary}`;
    const root = join(scratch, `workspace${ordinary}`);
    const tools = join(scratch, "tools");
    mkdirSync(root);
    mkdirSync(tools);
    const rootAlias = join(scratch, literal("cwd", "workspace-alias"));
    symlinkSync(root, rootAlias, "dir");
    const entry = join(root, `${literal("entry", "main")}.clo`);
    const dependency = join(root, `${literal("dependency", "dependency")}.clo`);
    const outside = join(scratch, "outside.clo");
    const nonclo = join(root, "notes.txt");
    for (const file of [entry, dependency, outside, nonclo]) {
      writeFileSync(file, "module observation;\nfunc main(): i32 { return 0; }\n");
    }
    const driver = join(tools, literal("executable", "clorraine"));
    writeFileSync(driver, "#!/usr/bin/env node\nif (process.argv[2] === '--version') console.log('clorraine 1.1.2'); else process.exitCode = 97;\n");
    chmodSync(driver, 0o755);
    const toolchain = join(tools, literal("toolchain", "toolchain"));
    const emulator = join(tools, literal("emulator", "fs-uae"));
    const rom = join(tools, literal("rom", "kickstart.rom"));
    mkdirSync(toolchain);
    writeFileSync(emulator, "#!/bin/sh\nexit 97\n");
    chmodSync(emulator, 0o755);
    writeFileSync(rom, "Observation-only bytes; not licensed media.\n");
    const settings = {
      "driver.path": driver, "toolchain.path": toolchain, "fsUae.path": emulator, "kickstart.path": rom,
    };
    const config = join(scratch, "scenario.json");
    writeFileSync(config, JSON.stringify({
      ...scenario, command: scenario.command ?? commands[0], profile: "hosted-amiga",
      scratch, staged, root: scenario.input === "cwd" ? rootAlias : root,
      entry, files: [entry, dependency], settings, outside, nonclo,
      aliasDependency: join(rootAlias, `${literal("dependency", "dependency")}.clo`),
      events: join(scratch, "events.jsonl"), release: join(scratch, "release"),
    }));
    const child = spawnSync(process.execPath, [join(here, "fixtures/actions-review-host.cjs"), config], {
      encoding: "utf8", timeout: 12000, maxBuffer: 1024 * 1024,
      env: { ...process.env, TMPDIR: scratch, CODEOPS_TMPDIR: scratch },
    });
    assert.equal(child.error, undefined, `fixture process failure: ${child.error?.message}`);
    assert.equal(child.status, 0, `fixture startup/runtime failure:\n${child.stderr}\n${child.stdout}`);
    const actual: unknown = JSON.parse(child.stdout);
    assert.ok(record(actual));
    assert.ok(Array.isArray(actual.tasks) && actual.tasks.every(record));
    assert.ok(Array.isArray(actual.messages) && actual.messages.every((value) => typeof value === "string"));
    return { actual, tasks: actual.tasks, message: actual.messages.join("\n"), root, entry, dependency, driver, settings };
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

/** Request-local subscriptions cannot survive completion or activation disposal. */
function cleaned(actual: Record<string, unknown>) {
  assert.deepEqual(actual.after, { change: 0, save: 0 }, "request subscriptions must end with the action");
  assert.deepEqual(actual.disposed, { change: 0, save: 0 }, "disposal must not retain event subscribers");
  assert.equal(actual.commandsAfterDispose, 0);
}

for (const [index, event] of (["dependency", "entry", "save-only", "alias"] as const).entries()) {
  test(`should reject a clean ${event} change or save while source selection is pending`, () => {
    const { actual, tasks, message } = observe({ event, command: commands[index % commands.length] });
    assert.deepEqual(actual.emitted, event === "save-only" ? ["save"] : ["change", "save"],
      "deliver the actual public event callbacks before releasing the reply");
    if (event === "dependency") assert.deepEqual(actual.finalDependency, { dirty: false, version: 2 });
    assert.equal(tasks.length, 0, "a clean final document does not make a stale selection safe");
    assert.match(message, /retry|try again/i);
    cleaned(actual);
  });
}

test("should subscribe to both editor events before sending selection and clean up on success", () => {
  const { actual, tasks } = observe();
  assert.equal(tasks.length, 1, "unchanged saved sources remain runnable");
  assert.ok(record(actual.during));
  assert.ok(Number(actual.during.change) > 0 && Number(actual.during.save) > 0,
    "both public events must be observed while selection is in flight");
  cleaned(actual);
});

for (const event of ["outside", "nonclo"] as const) {
  test(`should allow ${event} edits and saves while source selection is pending`, () => {
    const { actual, tasks } = observe({ event });
    assert.deepEqual(actual.emitted, ["change", "save"]);
    assert.equal(tasks.length, 1, "edits outside the selected root or outside .clo must not block");
    cleaned(actual);
  });
}

for (const scenario of [{ failure: true }, { disposePending: true },
  { sessionChange: true }, { dirtyDependency: true }, { dirtyEntry: true }]) {
  const name = Object.keys(scenario)[0];
  test(`should retain ${name} rejection and release request-local event subscribers`, () => {
    const { actual, tasks, message } = observe(scenario);
    assert.equal(tasks.length, 0);
    if (!("disposePending" in scenario)) assert.match(message, /retry|try again|save|changed|selection/i);
    cleaned(actual);
  });
}

// ProcessExecution still interprets task variables; every final input slot is unsafe.
for (const input of ["executable", "entry", "dependency", "toolchain", "emulator", "rom", "cwd"] as const) {
  for (const token of ["${command:marker}", "${env:MARKER}", "${config:marker}", "${input:marker}", "${unfinished"]) {
    test(`should reject literal ${token} in task ${input} before submission`, () => {
      const { actual, tasks, message } = observe({ input, token, command: commands[2] });
      assert.equal(tasks.length, 0, "literal task-variable introducers must never reach executeTask");
      assert.match(message, /rename|(?:change|choose|set|select|use).*(?:path|file|folder|setting)/i,
        "rejection must explain how to choose a safe literal path");
      cleaned(actual);
    });
  }
}

test("should preserve ordinary spaces punctuation unicode and dollar literals in all task inputs", () => {
  const { actual, tasks, root, entry, dependency, driver, settings } =
    observe({ punctuation: true, command: commands[2] });
  assert.equal(tasks.length, 1);
  assert.equal(tasks[0].process, driver);
  assert.equal(tasks[0].cwd, root);
  assert.ok(Array.isArray(tasks[0].args));
  assert.deepEqual(tasks[0].args, ["run", "--profile", "hosted-amiga", "--toolchain", settings["toolchain.path"],
    "--emulator", "--fs-uae", settings["fsUae.path"], "--kickstart", settings["kickstart.path"], entry, dependency]);
  cleaned(actual);
});
