import { join } from "node:path";
import { sceneDuration } from "../scenes.ts";
import type { CardScene, OutroScene } from "../scenes.ts";
import type { RenderEnv } from "./env.ts";

/** The query parameters a card page reads. */
export function cardQuery(scene: CardScene | OutroScene): Record<string, string> {
  const lines = "lines" in scene ? scene.lines ?? [] : [];
  const subtitle = "subtitle" in scene ? scene.subtitle ?? "" : "";
  return { title: scene.title, subtitle, caption: scene.caption ?? "", lines: JSON.stringify(lines) };
}

/**
 * Renders a card (or outro) scene to an MP4 clip by capturing one frame per output frame
 * from `card.html`.
 *
 * @returns the clip path.
 */
export async function renderCard(scene: CardScene | OutroScene, env: RenderEnv): Promise<string> {
  const count = Math.round(sceneDuration(scene) * env.fps);
  const frames = await env.frames.capture({
    id: scene.id,
    page: "card.html",
    query: cardQuery(scene),
    width: env.width,
    height: env.height,
    scale: env.scale,
    count,
    fps: env.fps,
  });
  const out = join(env.outDir, `${scene.id}.mp4`);
  await env.encode.encodeFrames(frames, out, { fps: env.fps });
  return out;
}
