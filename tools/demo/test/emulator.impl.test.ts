import { test } from "node:test";
import assert from "node:assert/strict";
import { buildFrameOverlayArgs, FRAME_SCREEN, frameScreenRect } from "../src/renderer/emulator.ts";

test("pins the still frame to the output rate so no capture frames are dropped", () => {
  const args = buildFrameOverlayArgs("capture.mp4", "frame.png", "out.mp4", 50);

  assert.equal(args[args.indexOf("-framerate") + 1], "50");
  assert.ok(args.includes("-loop"));
  assert.equal(args[args.indexOf("-r") + 1], "50");
  const filter = args[args.indexOf("-filter_complex") + 1] ?? "";
  assert.match(filter, /overlay=40:70:shortest=1/);
  assert.match(filter, new RegExp(`scale=${FRAME_SCREEN.width}:${FRAME_SCREEN.height}`));
  assert.equal(args.at(-1), "out.mp4");
});

test("frameScreenRect scales the design screen area onto a high-resolution frame", () => {
  assert.deepEqual(frameScreenRect(1), { ...FRAME_SCREEN });
  assert.deepEqual(frameScreenRect(2), { x: 80, y: 140, width: 2040, height: 1000 });
});
