import { test } from "node:test";
import assert from "node:assert/strict";
import { rm } from "node:fs/promises";
import { join } from "node:path";
import { ChromeFrameSource } from "../src/util/chrome.ts";
import { FfmpegEncoder } from "../src/util/ffmpeg.ts";
import { run } from "../src/util/proc.ts";
import { compose } from "../src/renderer/compose.ts";
import type { RenderEnv } from "../src/renderer/env.ts";
import type { DemoConfig } from "../src/scenes.ts";

const enabled = process.env.DEMO_E2E === "1";
const repoRoot = join(import.meta.dirname, "..", "..", "..");
const smokeDir = join(repoRoot, "build", "demo", "smoke");

interface Probe {
  streams: Array<{ codec_type: string; width?: number; height?: number; r_frame_rate?: string }>;
  format: { duration: string };
}

test(
  "renders a card(4s)+code(3s) story to a 6.5s 1100x700 clip",
  { skip: !enabled, timeout: 180_000 },
  async () => {
    await rm(smokeDir, { recursive: true, force: true });
    const env: RenderEnv = {
      frames: new ChromeFrameSource({
        assetsDir: join(repoRoot, "tools", "demo", "assets"),
        framesDir: join(smokeDir, "frames"),
      }),
      encode: new FfmpegEncoder(),
      root: repoRoot,
      width: 1100,
      height: 700,
      scale: 1,
      outputWidth: 1100,
      outputHeight: 700,
      captionStrip: 80,
      fps: 50,
      outDir: join(smokeDir, "out"),
      workDir: join(smokeDir, "work"),
    };
    const config: DemoConfig = {
      out: "build/demo/smoke/out/smoke.mp4",
      width: 1100,
      height: 700,
      fps: 50,
      transition: 0.5,
      scenes: [
        { id: "intro", kind: "card", title: "Intro", duration: 4 },
        { id: "code", kind: "code", file: "examples/hello/hello.clo", duration: 3 },
      ],
    };

    const out = await compose(config, env);
    const probeResult = await run("ffprobe", [
      "-v", "error",
      "-show_entries", "stream=codec_type,width,height,r_frame_rate",
      "-show_entries", "format=duration",
      "-of", "json",
      out,
    ]);
    assert.equal(probeResult.code, 0, probeResult.stderr);

    const probe = JSON.parse(probeResult.stdout) as Probe;
    const video = probe.streams.filter((stream) => stream.codec_type === "video");
    const audio = probe.streams.filter((stream) => stream.codec_type === "audio");
    assert.equal(video.length, 1);
    assert.equal(audio.length, 0);
    assert.equal(video[0]?.width, 1100);
    assert.equal(video[0]?.height, 700);
    assert.equal(video[0]?.r_frame_rate, "50/1");
    assert.ok(Math.abs(Number(probe.format.duration) - 6.5) < 0.2, `duration ${probe.format.duration}`);
  },
);
