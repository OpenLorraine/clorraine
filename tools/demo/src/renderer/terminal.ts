import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { sceneDuration } from "../scenes.ts";
import type { TerminalScene } from "../scenes.ts";
import { buildTape, VhsRunner } from "../util/vhs.ts";
import type { VhsRecorder } from "../util/vhs.ts";
import { CAPTION_STRIP } from "./env.ts";
import type { RenderEnv } from "./env.ts";

/**
 * Records a terminal scene with VHS and returns the raw clip path. The clip is not a
 * finished canvas yet: the composer scales it and adds the caption strip, because VHS
 * cannot draw the caption inside its own recording.
 *
 * @param recorder - VHS runner; tests inject a fake.
 * @throws {Error} when VHS produces no output.
 */
export async function renderTerminal(
  scene: TerminalScene,
  env: RenderEnv,
  recorder: VhsRecorder = new VhsRunner(),
): Promise<string> {
  const tapePath = join(env.workDir, "tape", `${scene.id}.tape`);
  const output = join(env.workDir, "raw", `${scene.id}.mp4`);
  await mkdir(dirname(tapePath), { recursive: true });
  await mkdir(dirname(output), { recursive: true });

  const tape = buildTape(scene, {
    output: relative(env.root, output),
    width: env.width * env.scale,
    height: env.height * env.scale,
    captionStrip: CAPTION_STRIP * env.scale,
    fontSize: 16 * env.scale,
    duration: sceneDuration(scene),
  });
  await writeFile(tapePath, tape, "utf8");
  await recorder.record(tapePath, output, env.root);
  return output;
}
