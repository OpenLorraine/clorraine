import { test } from "node:test";
import assert from "node:assert/strict";
import type { EmulatorScene } from "../src/scenes.ts";
import type { RenderEnv } from "../src/renderer/env.ts";
import { emulatorCaptureOptions, frameQuery, renderEmulator } from "../src/renderer/emulator.ts";

const scene: EmulatorScene = {
  id: "hello-emulator",
  kind: "emulator",
  file: "examples/hello/hello.clo",
  caption: "Running under FS-UAE (Amiga emulator)",
};

function makeEnv(): RenderEnv {
  return {
    frames: {
      capture: async (spec) => Array.from({ length: spec.count }, () => `frame-${spec.page}.png`),
    },
    encode: { encodeFrames: async () => {} },
    root: "/tmp/clorraine-demo-spec",
    width: 1100,
    height: 700,
    scale: 1,
    outputWidth: 1100,
    outputHeight: 700,
    captionStrip: 80,
    fps: 50,
    outDir: "/tmp/clorraine-demo-spec/out",
    workDir: "/tmp/clorraine-demo-spec/work",
  };
}

test("captures the window at 50 fps for the default 7 s", () => {
  const options = emulatorCaptureOptions(scene, makeEnv());
  assert.equal(options.fps, 50);
  assert.equal(options.duration, 7);
  assert.equal(options.source, "examples/hello/hello.clo");
});

test("the branded frame is titled from the file stem and carries the caption", () => {
  assert.deepEqual(frameQuery(scene), {
    title: "hello",
    caption: "Running under FS-UAE (Amiga emulator)",
  });
});

test("records, then overlays the capture into the branded frame", async () => {
  const recorded: Array<{ source: string; duration: number; fps: number; output: string }> = [];
  const overlays: Array<{ capture: string; frame: string; out: string }> = [];
  const recorder = {
    record: async (options: { source: string; duration: number; fps: number; output: string }): Promise<void> => {
      recorded.push(options);
    },
  };
  const overlay = {
    apply: async (capture: string, frame: string, out: string): Promise<void> => {
      overlays.push({ capture, frame, out });
    },
  };

  await renderEmulator(scene, makeEnv(), { recorder, overlay });

  assert.equal(recorded.length, 1);
  assert.equal(recorded[0]?.fps, 50);
  assert.equal(recorded[0]?.duration, 7);
  assert.equal(overlays.length, 1);
  assert.match(overlays[0]?.frame ?? "", /frame\.html\.png$/);
  assert.match(overlays[0]?.out ?? "", /hello-emulator\.mp4$/);
});

test("frame: false keeps the raw capture and applies no overlay", async () => {
  let overlays = 0;
  let captures = 0;
  const recorder = { record: async (): Promise<void> => {} };
  const overlay = {
    apply: async (): Promise<void> => {
      overlays++;
    },
  };
  const env = makeEnv();
  env.frames = {
    capture: async (spec) => {
      captures++;
      return Array.from({ length: spec.count }, () => "frame.png");
    },
  };

  const out = await renderEmulator({ ...scene, frame: false }, env, { recorder, overlay });

  assert.equal(overlays, 0);
  assert.equal(captures, 0);
  assert.match(out, /hello-emulator\.mp4$/);
});
