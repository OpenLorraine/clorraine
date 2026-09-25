import { copyFile, mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import type { FrameSource, FrameSpec } from "./renderer/env.ts";

/**
 * One channel branding image: the page that draws it, the page's canvas size,
 * and the file name it is written as. The sizes follow YouTube's published
 * requirements (banner 2560x1440, avatar 800x800, thumbnail 1280x720).
 */
export interface ChannelAsset {
  /** Scene id; frames are written under a directory named for it. */
  id: string;
  /** Page path, relative to the assets directory. */
  page: string;
  /** Canvas width in pixels. */
  width: number;
  /** Canvas height in pixels. */
  height: number;
  /** Output file name. */
  file: string;
  /** Query parameters passed to the page. */
  query?: Record<string, string>;
}

/** The channel branding images, in render order. */
export const CHANNEL_ASSETS: readonly ChannelAsset[] = [
  { id: "banner", page: "channel/banner.html", width: 2560, height: 1440, file: "banner.png" },
  { id: "avatar", page: "channel/avatar.html", width: 800, height: 800, file: "avatar.png" },
  {
    id: "thumbnail",
    page: "channel/thumbnail.html",
    width: 1280,
    height: 720,
    file: "thumbnail.png",
    query: {
      kick: "LorraineOS",
      title: "Clorraine",
      subtitle: "A safe, C-like language for the Amiga",
    },
  },
];

/**
 * Renders every {@link CHANNEL_ASSETS} image and copies it into `outDir`.
 *
 * @returns the absolute paths of the written PNGs, in render order.
 * @throws {Error} when a page produces no image.
 */
export async function renderChannelAssets(frames: FrameSource, outDir: string): Promise<string[]> {
  await mkdir(outDir, { recursive: true });
  const written: string[] = [];
  for (const asset of CHANNEL_ASSETS) {
    const spec: FrameSpec = {
      id: asset.id,
      page: asset.page,
      width: asset.width,
      height: asset.height,
      scale: 1,
      count: 1,
      fps: 1,
      query: asset.query,
    };
    const [frame] = await frames.capture(spec);
    if (frame === undefined) {
      throw new Error(`${asset.page} produced no image`);
    }
    const target = join(outDir, asset.file);
    await mkdir(dirname(target), { recursive: true });
    await copyFile(frame, target);
    written.push(target);
  }
  return written;
}
