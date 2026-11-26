/**
 * Edge cases of the extension's host path helpers: drive-letter and separator
 * spellings, the `.exe` suffix, and the installed-CLI fallbacks when
 * LOCALAPPDATA is missing or unusable.
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
  comparablePath(value: string, platform?: NodeJS.Platform): string;
  samePath(a: string, b: string, platform?: NodeJS.Platform): boolean;
  executableName(name: string, platform?: NodeJS.Platform): string;
  installedToolPath(name: string, platform: NodeJS.Platform, env: NodeJS.ProcessEnv, home: string): string;
};

test("should treat forward slashes and redundant segments as the same Windows file", () => {
  assert.equal(paths.samePath("C:/a/./b/../c.clo", "c:\\A\\C.clo", "win32"), true);
  assert.equal(paths.comparablePath("C:/A/b.clo", "win32"), "c:\\a\\b.clo");
});

test("should keep different Windows files distinct", () => {
  assert.equal(paths.samePath("C:\\a\\b.clo", "D:\\a\\b.clo", "win32"), false);
  assert.equal(paths.samePath("C:\\a\\b.clo", "C:\\a\\b.clo.bak", "win32"), false);
});

test("should compare exact bytes on Linux and macOS", () => {
  assert.equal(paths.samePath("/a/./b", "/a/b", "linux"), false);
  assert.equal(paths.samePath("/A", "/a", "darwin"), false);
  assert.equal(paths.comparablePath("/A/b", "linux"), "/A/b");
});

test("should add the exe suffix only on Windows and only once", () => {
  assert.equal(paths.executableName("clorraine-lsp", "win32"), "clorraine-lsp.exe");
  assert.equal(paths.executableName("clorraine.EXE", "win32"), "clorraine.EXE");
  assert.equal(paths.executableName("clorraine", "linux"), "clorraine");
});

test("should fall back to the profile's AppData when LOCALAPPDATA is missing or relative", () => {
  const expected = "C:\\Users\\u\\AppData\\Local\\Programs\\Clorraine\\current\\bin\\clorraine-lsp.exe";
  assert.equal(paths.installedToolPath("clorraine-lsp", "win32", {}, "C:\\Users\\u"), expected);
  assert.equal(paths.installedToolPath("clorraine-lsp", "win32", { LOCALAPPDATA: "rel" }, "C:\\Users\\u"), expected);
});

test("should ignore LOCALAPPDATA on Linux", () => {
  assert.equal(
    paths.installedToolPath("clorraine-lsp", "linux", { LOCALAPPDATA: "C:\\x" }, "/home/u"),
    "/home/u/.local/bin/clorraine-lsp",
  );
});
