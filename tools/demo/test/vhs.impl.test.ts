import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { buildTape, ensureVhsOutput, escapeTapeValue } from "../src/util/vhs.ts";

test("escapes quotes, backslashes, and newlines", () => {
  assert.equal(escapeTapeValue('say "hi"'), 'say \\"hi\\"');
  assert.equal(escapeTapeValue("a\\b"), "a\\\\b");
  assert.equal(escapeTapeValue("one\ntwo"), "one two");
});

test("a command with a quote stays on one Type line", () => {
  const tape = buildTape(
    { id: "quoted", kind: "terminal", commands: ['echo "hi"', "true"] },
    { output: "out.mp4", width: 1100, height: 700, duration: 3 },
  );
  assert.match(tape, /Type "echo \\"hi\\""\nEnter\nSleep 3s/);
  assert.match(tape, /Type "true"\nEnter\nSleep 3s/);
});

test("reports the v0.11.0 hint when VHS wrote nothing", async () => {
  const dir = await mkdtemp(join(tmpdir(), "clorraine-vhs-"));
  await assert.rejects(() => ensureVhsOutput(join(dir, "missing.mp4")), /v0\.11\.0/);
});

test("accepts a non-empty output", async () => {
  const dir = await mkdtemp(join(tmpdir(), "clorraine-vhs-"));
  const file = join(dir, "clip.mp4");
  await writeFile(file, "data");
  await assert.doesNotReject(() => ensureVhsOutput(file));
});
