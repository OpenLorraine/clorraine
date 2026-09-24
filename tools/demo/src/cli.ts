#!/usr/bin/env node
import { realpathSync } from "node:fs";
import { parseArgs } from "node:util";
import { pathToFileURL } from "node:url";
import { loadConfig } from "./config.ts";
import { ConfigError, sceneDuration } from "./scenes.ts";
import { PathError } from "./util/paths.ts";
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
      case "render":
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
