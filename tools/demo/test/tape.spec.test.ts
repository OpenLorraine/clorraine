import { test } from "node:test";
import assert from "node:assert/strict";
import type { TerminalScene } from "../src/scenes.ts";
import { buildTape } from "../src/util/vhs.ts";

const scene: TerminalScene = {
  id: "run",
  kind: "terminal",
  commands: ["make run FILE=examples/hello/hello.clo", "hello"],
  duration: 10,
};

const options = { output: "build/demo/raw/run.mp4", width: 1100, height: 700, duration: 10 };

test("types each command in order, each followed by Enter and one Sleep", () => {
  const tape = buildTape(scene, options);
  const first = tape.indexOf(`Type "${scene.commands[0]}"`);
  const second = tape.indexOf(`Type "${scene.commands[1]}"`);
  assert.ok(first >= 0, "the first command is typed");
  assert.ok(second > first, "the second command is typed after the first");

  for (const command of scene.commands) {
    const block = new RegExp(`Type "${command.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}"\\nEnter\\nSleep 10s`);
    assert.match(tape, block);
  }
  assert.equal((tape.match(/Sleep 10s/g) ?? []).length, 2);
});

test("declares the output and the caption-strip-aware terminal size", () => {
  const tape = buildTape(scene, options);
  assert.match(tape, /Output "build\/demo\/raw\/run\.mp4"/);
  assert.match(tape, /Set Height 620/);
});
