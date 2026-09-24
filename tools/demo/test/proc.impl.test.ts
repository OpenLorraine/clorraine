import { test } from "node:test";
import assert from "node:assert/strict";
import { commandExists, run, runOrThrow } from "../src/util/proc.ts";

test("runs a command and captures stdout", async () => {
  const result = await run("printf", ["hello"], { stdio: "ignore" });
  assert.equal(result.code, 0);
  assert.equal(result.stdout, "hello");
});

test("runOrThrow throws on a non-zero exit", async () => {
  await assert.rejects(() => runOrThrow("false", [], { stdio: "ignore" }), /exited with code/);
});

test("commandExists finds a real command and rejects a bogus one", async () => {
  assert.equal(await commandExists("sh"), true);
  assert.equal(await commandExists("definitely-not-a-command-xyz"), false);
});
