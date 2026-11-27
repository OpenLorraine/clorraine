//! The driver command used by portable starter tasks, the "Open install
//! guide" action on missing-tool errors, and the once-per-activation notice
//! when the installed language server is older than the extension. Each
//! scenario runs the staged extension in a fresh editor facade process with an
//! empty HOME and PATH, so only the tools a scenario provides can be found.

import { strict as assert } from "node:assert";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const staged = resolve(here, "../../../build/vscode/extension");
const guideUrl = process.platform === "win32" ?
  "https://openlorraine.github.io/clorraine/guide/quick-start#install-on-windows" :
  "https://openlorraine.github.io/clorraine/guide/quick-start#install-on-linux";

interface Message { kind: string; text: string; items: string[] }
interface Observation {
  messages: Message[];
  opened: string[];
  results: Record<string, { registered: boolean; value?: unknown }>;
  starts: number;
  commands: string[];
}

interface Scenario {
  /** Version the language server reports; omitted means no server is installed. */
  server?: string;
  /** Whether a driver executable is configured. */
  driver?: boolean;
  /** The extension's own version, as its manifest states it. */
  extension?: string;
  /** Action label the facade answers messages with. */
  choose?: string;
  /** Emit one profile change after startup, restarting the server. */
  restart?: boolean;
  /** Command ids to execute after startup. */
  commands?: string[];
}

/** Runs one scenario and returns the facade's observations. */
function run(scenario: Scenario): { result: Observation; driverPath: string } {
  const base = process.env.CODEOPS_TMPDIR ?? tmpdir();
  mkdirSync(base, { recursive: true });
  const scratch = mkdtempSync(join(base, "guide-spec-"));
  try {
    const home = join(scratch, "home");
    const empty = join(scratch, "empty-path");
    const workspace = join(scratch, "workspace");
    const tools = join(scratch, "tools");
    for (const dir of [home, empty, workspace, tools]) mkdirSync(dir, { recursive: true });
    const entry = join(workspace, "main.clo");
    writeFileSync(entry, "module app.main;\nfunc main(): i32 { return 0; }\n");
    const shebang = `#!${process.execPath}\n`;
    const lspPath = join(tools, "clorraine-lsp");
    const driverPath = join(tools, "clorraine");
    const configuration: Record<string, string> = {
      "lsp.path": "", "driver.path": "", "toolchain.path": "", "fsUae.path": "", "kickstart.path": "",
      profile: "hosted-amiga",
    };
    if (scenario.server !== undefined) {
      const peer = readFileSync(join(here, "fixtures/guide-lsp.cjs"), "utf8").replace(/^#![^\n]*\n/, shebang);
      writeFileSync(lspPath, peer, { mode: 0o755 });
      configuration["lsp.path"] = lspPath;
    }
    if (scenario.driver) {
      writeFileSync(driverPath, `${shebang}console.log("clorraine 1.3.0");\n`, { mode: 0o755 });
      configuration["driver.path"] = driverPath;
    }
    const manifest = JSON.parse(readFileSync(join(staged, "package.json"), "utf8"));
    manifest.version = scenario.extension ?? "1.3.0";
    const packageJson = join(scratch, "package.json");
    writeFileSync(packageJson, JSON.stringify(manifest));
    const scenarioPath = join(scratch, "scenario.json");
    writeFileSync(scenarioPath, JSON.stringify({
      extension: join(staged, "out/extension.js"), packageJson, workspace, entry, configuration,
      choose: scenario.choose ?? null, restart: scenario.restart ?? false, commands: scenario.commands ?? [],
    }));
    const env: NodeJS.ProcessEnv = { ...process.env, HOME: home, USERPROFILE: home, LOCALAPPDATA: home,
      PATH: empty, GUIDE_EVENTS: join(scratch, "events"), GUIDE_SERVER_VERSION: scenario.server ?? "none" };
    delete env.CLORRAINE_LSP;
    const child = spawnSync(process.execPath, [join(here, "fixtures/guide-host.cjs"), scenarioPath], {
      encoding: "utf8", env, timeout: 15000, maxBuffer: 1024 * 1024,
    });
    assert.equal(child.status, 0, `facade error:\n${child.stderr}\n${child.stdout}`);
    const line = child.stdout.split("\n").find((item) => item.startsWith("GUIDE_OBSERVATION="));
    assert.ok(line, "facade must report observations");
    return { result: JSON.parse(line.slice("GUIDE_OBSERVATION=".length)) as Observation, driverPath };
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

test("should tell the user once per activation that the CLI is older than the extension", () => {
  const { result } = run({ server: "1.2.0", extension: "1.3.0", restart: true });
  assert.ok(result.starts >= 2, "the language server restarted after the profile change");
  const notices = result.messages.filter((item) => item.kind === "information");
  assert.deepEqual(notices, [{
    kind: "information",
    text: "Clorraine CLI 1.2.0 is older than the extension 1.3.0. Run the installer to update.",
    items: ["Open install guide"],
  }]);
});

test("should return the resolved driver path to a task", () => {
  const { result, driverPath } = run({ server: "1.3.0", driver: true, commands: ["clorraine.driverPath"] });
  assert.deepEqual(result.results["clorraine.driverPath"], { registered: true, value: driverPath });
  assert.deepEqual(result.messages, []);
});

test("should cancel the task and offer the install guide when no driver exists", () => {
  const { result } = run({ server: "1.3.0", commands: ["clorraine.driverPath"], choose: "Open install guide" });
  assert.deepEqual(result.results["clorraine.driverPath"], { registered: true, value: null });
  const errors = result.messages.filter((item) => item.kind === "error");
  assert.equal(errors.length, 1, JSON.stringify(result.messages));
  assert.ok(errors[0].text.startsWith("Clorraine: No clorraine found"), errors[0].text);
  assert.deepEqual(errors[0].items, ["Open install guide"]);
  assert.deepEqual(result.opened, [guideUrl]);
});

test("should offer the install guide when no language server exists", () => {
  const { result } = run({ driver: true });
  const errors = result.messages.filter((item) => item.kind === "error");
  assert.equal(errors.length, 1, JSON.stringify(result.messages));
  assert.ok(errors[0].text.startsWith("Clorraine: No clorraine-lsp found"), errors[0].text);
  assert.deepEqual(errors[0].items, ["Open install guide"]);
});

test("should offer the install guide when Build finds no driver", () => {
  const { result } = run({ server: "1.3.0", commands: ["clorraine.build"] });
  const errors = result.messages.filter((item) => item.kind === "error");
  assert.equal(errors.length, 1, JSON.stringify(result.messages));
  assert.ok(errors[0].text.includes("No clorraine found"), errors[0].text);
  assert.deepEqual(errors[0].items, ["Open install guide"]);
});
