import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { loadConfig } from "../src/config.ts";

const repoRoot = join(import.meta.dirname, "..", "..", "..");

const scenes = [
  { id: "intro", kind: "card", title: "Intro", duration: 5 },
  { id: "src", kind: "code", file: "examples/hello/hello.clo", duration: 8 },
];

async function writeTemp(name: string, contents: string): Promise<string> {
  const dir = await mkdtemp(join(tmpdir(), "clorraine-demo-test-"));
  const file = join(dir, name);
  await writeFile(file, contents, "utf8");
  return file;
}

test("loads a JSON config and a TS config with the same schema identically", async () => {
  const jsonFile = await writeTemp("config.json", JSON.stringify({ out: "demo/out/x.mp4", scenes }));
  const tsFile = await writeTemp(
    "config.ts",
    `export default ${JSON.stringify({ out: "demo/out/x.mp4", scenes })};\n`,
  );

  const fromJson = await loadConfig(jsonFile, { root: repoRoot });
  const fromTs = await loadConfig(tsFile, { root: repoRoot });

  assert.deepEqual(
    fromJson.scenes.map((scene) => ({ id: scene.id, kind: scene.kind })),
    fromTs.scenes.map((scene) => ({ id: scene.id, kind: scene.kind })),
  );
});

test("rejects a scene file that escapes the repository root", async () => {
  const file = await writeTemp(
    "config.json",
    JSON.stringify({ out: "demo/out/x.mp4", scenes: [{ id: "bad", kind: "code", file: "../../etc/passwd", duration: 5 }] }),
  );
  await assert.rejects(() => loadConfig(file, { root: repoRoot }), /escape|outside|root/i);
});

test("rejects an absolute output path", async () => {
  const file = await writeTemp(
    "config.json",
    JSON.stringify({ out: "/tmp/clorraine-demo-x.mp4", scenes: [{ id: "one", kind: "card", title: "T", duration: 5 }] }),
  );
  await assert.rejects(() => loadConfig(file, { root: repoRoot }), /absolute|root|outside/i);
});
