#!/usr/bin/env node
import { realpathSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { pathToFileURL } from "node:url";
import { loadConfig } from "./config.ts";
import { compose } from "./renderer/compose.ts";
import type { RenderEnv } from "./renderer/env.ts";
import { ConfigError, sceneDuration } from "./scenes.ts";
import type { DemoConfig } from "./scenes.ts";
import { ChromeFrameSource } from "./util/chrome.ts";
import { FfmpegEncoder } from "./util/ffmpeg.ts";
import { defaultRepoRoot, PathError } from "./util/paths.ts";
import { error, info, messageOf } from "./util/log.ts";

const USAGE = `Usage: clorraine-demo <command> [options]

Commands:
  render <config>   Render a demo config (a .ts or .json scene script)
  new <file.clo>    Write a scene-script scaffold for a Clorraine source file
  list <config>     Print the scenes in a config

Options:
  --only <ids>      With render: render only these scenes (comma-separated or repeatable)
  --out <path>      With render: override the config's output path
  --root <path>     Repository root for path resolution (also CLORRAINE_DEMO_ROOT)
  --force           With new: overwrite an existing scaffold
  -h, --help        Show this help
`;

/** Parses arguments and runs the requested command. Returns the process exit code. */
export async function main(argv: string[]): Promise<number> {
  let values: {
    only?: string[];
    out?: string;
    root?: string;
    force?: boolean;
    help?: boolean;
  };
  let positionals: string[];
  try {
    ({ values, positionals } = parseArgs({
      args: argv,
      allowPositionals: true,
      options: {
        only: { type: "string", multiple: true },
        out: { type: "string" },
        root: { type: "string" },
        force: { type: "boolean" },
        help: { type: "boolean", short: "h" },
      },
    }));
  } catch (thrown) {
    error(messageOf(thrown));
    info(USAGE);
    return 2;
  }

  const command = positionals[0];
  if (values.help === true || command === undefined) {
    info(USAGE);
    return command === undefined && values.help !== true ? 2 : 0;
  }

  try {
    switch (command) {
      case "list": {
        const file = requireConfig(positionals[1]);
        const config = await loadConfig(file, { root: values.root });
        for (const scene of config.scenes) {
          const caption = scene.caption === undefined ? "" : `  # ${scene.caption}`;
          info(`${scene.id}\t${scene.kind}\t${sceneDuration(scene)}s${caption}`);
        }
        return 0;
      }
      case "render": {
        const file = requireConfig(positionals[1]);
        const root = values.root ?? process.env.CLORRAINE_DEMO_ROOT ?? defaultRepoRoot();
        const config = await loadConfig(file, { root });
        const out = await compose(config, buildRenderEnv(config, root), {
          out: values.out,
          only: parseOnly(values.only),
        });
        info(`wrote ${out}`);
        return 0;
      }
      case "new":
        error(`"${command}" is not implemented yet`);
        return 1;
      default:
        error(`unknown command "${command}"`);
        info(USAGE);
        return 2;
    }
  } catch (thrown) {
    error(messageOf(thrown));
    return thrown instanceof ConfigError || thrown instanceof PathError ? 2 : 1;
  }
}

function requireConfig(value: string | undefined): string {
  if (value === undefined) {
    throw new ConfigError("a config path is required");
  }
  return value;
}

/** Splits `--only a,b` (repeatable) into scene ids, rejecting an empty selection. */
function parseOnly(values: string[] | undefined): string[] | undefined {
  if (values === undefined) {
    return undefined;
  }
  const ids = values
    .flatMap((value) => value.split(","))
    .map((id) => id.trim())
    .filter((id) => id.length > 0);
  if (ids.length === 0) {
    throw new ConfigError("--only needs at least one scene id");
  }
  return ids;
}

/**
 * Builds the real renderer environment: headless Chrome for HTML frames, ffmpeg for
 * encoding, and the repository's `build/demo/` tree for intermediates.
 *
 * `rawRoot` is normalized to an absolute path so that a relative `--root` does not get
 * combined twice when a child process is started with `cwd: root`.
 */
function buildRenderEnv(config: DemoConfig, rawRoot: string): RenderEnv {
  const root = resolve(rawRoot);
  const base = join(root, "build", "demo");
  return {
    frames: new ChromeFrameSource({
      assetsDir: join(root, "tools", "demo", "assets"),
      framesDir: join(base, "frames"),
    }),
    encode: new FfmpegEncoder(),
    root,
    width: config.width ?? 1100,
    height: config.height ?? 700,
    fps: config.fps ?? 50,
    outDir: join(base, "raw"),
    workDir: join(base, "work"),
  };
}

async function runAsEntryPoint(): Promise<void> {
  const entry = process.argv[1];
  if (entry === undefined) {
    return;
  }
  let entryUrl: string;
  try {
    entryUrl = pathToFileURL(realpathSync(entry)).href;
  } catch {
    // `entry` is not a resolvable path; this module was imported, not run.
    return;
  }
  if (import.meta.url === entryUrl) {
    process.exitCode = await main(process.argv.slice(2));
  }
}

await runAsEntryPoint();
