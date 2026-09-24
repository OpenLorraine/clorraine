import { readFile, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { sceneDuration } from "../scenes.ts";
import type { CodeScene } from "../scenes.ts";
import { tokenizeLines } from "../util/shiki.ts";
import type { RenderEnv } from "./env.ts";

const toolDir = join(fileURLToPath(import.meta.url), "..", "..", "..");
const assetsDir = join(toolDir, "assets");

function escapeHtml(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/** Builds the line-numbered HTML for the highlighted source. */
async function buildLineHtml(source: string): Promise<string> {
  const lines = await tokenizeLines(source);
  return lines
    .map((tokens, index) => {
      const text = tokens
        .map((token) => `<span style="color:${token.color}">${escapeHtml(token.content)}</span>`)
        .join("");
      return `<div class="line" data-line="${index + 1}"><span class="num">${index + 1}</span><span class="text">${text}</span></div>`;
    })
    .join("\n");
}

/**
 * Renders a code scene to an MP4 clip: the source is highlighted, wrapped in a
 * line-numbered page, and captured with optional scrolling and line highlighting.
 *
 * @returns the clip path.
 * @throws {Error} when the source file cannot be read.
 */
export async function renderCode(scene: CodeScene, env: RenderEnv): Promise<string> {
  const source = await readFile(resolve(env.root, scene.file), "utf8");
  const lineHtml = await buildLineHtml(source);
  const css = await readFile(join(assetsDir, "demo.css"), "utf8");
  const template = await readFile(join(assetsDir, "code.html"), "utf8");

  const duration = sceneDuration(scene);
  const page = template
    .replace("{{CSS}}", () => css)
    .replace("{{FILENAME}}", () => escapeHtml(scene.file))
    .replace("{{LINES}}", () => lineHtml);

  const pagePath = join(env.workDir, `${scene.id}.html`);
  await writeFile(pagePath, page, "utf8");

  const count = Math.round(duration * env.fps);
  const frames = await env.frames.capture({
    id: scene.id,
    page: pagePath,
    query: {
      duration: String(duration),
      scroll: String(scene.scroll ?? false),
      highlight: JSON.stringify(scene.highlight ?? []),
    },
    width: env.width,
    height: env.height,
    count,
    fps: env.fps,
  });
  const out = join(env.outDir, `${scene.id}.mp4`);
  await env.encode.encodeFrames(frames, out, { fps: env.fps });
  return out;
}
