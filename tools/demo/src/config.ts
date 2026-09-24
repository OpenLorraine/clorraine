import { access, readFile } from "node:fs/promises";
import { constants } from "node:fs";
import { extname, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { validateConfig } from "./scenes.ts";
import type { DemoConfig } from "./scenes.ts";
import { defaultRepoRoot, PathError, resolveRepoPath } from "./util/paths.ts";

/** Options for {@link loadConfig}. */
export interface LoadOptions {
  /** Repository root for path resolution. Defaults to the real repository root. */
  root?: string;
}

async function readRaw(file: string): Promise<unknown> {
  if (extname(file) === ".json") {
    return JSON.parse(await readFile(file, "utf8"));
  }
  const module = (await import(pathToFileURL(resolve(file)).href)) as { default?: unknown };
  if (module.default === undefined) {
    throw new PathError(`${file}: a .ts config must have a default export`);
  }
  return module.default;
}

async function assertFileExists(root: string, candidate: string, label: string): Promise<void> {
  const resolved = resolveRepoPath(root, candidate, label);
  try {
    await access(resolved, constants.R_OK);
  } catch {
    throw new PathError(`${label}: file not found or unreadable (${candidate})`);
  }
}

/**
 * Loads a `.ts` or `.json` demo config, validates its structure, and checks that every
 * referenced path is inside the repository root and that input files exist.
 *
 * @param file - Path to the config file (absolute or relative to the process cwd).
 * @param options - Optional repository root override (used by tests).
 * @throws {ConfigError} when the structure is invalid.
 * @throws {PathError} when a path is absolute, escapes the root, or a required input is missing.
 */
export async function loadConfig(file: string, options: LoadOptions = {}): Promise<DemoConfig> {
  const root = options.root ?? defaultRepoRoot();
  const raw = await readRaw(file);
  validateConfig(raw);
  const config = raw as DemoConfig;

  resolveRepoPath(root, config.out, "out");
  if (config.poster !== undefined) {
    resolveRepoPath(root, config.poster, "poster");
  }
  if (config.gif !== undefined) {
    resolveRepoPath(root, config.gif.out, "gif.out");
  }
  for (const scene of config.scenes) {
    const label = `scene "${scene.id}"`;
    if (scene.kind === "code" || scene.kind === "emulator") {
      await assertFileExists(root, scene.file, label);
    }
  }
  return config;
}
