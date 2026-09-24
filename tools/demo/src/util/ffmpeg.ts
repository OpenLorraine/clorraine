import { dirname, join } from "node:path";
import type { Encoder } from "../renderer/env.ts";
import { runOrThrow } from "./proc.ts";

/**
 * Builds the ffmpeg arguments that encode a zero-padded PNG sequence into an H.264 MP4.
 *
 * @param frames - Ordered frame paths; the directory and `%05d.png` pattern are used.
 * @param out - Output MP4 path.
 * @param fps - Output frame rate.
 * @throws {Error} when `frames` is empty.
 */
export function encodeFramesArgs(frames: string[], out: string, fps: number): string[] {
  const first = frames[0];
  if (first === undefined) {
    throw new Error("cannot encode an empty frame list");
  }
  const pattern = join(dirname(first), "%05d.png");
  return ["-v", "error", "-y", "-framerate", String(fps), "-i", pattern, "-pix_fmt", "yuv420p", "-c:v", "libx264", "-crf", "20", out];
}

/** Encodes PNG frame sequences with ffmpeg. */
export class FfmpegEncoder implements Encoder {
  async encodeFrames(frames: string[], out: string, options: { fps: number }): Promise<void> {
    await runOrThrow("ffmpeg", encodeFramesArgs(frames, out, options.fps), { stdio: "ignore" });
  }
}
