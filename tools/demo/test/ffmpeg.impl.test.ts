import { test } from "node:test";
import assert from "node:assert/strict";
import { encodeFramesArgs } from "../src/util/ffmpeg.ts";

test("encodes a zero-padded PNG pattern at the requested frame rate", () => {
  const args = encodeFramesArgs(["/tmp/frames/intro/00000.png"], "/tmp/out.mp4", 50);
  assert.equal(args[args.indexOf("-framerate") + 1], "50");
  assert.ok(args.includes("/tmp/frames/intro/%05d.png"));
  assert.equal(args.at(-1), "/tmp/out.mp4");
});

test("rejects an empty frame list", () => {
  assert.throws(() => encodeFramesArgs([], "x.mp4", 30), /empty/i);
});
