import { test } from "node:test";
import assert from "node:assert/strict";
import { compose, expectedTotal, posterScene, xfadeFilter, xfadeOffsets } from "../src/renderer/compose.ts";
import type { RenderEnv } from "../src/renderer/env.ts";
import type { DemoConfig } from "../src/scenes.ts";
import { ConfigError, validateConfig } from "../src/scenes.ts";

test("chains one xfade per boundary with the running-length offsets", () => {
  assert.deepEqual(xfadeOffsets([5, 4, 6], 0.5), [4.5, 8.0]);
  assert.equal(expectedTotal([5, 4, 6], 0.5), 14.0);

  const graph = xfadeFilter([5, 4, 6], 0.5);
  assert.match(graph, /offset=4\.5/);
  assert.match(graph, /offset=8\.0/);
  assert.equal((graph.match(/xfade=/g) ?? []).length, 2);
});

test("a single scene skips the xfade chain", () => {
  assert.deepEqual(xfadeOffsets([5], 0.5), []);
  assert.equal(expectedTotal([5], 0.5), 5.0);
  assert.doesNotMatch(xfadeFilter([5], 0.5), /xfade/);
});

test("rejects a gif that names an unknown scene at load time", () => {
  assert.throws(
    () =>
      validateConfig({
        out: "demo/out/x.mp4",
        gif: { out: "demo/out/x.gif", sceneIds: ["missing"] },
        scenes: [{ id: "intro", kind: "card", title: "Intro", duration: 5 }],
      }),
    /gif/i,
  );
});

test("the poster falls back to the first scene when there is no card", () => {
  const withCard: DemoConfig = {
    out: "demo/out/x.mp4",
    scenes: [
      { id: "src", kind: "code", file: "examples/hello/hello.clo", duration: 5 },
      { id: "intro", kind: "card", title: "Intro", duration: 5 },
    ],
  };
  assert.equal(posterScene(withCard).id, "intro");

  const noCard: DemoConfig = {
    out: "demo/out/x.mp4",
    scenes: [{ id: "src", kind: "code", file: "examples/hello/hello.clo", duration: 5 }],
  };
  assert.equal(posterScene(noCard).id, "src");
});

test("an unknown --only id is a config error and fails before any rendering", async () => {
  const config: DemoConfig = {
    out: "demo/out/x.mp4",
    scenes: [{ id: "intro", kind: "card", title: "Intro" }],
  };
  let captured = false;
  const env: RenderEnv = {
    frames: {
      capture: async () => {
        captured = true;
        return [];
      },
    },
    encode: { encodeFrames: async () => {} },
    root: "/tmp",
    width: 1100,
    height: 700,
    fps: 50,
    outDir: "/tmp/out",
    workDir: "/tmp/work",
  };

  await assert.rejects(() => compose(config, env, { only: ["nope"] }), ConfigError);
  assert.equal(captured, false);
});
