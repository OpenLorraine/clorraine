import { existsSync, realpathSync } from "node:fs";
import { dirname, isAbsolute, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

/** Thrown when a path is unsafe: absolute, or outside the repository root. */
export class PathError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "PathError";
  }
}

/**
 * The repository root inferred from this module's location
 * (`tools/demo/src/util/` is four levels below the root).
 */
export function defaultRepoRoot(): string {
  return resolve(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "..");
}

/**
 * Resolves a repository-relative path and rejects it if it escapes the root.
 *
 * Absolute paths are rejected outright. For a path that does not exist yet (an output
 * file), the nearest existing ancestor is canonicalized so a symlink cannot smuggle the
 * target outside the root.
 *
 * @throws {PathError} when the path is absolute or escapes the root.
 */
export function resolveRepoPath(root: string, candidate: string, label: string): string {
  if (isAbsolute(candidate)) {
    throw new PathError(`${label}: absolute paths are not allowed (${candidate})`);
  }
  const resolved = resolve(root, candidate);
  const realRoot = realpathSync(root);
  let probe = resolved;
  while (!existsSync(probe)) {
    const parent = dirname(probe);
    if (parent === probe) {
      break;
    }
    probe = parent;
  }
  const realProbe = realpathSync(probe);
  if (realProbe !== realRoot && !realProbe.startsWith(realRoot + sep)) {
    throw new PathError(`${label}: path escapes the repository root (${candidate})`);
  }
  return resolved;
}
