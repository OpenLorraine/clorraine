/**
 * Windows path rules of the extension: the language server and the editor
 * spell one Windows file in different ways (drive-letter case, separators),
 * so saved-source checks must compare them as the same file, and the CLI the
 * Windows installer puts on disk must be found without a terminal PATH. The
 * helpers are pure, so these cases run on any host.
 */
import { strict as assert } from "node:assert";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const load = createRequire(import.meta.url);

/** The staged path helpers, as the packaged extension ships them. */
const paths = load(resolve(here, "../../../build/vscode/extension/out/paths.js")) as {
  samePath(a: string, b: string, platform?: NodeJS.Platform): boolean;
  installedToolPath(name: string, platform: NodeJS.Platform, env: NodeJS.ProcessEnv, home: string): string;
};

test("should treat Windows paths that differ only in case as the same file", () => {
  assert.equal(paths.samePath("c:\\A\\b.clo", "C:\\a\\B.clo", "win32"), true);
});

test("should keep paths that differ only in case distinct on Linux", () => {
  assert.equal(paths.samePath("c:\\A\\b.clo", "C:\\a\\B.clo", "linux"), false);
});

test("should find the installed Windows CLI under LOCALAPPDATA", () => {
  assert.equal(
    paths.installedToolPath("clorraine", "win32", { LOCALAPPDATA: "C:\\Users\\u\\AppData\\Local" }, "C:\\Users\\u"),
    "C:\\Users\\u\\AppData\\Local\\Programs\\Clorraine\\current\\bin\\clorraine.exe",
  );
});

test("should keep the Linux CLI location in ~/.local/bin", () => {
  assert.equal(paths.installedToolPath("clorraine", "linux", {}, "/home/u"), "/home/u/.local/bin/clorraine");
});
