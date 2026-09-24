import { mkdir, readFile, writeFile } from "node:fs/promises";
import { basename, dirname, extname, join, relative } from "node:path";
import { ConfigError } from "./scenes.ts";
import { resolveRepoPath } from "./util/paths.ts";

/** Options for {@link writeScaffold}. */
export interface ScaffoldOptions {
  /** Repository root that the source path and the output are resolved against. */
  root: string;
  /** Source `.clo` file, relative to `root`. */
  file: string;
  /** Overwrite an existing scene script. */
  force?: boolean;
}

/** The scaffold that {@link writeScaffold} wrote. */
export interface ScaffoldResult {
  /** Absolute path of the written scene script. */
  path: string;
  /** The scene script path relative to the repository root. */
  relativePath: string;
  /** The intro title derived from the source. */
  title: string;
}

/**
 * Derives the intro card title from a source file: the first leading `//` comment line,
 * stripped of its `//` marker and a trailing period. Falls back to the file stem when the
 * file has no leading comment (or the comment is empty).
 */
export function deriveTitle(contents: string, stem: string): string {
  const firstLine = contents.split("\n", 1)[0] ?? "";
  const comment = /^\s*\/\/\s?(.*)$/.exec(firstLine)?.[1]?.trim() ?? "";
  const title = comment.replace(/\.$/, "");
  return title.length > 0 ? title : stem;
}

/** The scene-script text for a source file, given its title. */
function scaffoldSource(relativeFile: string, stem: string, title: string): string {
  const out = `demo/out/${stem}`;
  const run = `make -s run FILE=${relativeFile} PROFILE=hosted-amiga`;
  return `import { defineDemo } from "../tools/demo/src/index.ts";

export default defineDemo({
  out: ${JSON.stringify(`${out}.mp4`)},
  poster: ${JSON.stringify(`${out}.png`)},
  scenes: [
    { id: "intro", kind: "card", title: ${JSON.stringify(title)}, duration: 6 },
    { id: "src", kind: "code", file: ${JSON.stringify(relativeFile)}, duration: 12 },
    { id: "run", kind: "terminal", commands: [${JSON.stringify(run)}], duration: 14 },
    { id: "emu", kind: "emulator", file: ${JSON.stringify(relativeFile)}, frame: true },
    { id: "outro", kind: "outro", title: "Clorraine", lines: ["github.com/OpenLorraine"], duration: 6 },
  ],
});
`;
}

/**
 * Writes a scene-script scaffold for a Clorraine source file to `demo/<stem>.demo.ts`.
 *
 * The scaffold is a starting point only; it is not rendered. An existing script is left
 * untouched unless `force` is set.
 *
 * @throws {ConfigError} when the source file is missing or unreadable, or the target
 *   already exists without `force`.
 */
export async function writeScaffold(options: ScaffoldOptions): Promise<ScaffoldResult> {
  const sourcePath = resolveRepoPath(options.root, options.file, "source");
  let contents: string;
  try {
    contents = await readFile(sourcePath, "utf8");
  } catch {
    throw new ConfigError(`source file not found or unreadable: ${options.file}`);
  }

  const relativeFile = relative(options.root, sourcePath).split("\\").join("/");
  const stem = basename(sourcePath, extname(sourcePath));
  const title = deriveTitle(contents, stem);
  const target = resolveRepoPath(options.root, join("demo", `${stem}.demo.ts`), "scaffold");
  const relativePath = relative(options.root, target).split("\\").join("/");

  await mkdir(dirname(target), { recursive: true });
  try {
    await writeFile(target, scaffoldSource(relativeFile, stem, title), {
      encoding: "utf8",
      flag: options.force === true ? "w" : "wx",
    });
  } catch (thrown) {
    const code = thrown instanceof Error && "code" in thrown ? thrown.code : undefined;
    if (!options.force && code === "EEXIST") {
      throw new ConfigError(`${relativePath} already exists; pass --force to overwrite`);
    }
    throw thrown;
  }

  return { path: target, relativePath, title };
}
