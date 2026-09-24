import { stat } from "node:fs/promises";
import type { TerminalScene } from "../scenes.ts";
import { contentHeight } from "../renderer/env.ts";
import { run } from "./proc.ts";

/** Options for {@link buildTape}. */
export interface TapeOptions {
  /** Raw clip path the recording is written to. */
  output: string;
  /** Canvas width in pixels. */
  width: number;
  /** Canvas height in pixels; the caption strip is subtracted for the terminal. */
  height: number;
  /** Seconds each command stays on screen after it is typed. */
  duration: number;
}

/**
 * Escapes a value for a double-quoted VHS argument. Newlines are collapsed because a
 * tape is line-oriented and a raw newline would start a new directive. The values are
 * developer-authored (a scene script is trusted code), so this keeps the tape
 * well-formed; it is not a security boundary.
 */
export function escapeTapeValue(value: string): string {
  return value.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\r\n?|\n/g, " ");
}

/**
 * Builds a VHS tape that types each command, presses Enter, and holds the result for
 * the scene's duration.
 *
 * @example
 * ```ts
 * buildTape({ id: "run", kind: "terminal", commands: ["ls"] }, options);
 * ```
 */
export function buildTape(scene: TerminalScene, options: TapeOptions): string {
  const lines = [
    `Output ${options.output}`,
    'Set Shell "bash"',
    "Set FontSize 16",
    `Set Width ${options.width}`,
    `Set Height ${contentHeight(options.height)}`,
    "Set Padding 20",
    'Set Theme "Dracula"',
    "Set TypingSpeed 80ms",
    "",
  ];
  for (const command of scene.commands) {
    lines.push(`Type "${escapeTapeValue(command)}"`, "Enter", `Sleep ${options.duration}s`, "");
  }
  return lines.join("\n");
}

/** Records a VHS tape to a clip. */
export interface VhsRecorder {
  record(tapePath: string, output: string): Promise<void>;
}

/**
 * Builds the environment VHS needs. VHS is commonly installed by `go install` into
 * `~/go/bin` or `~/.local/bin`, which are not on the default PATH here.
 */
function vhsEnv(): NodeJS.ProcessEnv {
  const home = process.env.HOME ?? "";
  const extra = [`${home}/go/bin`, `${home}/.local/bin`].join(":");
  return { ...process.env, PATH: `${extra}:${process.env.PATH ?? ""}` };
}

/**
 * Throws when VHS did not produce a non-empty clip. VHS v0.12.0 is known to write
 * nothing, so the message points at the pinned version.
 */
export async function ensureVhsOutput(output: string): Promise<void> {
  let size = 0;
  try {
    size = (await stat(output)).size;
  } catch {
    size = 0;
  }
  if (size === 0) {
    throw new Error(`VHS did not write ${output}; VHS v0.12.0 is broken, use v0.11.0`);
  }
}

/** Runs the real `vhs` binary and checks that it wrote the clip. */
export class VhsRunner implements VhsRecorder {
  async record(tapePath: string, output: string): Promise<void> {
    const result = await run("vhs", [tapePath], { stdio: "inherit", env: vhsEnv() });
    if (result.code !== 0) {
      throw new Error(`vhs exited with code ${result.code}`);
    }
    await ensureVhsOutput(output);
  }
}
