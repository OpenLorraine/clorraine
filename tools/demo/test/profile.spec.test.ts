import { test } from "node:test";
import assert from "node:assert/strict";
import { contentWidth, DEFAULT_PROFILE, profileForHeight } from "../src/renderer/profile.ts";
import { ConfigError } from "../src/scenes.ts";

test("the default profile maps the design one-to-one onto the output", () => {
  assert.deepEqual(DEFAULT_PROFILE, {
    designWidth: 1100,
    designHeight: 700,
    scale: 1,
    outputWidth: 1100,
    outputHeight: 700,
    captionStrip: 80,
  });
});

test("a 1080 profile is 16:9, rendered at twice size, and narrower than the canvas", () => {
  const profile = profileForHeight(1080);

  assert.equal(profile.outputWidth, 1920);
  assert.equal(profile.outputHeight, 1080);
  assert.equal(profile.scale, 2);
  assert.equal(profile.captionStrip, Math.round((80 * 1080) / 700));
  assert.equal(contentWidth(profile), 1698);
  assert.ok(contentWidth(profile) < profile.outputWidth, "the design is pillarboxed");
});

test("a 720 profile is 1280x720", () => {
  const profile = profileForHeight(720);

  assert.equal(profile.outputWidth, 1280);
  assert.equal(profile.outputHeight, 720);
  assert.equal(profile.captionStrip, Math.round((80 * 720) / 700));
});

test("profileForHeight rejects anything that is not a positive even height", () => {
  for (const size of [0, -1080, 1081, 12.5, Number.NaN]) {
    assert.throws(() => profileForHeight(size), ConfigError, `size ${size}`);
  }
});
