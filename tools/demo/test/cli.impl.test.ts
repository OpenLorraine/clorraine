import { test } from "node:test";
import assert from "node:assert/strict";
import { join } from "node:path";
import { run } from "../src/util/proc.ts";

const cli = join(import.meta.dirname, "..", "src", "cli.ts");

test("--help prints usage and exits 0", async () => {
  const result = await run(process.execPath, [cli, "--help"], { stdio: "ignore" });
  assert.equal(result.code, 0);
  assert.match(result.stderr, /Usage: clorraine-demo/);
});

test("an unknown option exits 2", async () => {
  const result = await run(process.execPath, [cli, "list", "x", "--bogus"], { stdio: "ignore" });
  assert.equal(result.code, 2);
});

test("a missing config path exits 2", async () => {
  const result = await run(process.execPath, [cli, "list"], { stdio: "ignore" });
  assert.equal(result.code, 2);
});
