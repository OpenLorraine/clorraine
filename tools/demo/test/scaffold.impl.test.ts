import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { deriveTitle, writeScaffold } from "../src/scaffold.ts";
import { ConfigError } from "../src/scenes.ts";
import { PathError } from "../src/util/paths.ts";

async function makeRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), "clorraine-scaffold-impl-"));
}

test("deriveTitle strips the marker and a trailing period", () => {
  assert.equal(deriveTitle("// The smallest Clorraine program: print one line.\nfn main() {}\n", "hello"), "The smallest Clorraine program: print one line");
  assert.equal(deriveTitle("\t//   Indented comment\n", "stem"), "Indented comment");
  assert.equal(deriveTitle("// No period here\n", "stem"), "No period here");
});

test("deriveTitle falls back to the stem for a missing or empty comment", () => {
  assert.equal(deriveTitle("fn main() {}\n", "hello"), "hello");
  assert.equal(deriveTitle("//\n", "hello"), "hello");
  assert.equal(deriveTitle("//   \n", "hello"), "hello");
  assert.equal(deriveTitle("", "hello"), "hello");
});

test("writeScaffold writes demo/<stem>.demo.ts with the derived title", async () => {
  const root = await makeRoot();
  await mkdir(join(root, "examples"), { recursive: true });
  await writeFile(join(root, "examples/greet.clo"), "// Say hello.\nfn main() {}\n", "utf8");

  const result = await writeScaffold({ root, file: "examples/greet.clo" });

  assert.equal(result.relativePath, "demo/greet.demo.ts");
  assert.equal(result.title, "Say hello");
  const content = await readFile(join(root, "demo/greet.demo.ts"), "utf8");
  assert.match(content, /title: "Say hello"/);
  assert.match(content, /file: "examples\/greet\.clo"/);
});

test("writeScaffold refuses an existing target unless force is set", async () => {
  const root = await makeRoot();
  await mkdir(join(root, "demo"), { recursive: true });
  await writeFile(join(root, "examples.clo"), "fn main() {}\n", "utf8");
  const target = join(root, "demo/examples.demo.ts");
  await writeFile(target, "// sentinel\n", "utf8");

  await assert.rejects(() => writeScaffold({ root, file: "examples.clo" }), ConfigError);
  assert.equal(await readFile(target, "utf8"), "// sentinel\n");

  await writeScaffold({ root, file: "examples.clo", force: true });
  assert.match(await readFile(target, "utf8"), /export default defineDemo\(/);
});

test("writeScaffold rejects a missing source and a path that escapes the root", async () => {
  const root = await makeRoot();
  await assert.rejects(() => writeScaffold({ root, file: "missing.clo" }), ConfigError);
  await assert.rejects(() => writeScaffold({ root, file: "../outside.clo" }), PathError);
});
