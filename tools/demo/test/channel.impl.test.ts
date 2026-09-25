import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { CHANNEL_ASSETS, renderChannelAssets } from "../src/channel.ts";
import type { FrameSource, FrameSpec } from "../src/renderer/env.ts";

async function makeRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), "clorraine-channel-"));
}

test("renderChannelAssets captures each page at its size and copies the image", async () => {
  const root = await makeRoot();
  const outDir = join(root, "channel");
  const specs: FrameSpec[] = [];
  const frames: FrameSource = {
    async capture(spec) {
      specs.push(spec);
      const dir = join(root, "frames", spec.id);
      await mkdir(dir, { recursive: true });
      const file = join(dir, "00000.png");
      await writeFile(file, `png-${spec.id}`, "utf8");
      return [file];
    },
  };

  const written = await renderChannelAssets(frames, outDir);

  assert.equal(written.length, CHANNEL_ASSETS.length);
  for (const asset of CHANNEL_ASSETS) {
    assert.equal(await readFile(join(outDir, asset.file), "utf8"), `png-${asset.id}`);
  }
  assert.deepEqual(
    specs.map((spec) => [spec.id, spec.width, spec.height]),
    [
      ["banner", 2560, 1440],
      ["avatar", 800, 800],
      ["thumbnail", 1280, 720],
    ],
  );
  assert.equal(specs[2]?.query?.title, "Clorraine");
});

test("renderChannelAssets fails when a page yields no image", async () => {
  const root = await makeRoot();
  const frames: FrameSource = {
    async capture() {
      return [];
    },
  };

  await assert.rejects(() => renderChannelAssets(frames, join(root, "channel")), /no image/);
});
