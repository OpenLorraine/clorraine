import { copyFile } from "node:fs/promises";
import type { CardScene, OutroScene } from "../scenes.ts";
import { cardQuery } from "./card.ts";
import type { RenderEnv } from "./env.ts";

/**
 * Writes a single-frame PNG poster from a card scene (the first frame of `card.html`).
 *
 * @param out - Destination PNG path.
 */
export async function renderPoster(scene: CardScene | OutroScene, env: RenderEnv, out: string): Promise<void> {
  const frames = await env.frames.capture({
    id: `${scene.id}-poster`,
    page: "card.html",
    query: cardQuery(scene),
    width: env.width,
    height: env.height,
    count: 1,
    fps: env.fps,
  });
  const first = frames[0];
  if (first === undefined) {
    throw new Error("poster capture produced no frame");
  }
  await copyFile(first, out);
}
