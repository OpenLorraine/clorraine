import { mkdir } from "node:fs/promises";
import { basename, dirname, join } from "node:path";
import { sceneDuration } from "../scenes.ts";
import type { EmulatorScene } from "../scenes.ts";
import { runOrThrow } from "../util/proc.ts";
import type { RenderEnv } from "./env.ts";

/** The region of the branded frame the emulator capture is drawn into. */
export const FRAME_SCREEN = { x: 40, y: 70, width: 1020, height: 500 } as const;

/** The capture parameters for one emulator scene. */
export interface EmulatorCaptureOptions {
  /** Repository-relative source file to build and run. */
  source: string;
  /** Seconds to record. */
  duration: number;
  /** Capture frame rate, in frames per second. */
  fps: number;
}

/** The per-scene capture parameters, applying the kind defaults. */
export function emulatorCaptureOptions(scene: EmulatorScene, env: RenderEnv): EmulatorCaptureOptions {
  return { source: scene.file, duration: sceneDuration(scene), fps: env.fps };
}

/** The branded-frame page parameters; the title is the source file stem. */
export function frameQuery(scene: EmulatorScene): { title: string; caption: string } {
  return { title: basename(scene.file, ".clo"), caption: scene.caption ?? "" };
}

/** Records the emulator window to a raw clip. */
export interface EmulatorRecorder {
  record(options: EmulatorCaptureOptions & { output: string }): Promise<void>;
}

/** Builds the ffmpeg arguments that composite a capture into the frame's screen area. */
export function buildFrameOverlayArgs(capture: string, frame: string, out: string, fps: number): string[] {
  const screen = FRAME_SCREEN;
  const filter =
    `[1:v]scale=${screen.width}:${screen.height}:force_original_aspect_ratio=decrease,` +
    `pad=${screen.width}:${screen.height}:(ow-iw)/2:(oh-ih)/2:color=black[c];` +
    `[0:v][c]overlay=${screen.x}:${screen.y}:shortest=1`;
  // The still frame is a single image; without `-loop 1 -framerate` it defaults to
  // 25 fps and throttles the overlay, discarding half of a 50 fps capture.
  return [
    "-v", "error", "-y",
    "-loop", "1", "-framerate", String(fps), "-i", frame,
    "-i", capture,
    "-filter_complex", filter,
    "-r", String(fps),
    "-pix_fmt", "yuv420p", "-c:v", "libx264", "-crf", "20", "-an",
    out,
  ];
}

/** Composites a raw emulator capture into a rendered frame PNG. */
export interface FrameOverlay {
  apply(capture: string, frame: string, out: string, options: { fps: number }): Promise<void>;
}

/** Records with the repository's `demo/record-emulator.sh`, which drives FS-UAE. */
export class ScriptEmulatorRecorder implements EmulatorRecorder {
  private readonly root: string;

  constructor(root: string) {
    this.root = root;
  }

  async record(options: EmulatorCaptureOptions & { output: string }): Promise<void> {
    await runOrThrow(
      join(this.root, "demo", "record-emulator.sh"),
      [options.source, options.output, String(options.duration), String(options.fps)],
      { cwd: this.root },
    );
  }
}

/** Scales the capture into the frame's screen area with ffmpeg. */
export class FfmpegFrameOverlay implements FrameOverlay {
  async apply(capture: string, frame: string, out: string, options: { fps: number }): Promise<void> {
    await runOrThrow("ffmpeg", buildFrameOverlayArgs(capture, frame, out, options.fps), { stdio: "ignore" });
  }
}

/** Injectable dependencies for {@link renderEmulator}. */
export interface EmulatorDeps {
  recorder?: EmulatorRecorder;
  overlay?: FrameOverlay;
}

/**
 * Renders an emulator scene: the source is built and captured under FS-UAE at the
 * output frame rate, then composited into the branded frame.
 *
 * @returns the clip path.
 */
export async function renderEmulator(
  scene: EmulatorScene,
  env: RenderEnv,
  deps: EmulatorDeps = {},
): Promise<string> {
  const capture = emulatorCaptureOptions(scene, env);
  const recorder = deps.recorder ?? new ScriptEmulatorRecorder(env.root);
  const out = join(env.outDir, `${scene.id}.mp4`);
  await mkdir(env.outDir, { recursive: true });

  if (scene.frame === false) {
    await recorder.record({ ...capture, output: out });
    return out;
  }

  const raw = join(env.workDir, "raw", `${scene.id}.mp4`);
  await mkdir(dirname(raw), { recursive: true });
  await recorder.record({ ...capture, output: raw });

  const frames = await env.frames.capture({
    id: `${scene.id}-frame`,
    page: "frame.html",
    query: frameQuery(scene),
    width: env.width,
    height: env.height,
    count: 1,
    fps: env.fps,
  });
  const frame = frames[0];
  if (frame === undefined) {
    throw new Error("the branded frame produced no image");
  }
  const overlay = deps.overlay ?? new FfmpegFrameOverlay();
  await overlay.apply(raw, frame, out, { fps: env.fps });
  return out;
}
