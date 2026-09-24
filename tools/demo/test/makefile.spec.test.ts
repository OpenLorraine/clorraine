import { test } from "node:test";
import assert from "node:assert/strict";
import { defaultRepoRoot } from "../src/util/paths.ts";
import { run } from "../src/util/proc.ts";

const root = defaultRepoRoot();

async function dryRun(target: string): Promise<string> {
  const result = await run("make", ["-n", target], { cwd: root, stdio: "ignore" });
  assert.equal(result.code, 0, `make -n ${target} failed:\n${result.stderr}`);
  return result.stdout + result.stderr;
}

test("make -n demo installs dependencies before rendering the story", async () => {
  const out = await dryRun("demo");

  const install = out.indexOf("npm ci");
  const render = out.indexOf("render demo/story.ts");
  assert.notEqual(install, -1, out);
  assert.notEqual(render, -1, out);
  assert.ok(install < render, "npm ci must run before the render");
  assert.match(out, /--only emulator --out demo\/emulator\.mp4/);
  assert.doesNotMatch(out, /node --test/);
});

test("make -n demo-check type-checks and runs the tool tests once", async () => {
  const out = await dryRun("demo-check");

  assert.match(out, /tsc --noEmit/);
  assert.equal((out.match(/node --test/g) ?? []).length, 1);
});
