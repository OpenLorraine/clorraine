import { existsSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { ConfigError, sceneDuration } from "../scenes.ts";
import type { DemoConfig, Scene } from "../scenes.ts";
import { ffprobeDuration } from "../util/ffmpeg.ts";
import { resolveRepoPath } from "../util/paths.ts";
import { runOrThrow } from "../util/proc.ts";
import type { VhsRecorder } from "../util/vhs.ts";
import { CAPTION_STRIP, contentHeight } from "./env.ts";
import type { RenderEnv } from "./env.ts";
import { renderCard } from "./card.ts";
import { renderCode } from "./code.ts";
import { renderEmulator } from "./emulator.ts";
import type { EmulatorDeps } from "./emulator.ts";
import { renderTerminal } from "./terminal.ts";
import { renderPoster } from "./poster.ts";

/** The canvas background, matching the HTML pages. */
export const BG_COLOR = "0x1e1e2e";

/** The font used for terminal captions. */
export const CAPTION_FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";

/** How far the composed length may differ from the expected length, in seconds. */
const DURATION_TOLERANCE = 0.25;

/** Rounds to milliseconds so xfade offsets do not carry float noise. */
function round(value: number): number {
  return Math.round(value * 1000) / 1000;
}

/** Formats a number of seconds for an ffmpeg argument, keeping up to three decimals. */
export function formatSeconds(value: number): string {
  const fixed = value.toFixed(3).replace(/0+$/, "");
  return fixed.endsWith(".") ? `${fixed}0` : fixed;
}

/**
 * The `xfade` offsets for a chain of clips. Each boundary's offset is the running
 * output length minus the transition, so the clips overlap by exactly `transition`.
 *
 * @returns one offset per boundary; empty for fewer than two clips.
 */
export function xfadeOffsets(durations: number[], transition: number): number[] {
  if (durations.length < 2) {
    return [];
  }
  const offsets: number[] = [];
  let running = durations[0] ?? 0;
  for (let index = 1; index < durations.length; index++) {
    const offset = round(running - transition);
    offsets.push(offset);
    running = offset + (durations[index] ?? 0);
  }
  return offsets;
}

/** The final duration of a crossfaded chain: the sum minus one transition per boundary. */
export function expectedTotal(durations: number[], transition: number): number {
  if (durations.length === 0) {
    return 0;
  }
  if (durations.length === 1) {
    return durations[0] ?? 0;
  }
  const sum = durations.reduce((total, value) => total + value, 0);
  return round(sum - (durations.length - 1) * transition);
}

/**
 * Builds the `xfade` filtergraph that chains every clip. `xfade` takes two inputs, so
 * the graph feeds each stage's output into the next.
 *
 * @returns the filtergraph, or an empty string when no crossfade is needed.
 */
export function xfadeFilter(durations: number[], transition: number): string {
  const offsets = xfadeOffsets(durations, transition);
  if (offsets.length === 0) {
    return "";
  }
  const stages: string[] = [];
  let previous = "[0:v]";
  for (let index = 0; index < offsets.length; index++) {
    const label = `[v${index + 1}]`;
    stages.push(
      `${previous}[${index + 1}:v]xfade=transition=fade:duration=${formatSeconds(transition)}:offset=${formatSeconds(offsets[index] ?? 0)}${label}`,
    );
    previous = label;
  }
  return stages.join(";");
}

/** The scene a poster is taken from: the first card, or the first scene when none. */
export function posterScene(config: DemoConfig): Scene {
  const card = config.scenes.find((scene) => scene.kind === "card");
  if (card !== undefined) {
    return card;
  }
  const first = config.scenes[0];
  if (first === undefined) {
    throw new Error("a demo needs at least one scene");
  }
  return first;
}

/** Options for {@link buildNormalizeArgs}. */
export interface NormalizeOptions {
  width: number;
  height: number;
  fps: number;
  /** Seconds to keep; the rest is trimmed. */
  duration: number;
  /** True when the clip is already on the common canvas and only needs re-encoding. */
  canvasReady: boolean;
  /** Text file whose contents are drawn in the caption strip. */
  captionTextFile?: string;
}

function captionDrawtext(textFile: string, height: number): string {
  const fontSize = 26;
  const y = height - CAPTION_STRIP + Math.round((CAPTION_STRIP - fontSize) / 2);
  return `drawtext=fontfile=${CAPTION_FONT}:textfile=${textFile}:fontsize=${fontSize}:fontcolor=white:x=(w-text_w)/2:y=${y}`;
}

/**
 * Builds the ffmpeg arguments that bring a clip onto the common canvas: the frame rate
 * and pixel format are fixed, a raw clip is scaled and padded into the content area,
 * and the clip is trimmed to its declared duration. Clips that are already on the canvas
 * are only re-encoded and trimmed, so they are not scaled a second time.
 */
export function buildNormalizeArgs(input: string, output: string, options: NormalizeOptions): string[] {
  const content = contentHeight(options.height);
  const filters = [`fps=${options.fps}`];
  if (!options.canvasReady) {
    filters.push(`scale=${options.width}:${content}:force_original_aspect_ratio=decrease`);
    filters.push(`pad=${options.width}:${content}:(ow-iw)/2:(oh-ih)/2:color=black`);
    filters.push(`pad=${options.width}:${options.height}:0:0:color=${BG_COLOR}`);
  }
  filters.push("setsar=1");
  if (options.captionTextFile !== undefined) {
    filters.push(captionDrawtext(options.captionTextFile, options.height));
  }
  return [
    "-v", "error", "-y",
    "-i", input,
    "-vf", filters.join(","),
    "-t", formatSeconds(options.duration),
    "-pix_fmt", "yuv420p",
    "-an",
    "-c:v", "libx264", "-crf", "20",
    output,
  ];
}

/** Builds the arguments that crossfade and concatenate the normalized clips. */
export function buildComposeArgs(inputs: string[], out: string, durations: number[], transition: number): string[] {
  const first = inputs[0];
  if (first === undefined) {
    throw new Error("cannot compose an empty clip list");
  }
  if (inputs.length === 1) {
    return ["-v", "error", "-y", "-i", first, "-pix_fmt", "yuv420p", "-an", "-c:v", "libx264", "-crf", "20", out];
  }
  const args = ["-v", "error", "-y"];
  for (const input of inputs) {
    args.push("-i", input);
  }
  const outputLabel = `[v${inputs.length - 1}]`;
  args.push("-filter_complex", xfadeFilter(durations, transition), "-map", outputLabel, "-pix_fmt", "yuv420p", "-an", "-c:v", "libx264", "-crf", "20", out);
  return args;
}

/** Builds the arguments that extract the first frame of a clip as a PNG poster. */
export function buildPosterArgs(input: string, out: string): string[] {
  return ["-v", "error", "-y", "-i", input, "-frames:v", "1", out];
}

/** Builds the palette-based GIF arguments for the given clips. */
export function buildGifArgs(inputs: string[], out: string, fps: number): string[] {
  const args = ["-v", "error", "-y"];
  for (const input of inputs) {
    args.push("-i", input);
  }
  const count = inputs.length;
  const concatInputs = inputs.map((_, index) => `[${index}:v]`).join("");
  const filter =
    `${concatInputs}concat=n=${count}:v=1:a=0[joined];` +
    `[joined]fps=${fps},scale=1100:-1:flags=lanczos,split[s0][s1];` +
    `[s0]palettegen[p];[s1][p]paletteuse`;
  args.push("-filter_complex", filter, "-loop", "0", out);
  return args;
}

/** Injectable dependencies for {@link compose}. */
export interface ComposeDeps {
  terminal?: VhsRecorder;
  emulator?: EmulatorDeps;
}

/** Options for {@link compose}. */
export interface ComposeOptions {
  /** Output override; replaces the config's `out`. */
  out?: string;
  /** When set, only these scene ids are rendered; gif and poster are skipped. */
  only?: string[];
  deps?: ComposeDeps;
}

async function renderScene(scene: Scene, env: RenderEnv, deps: ComposeDeps): Promise<string> {
  switch (scene.kind) {
    case "card":
    case "outro":
      return renderCard(scene, env);
    case "code":
      return renderCode(scene, env);
    case "terminal":
      return renderTerminal(scene, env, deps.terminal);
    case "emulator":
      return renderEmulator(scene, env, deps.emulator);
  }
}

/** True when a renderer already produced a clip on the common canvas. */
function isCanvasReady(scene: Scene): boolean {
  if (scene.kind === "terminal") {
    return false;
  }
  if (scene.kind === "emulator") {
    return scene.frame !== false;
  }
  return true;
}

/**
 * Renders every scene, brings each clip onto the common canvas, crossfades them, and
 * writes the output. When the config declares a gif or a poster, those are written too.
 *
 * @returns the resolved output path.
 * @throws {Error} when the caption font is missing or the composed length is wrong.
 */
export async function compose(config: DemoConfig, env: RenderEnv, options: ComposeOptions = {}): Promise<string> {
  const deps = options.deps ?? {};
  const transition = config.transition ?? 0.5;
  const out = options.out === undefined ? join(env.root, config.out) : resolveRepoPath(env.root, options.out, "out");

  if (options.only !== undefined) {
    const unknown = options.only.filter((id) => !config.scenes.some((scene) => scene.id === id));
    if (unknown.length > 0) {
      throw new ConfigError(`unknown scene id(s): ${unknown.join(", ")}`);
    }
  }
  const selected =
    options.only === undefined ? config.scenes : config.scenes.filter((scene) => options.only?.includes(scene.id));

  if (
    selected.some((scene) => scene.kind === "terminal" && scene.caption !== undefined) &&
    !existsSync(CAPTION_FONT)
  ) {
    throw new Error(`the caption font is missing: ${CAPTION_FONT}`);
  }

  const normalizedDir = join(env.workDir, "normalized");
  const textDir = join(env.workDir, "text");
  await mkdir(normalizedDir, { recursive: true });
  await mkdir(textDir, { recursive: true });
  await mkdir(dirname(out), { recursive: true });

  const clips: string[] = [];
  const durations: number[] = [];
  for (const scene of selected) {
    const raw = await renderScene(scene, env, deps);
    const normalized = join(normalizedDir, `${scene.id}.mp4`);
    let captionTextFile: string | undefined;
    if (scene.kind === "terminal" && scene.caption !== undefined) {
      captionTextFile = join(textDir, `${scene.id}.txt`);
      await writeFile(captionTextFile, scene.caption, "utf8");
    }
    await runOrThrow(
      "ffmpeg",
      buildNormalizeArgs(raw, normalized, {
        width: env.width,
        height: env.height,
        fps: env.fps,
        duration: sceneDuration(scene),
        canvasReady: isCanvasReady(scene),
        captionTextFile,
      }),
      { stdio: "ignore" },
    );
    clips.push(normalized);
    // Trim only shortens a clip, and VHS can deliver less than the requested hold, so
    // read the real length: the xfade offsets and the length check must use it, or an
    // offset can run past the accumulated clip and truncate the whole chain.
    durations.push(await ffprobeDuration(normalized));
  }

  if (clips.length > 1) {
    const shortIndex = durations.findIndex((duration) => duration <= transition);
    if (shortIndex !== -1) {
      throw new Error(
        `scene "${selected[shortIndex]?.id}" is ${durations[shortIndex]}s, not longer than the ` +
          `${transition}s crossfade; shorten the transition or lengthen the scene`,
      );
    }
  }

  await runOrThrow("ffmpeg", buildComposeArgs(clips, out, durations, transition), { stdio: "ignore" });

  const expected = expectedTotal(durations, transition);
  const actual = await ffprobeDuration(out);
  if (Math.abs(actual - expected) > DURATION_TOLERANCE) {
    throw new Error(`the composed length is ${actual.toFixed(2)}s but the scenes expect ${expected.toFixed(2)}s`);
  }

  if (options.only === undefined && config.gif !== undefined) {
    const gifInputs = config.gif.sceneIds.map((id) => join(normalizedDir, `${id}.mp4`));
    await runOrThrow("ffmpeg", buildGifArgs(gifInputs, join(env.root, config.gif.out), env.fps), { stdio: "ignore" });
  }

  if (options.only === undefined && config.poster !== undefined) {
    const scene = posterScene(config);
    const posterPath = join(env.root, config.poster);
    await mkdir(dirname(posterPath), { recursive: true });
    if (scene.kind === "card" || scene.kind === "outro") {
      await renderPoster(scene, env, posterPath);
    } else {
      await runOrThrow("ffmpeg", buildPosterArgs(join(normalizedDir, `${scene.id}.mp4`), posterPath), { stdio: "ignore" });
    }
  }

  return out;
}
