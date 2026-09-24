import { test } from "node:test";
import assert from "node:assert/strict";
import { join } from "node:path";
import { loadConfig } from "../src/config.ts";
import { sceneDuration } from "../src/scenes.ts";
import type { Scene } from "../src/scenes.ts";
import { defaultRepoRoot } from "../src/util/paths.ts";

const root = defaultRepoRoot();
const story = join(root, "demo/story.ts");

/** The source file of a code or emulator scene, or undefined for any other scene. */
function sourceFile(scene: Scene | undefined): string | undefined {
  return scene !== undefined && (scene.kind === "code" || scene.kind === "emulator") ? scene.file : undefined;
}

test("every recorded chapter carries a caption and the cards carry a title", async () => {
  const config = await loadConfig(story, { root });

  for (const scene of config.scenes) {
    if (scene.kind === "card" || scene.kind === "outro") {
      assert.ok(scene.title.length > 0, `${scene.id} has a title`);
    } else {
      assert.ok((scene.caption ?? "").length > 0, `${scene.id} has a caption`);
    }
  }
});

test("the code and emulator chapters point at the expected sources", async () => {
  const config = await loadConfig(story, { root });
  const byId = new Map(config.scenes.map((scene) => [scene.id, scene]));

  assert.equal(sourceFile(byId.get("hello-src")), "examples/hello/hello.clo");
  assert.equal(sourceFile(byId.get("source")), "spec/tests/corpus/string-parser/program.clo");
  assert.equal(sourceFile(byId.get("graphics")), "spec/tests/corpus/graphics/program.clo");
});

test("the long source chapter scrolls so the whole program is shown", async () => {
  const config = await loadConfig(story, { root });
  const source = config.scenes.find((scene) => scene.id === "source");
  assert.ok(source !== undefined && source.kind === "code");
  assert.equal(source.scroll, true);
});

test("the story keeps the per-kind default durations (no per-scene override)", async () => {
  const config = await loadConfig(story, { root });

  for (const scene of config.scenes) {
    assert.equal(scene.duration, undefined, `${scene.id} uses the kind default`);
    assert.ok(sceneDuration(scene) > 0);
  }
});
