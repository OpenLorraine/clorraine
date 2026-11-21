/**
 * Implementation controls for configuration narrowing and asynchronous task
 * ownership. The child uses real LSP framing and the external editor API
 * boundary; it does not establish installed compiler or desktop acceptance.
 */
import { strict as assert } from "node:assert";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const staged = resolve(here, "../../../build/vscode/extension/out");

/** Run one isolated boundary observation and always remove its private files. */
function observe(scenario: Record<string, unknown>): Record<string, unknown> {
  const base = process.env.CODEOPS_TMPDIR ?? tmpdir();
  mkdirSync(base, { recursive: true });
  const scratch = mkdtempSync(join(base, "editor-ownership-"));
  try {
    const entry = join(scratch, "main.clo");
    writeFileSync(entry, "module app.main;\nfunc main(): i32 { return 0; }\n");
    const config = join(scratch, "scenario.json");
    writeFileSync(config, JSON.stringify({ ...scenario, scratch, staged, entry }));
    const child = spawnSync(process.execPath, [join(here, "fixtures/actions-impl-host.cjs"), config], {
      encoding: "utf8", timeout: 12000, maxBuffer: 1024 * 1024,
      env: { ...process.env, TMPDIR: scratch, CODEOPS_TMPDIR: scratch },
    });
    assert.equal(child.error, undefined, child.error?.message);
    assert.equal(child.status, 0, `boundary fixture failed:\n${child.stderr}\n${child.stdout}`);
    const value: unknown = JSON.parse(child.stdout);
    assert.ok(typeof value === "object" && value !== null && !Array.isArray(value));
    return Object.fromEntries(Object.entries(value));
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

for (const [name, fields, expected] of [
  ["missing inspection", undefined, ""],
  ["empty inspection", {}, ""],
  ["default-only path", { defaultValue: "/default/tool" }, "/default/tool"],
  ["explicit empty user value", { defaultValue: "/default/tool", globalValue: "" }, ""],
  ["literal filename bytes", { globalValue: "/tools ; literal/tool " }, "/tools ; literal/tool "],
  ["workspace and language values", {
    defaultValue: "", workspaceValue: "/workspace/tool", workspaceFolderValue: "/folder/tool",
    globalLanguageValue: "/language/tool", workspaceLanguageValue: "/workspace-language/tool",
    workspaceFolderLanguageValue: "/folder-language/tool",
  }, ""],
] as const) {
  test(`should narrow ${name} without merging executable configuration`, () => {
    const actual = observe({ mode: "setting", fields });
    assert.equal(actual.value, expected);
    assert.equal(actual.error, undefined);
  });
}

for (const [name, value] of [
  ["null", null], ["boolean", false], ["number", 0], ["array", []], ["object", {}],
] as const) {
  test(`should reject an explicit ${name} user path instead of falling back`, () => {
    const actual = observe({ mode: "setting", fields: {
      globalValue: value, defaultValue: "/otherwise-valid/default",
    } });
    assert.equal(typeof actual.error, "string", "invalid explicit values must fail closed");
    assert.match(String(actual.error), /clorraine\.driver\.path.*valid path string/i);
    assert.equal(actual.value, undefined);
  });
}

test("should reject an invalid present default rather than silently manufacture an empty path", () => {
  const actual = observe({ mode: "setting", fields: { defaultValue: null } });
  assert.equal(typeof actual.error, "string");
  assert.match(String(actual.error), /clorraine\.driver\.path.*valid path string/i);
  assert.equal(actual.value, undefined);
});

for (const [scenario, tasks, terminated, requests] of [
  ["running", 1, 1, 1],
  ["completed", 1, 0, 1],
  ["unrelated", 1, 1, 1],
  ["pending-execution", 1, 1, 1],
  ["pending-execution-error", 1, 0, 1],
  ["pending-selection", 0, 0, 1],
  ["rejected-selection", 0, 0, 1],
  ["pending-save", 0, 0, 0],
  ["reactivate", 1, 1, 1],
] as const) {
  test(`should retain correct task ownership when disposal meets ${scenario}`, () => {
    const actual = observe({ mode: "ownership", scenario });
    assert.equal(actual.tasks, tasks, "only a live action may submit a task");
    assert.equal(actual.terminated, terminated, "stop only unfinished tasks owned by this activation");
    assert.equal(actual.selectionRequests, requests, "disposed saves must not start selection");
    assert.equal(actual.unrelatedTerminated, 0, "another extension's task is never owned");
    assert.equal(actual.commandsAfterDispose, 0, "command registrations must be disposable");
    assert.equal(actual.errors, 0, "normal deactivation must not show a late action error");
  });
}
