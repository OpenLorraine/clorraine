import { spawn } from "node:child_process";

/** The captured result of a finished process. */
export interface RunResult {
  code: number;
  stdout: string;
  stderr: string;
}

/** Options for {@link run}. */
export interface RunOptions {
  /** Working directory for the child process. */
  cwd?: string;
  /** Level of stdio when the output is not captured: "inherit" (default) or "ignore". */
  stdio?: "inherit" | "ignore";
}

/**
 * Runs a command with an explicit argument array (never a shell string), so values such as
 * file paths or commands cannot be interpreted by a shell.
 *
 * Child output is always captured. When `stdio` is "inherit" it is also echoed live, which
 * keeps long renders visible while still returning the text to the caller.
 *
 * @returns the exit code and captured stdout/stderr.
 * @throws {Error} when the process cannot be spawned.
 */
export function run(command: string, args: string[], options: RunOptions = {}): Promise<RunResult> {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    const echo = options.stdio !== "ignore";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      stdout += chunk;
      if (echo) {
        process.stderr.write(chunk);
      }
    });
    child.stderr.on("data", (chunk: string) => {
      stderr += chunk;
      if (echo) {
        process.stderr.write(chunk);
      }
    });
    child.on("error", reject);
    child.on("close", (code) => resolvePromise({ code: code ?? 0, stdout, stderr }));
  });
}

/** Runs a command and throws when it exits non-zero. */
export async function runOrThrow(command: string, args: string[], options: RunOptions = {}): Promise<RunResult> {
  const result = await run(command, args, options);
  if (result.code !== 0) {
    throw new Error(`${command} exited with code ${result.code}`);
  }
  return result;
}

/** True when the command can be found on PATH. */
export function commandExists(command: string): Promise<boolean> {
  return run("which", [command], { stdio: "ignore" }).then((result) => result.code === 0);
}
