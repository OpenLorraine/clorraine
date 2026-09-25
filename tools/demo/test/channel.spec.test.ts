import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { CHANNEL_ASSETS } from "../src/channel.ts";

const assetsDir = join(import.meta.dirname, "..", "assets");

test("the channel images meet YouTube's published sizes", () => {
  const byId = new Map(CHANNEL_ASSETS.map((asset) => [asset.id, asset]));

  assert.deepEqual(sizeOf(byId.get("banner")), { width: 2560, height: 1440 });
  assert.deepEqual(sizeOf(byId.get("avatar")), { width: 800, height: 800 });
  assert.deepEqual(sizeOf(byId.get("thumbnail")), { width: 1280, height: 720 });
});

test("every channel page exists", () => {
  for (const asset of CHANNEL_ASSETS) {
    assert.ok(existsSync(join(assetsDir, asset.page)), asset.page);
  }
});

function sizeOf(asset: { width: number; height: number } | undefined): { width: number; height: number } {
  assert.ok(asset !== undefined, "asset is defined");
  return { width: asset.width, height: asset.height };
}
