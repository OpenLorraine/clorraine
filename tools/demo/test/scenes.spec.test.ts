import { test } from "node:test";
import assert from "node:assert/strict";
import { renderCard } from "../src/renderer/card.ts";
import type { RenderEnv } from "../src/renderer/env.ts";
import { validateConfig } from "../src/scenes.ts";

const baseScene = { id: "one", kind: "card", title: "Title", duration: 7 } as const;

test("rejects a config with no scenes", () => {
  assert.throws(
    () => validateConfig({ out: "demo/out/x.mp4", scenes: [] }),
    /at least one scene|scenes/i,
  );
});

test("rejects duplicate scene ids and names them", () => {
  assert.throws(
    () =>
      validateConfig({
        out: "demo/out/x.mp4",
        scenes: [baseScene, { ...baseScene, title: "Other" }],
      }),
    (error: unknown) => {
      assert.ok(error instanceof Error);
      return /duplicate/i.test(error.message) && error.message.includes("one");
    },
  );
});

test("rejects a non-positive duration", () => {
  assert.throws(
    () => validateConfig({ out: "demo/out/x.mp4", scenes: [{ ...baseScene, duration: 0 }] }),
    /duration/i,
  );
});

test("reports every problem at once, each tied to its scene", () => {
  let message = "";
  try {
    validateConfig({
      out: "demo/out/x.mp4",
      scenes: [
        { id: "a", kind: "card", title: "A", duration: -1 },
        { id: "a", kind: "card", title: "B", duration: 2 },
      ],
    });
  } catch (error) {
    message = error instanceof Error ? error.message : String(error);
  }
  assert.match(message, /duration/i);
  assert.match(message, /duplicate/i);
  assert.match(message, /"a"/);
});

test("accepts a minimal valid config", () => {
  assert.doesNotThrow(() => validateConfig({ out: "demo/out/x.mp4", scenes: [baseScene] }));
});

test("rejects a non-string poster path", () => {
  assert.throws(
    () => validateConfig({ out: "demo/out/x.mp4", poster: 123, scenes: [baseScene] }),
    /poster/i,
  );
});

test("rejects a gif that names an unknown scene", () => {
  assert.throws(
    () =>
      validateConfig({
        out: "demo/out/x.mp4",
        gif: { out: "demo/out/x.gif", sceneIds: ["missing"] },
        scenes: [baseScene],
      }),
    /gif/i,
  );
});

test("a card scene requests one frame per output frame", async () => {
  const specs: Array<{ page: string; count: number }> = [];
  const frames = {
    capture: async (spec: { page: string; count: number }): Promise<string[]> => {
      specs.push(spec);
      return Array.from({ length: spec.count }, (_, i) => `frame-${i}.png`);
    },
  };
  const encode = { encodeFrames: async (): Promise<void> => {} };
  const env: RenderEnv = { frames, encode, root: "/tmp/clorraine-demo-spec", width: 1100, height: 700, scale: 1, outputWidth: 1100, outputHeight: 700, captionStrip: 80, fps: 50, outDir: "/tmp/clorraine-demo-spec", workDir: "/tmp/clorraine-demo-spec/work" };

  await renderCard({ id: "intro", kind: "card", title: "Title", duration: 6 }, env);

  assert.equal(specs.length, 1);
  assert.equal(specs[0]?.page, "card.html");
  assert.equal(specs[0]?.count, 300);
});
