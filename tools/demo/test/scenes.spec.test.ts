import { test } from "node:test";
import assert from "node:assert/strict";
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
});

test("accepts a minimal valid config", () => {
  assert.doesNotThrow(() => validateConfig({ out: "demo/out/x.mp4", scenes: [baseScene] }));
});
