import { test } from "node:test";
import assert from "node:assert/strict";
import { join } from "node:path";
import { loadConfig } from "../src/config.ts";
import { defaultRepoRoot } from "../src/util/paths.ts";

const root = defaultRepoRoot();
const story = join(root, "demo/story.ts");

test("the story is the fifteen chapters in order with the expected kinds", async () => {
  const config = await loadConfig(story, { root });

  assert.deepEqual(
    config.scenes.map((scene) => [scene.id, scene.kind]),
    [
      ["intro", "card"],
      ["hello-src", "code"],
      ["hello-emu", "emulator"],
      ["hello-explain", "card"],
      ["real", "card"],
      ["source", "code"],
      ["own-explain", "card"],
      ["ownership", "terminal"],
      ["build-explain", "card"],
      ["run", "terminal"],
      ["emulator", "emulator"],
      ["draw-explain", "card"],
      ["graphics", "emulator"],
      ["los", "card"],
      ["outro", "outro"],
    ],
  );
});

test("the story wires the full video, the poster, and the opening gif", async () => {
  const config = await loadConfig(story, { root });

  assert.equal(config.out, "demo/demo-full.mp4");
  assert.equal(config.poster, "demo/poster.png");
  assert.equal(config.gif?.out, "demo/demo.gif");
  assert.deepEqual(config.gif?.sceneIds, ["intro", "hello-src", "hello-emu", "hello-explain"]);
});

test("the emulator chapters are framed and use the FS-UAE wording", async () => {
  const config = await loadConfig(story, { root });
  const emulatorScenes = config.scenes.filter((scene) => scene.kind === "emulator");

  assert.equal(emulatorScenes.length, 3);
  for (const scene of emulatorScenes) {
    assert.equal(scene.kind === "emulator" && scene.frame, true);
    assert.match(scene.caption ?? "", /FS-UAE/);
    assert.doesNotMatch(scene.caption ?? "", /real Amiga/i);
  }
});
