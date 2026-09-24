import { dirname, join } from "node:path";
import type { Encoder } from "../renderer/env.ts";
import { run, runOrThrow } from "./proc.ts";

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

/**
 * Reads a media file's duration in seconds with ffprobe.
 *
 * @throws {Error} when ffprobe fails or reports a non-numeric duration.
 */
export async function ffprobeDuration(file: string): Promise<number> {
  const result = await run(
    "ffprobe",
    ["-v", "error", "-show_entries", "format=duration", "-of", "default=nw=1:nk=1", file],
    { stdio: "ignore" },
  );
  if (result.code !== 0) {
    throw new Error(`ffprobe failed for ${file}: ${result.stderr.trim()}`);
  }
  const text = result.stdout.trim();
  if (text === "") {
    throw new Error(`ffprobe reported no duration for ${file}`);
  }
  const duration = Number(text);
  if (!Number.isFinite(duration)) {
    throw new Error(`ffprobe reported no duration for ${file}`);
  }
  return duration;
}
