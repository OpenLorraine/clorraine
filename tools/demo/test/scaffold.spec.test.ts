import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { run } from "../src/util/proc.ts";

const repoRoot = join(import.meta.dirname, "..", "..", "..");
const cli = join(import.meta.dirname, "..", "src", "cli.ts");

async function makeRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), "clorraine-scaffold-"));
}

async function writeSource(root: string, relative: string, contents: string): Promise<void> {
  const file = join(root, relative);
  await mkdir(dirname(file), { recursive: true });
  await writeFile(file, contents, "utf8");
}

function scaffold(relative: string, root: string, ...extra: string[]) {
  return run(process.execPath, [cli, "new", relative, "--root", root, ...extra], {
    cwd: repoRoot,
    stdio: "ignore",
  });
}

test("new derives the intro title from a leading comment", async () => {
  const root = await makeRoot();
  await writeSource(
    root,
    "examples/hello/hello.clo",
    "// The smallest Clorraine program: print one templated line.\nfn main() {}\n",
  );

  const result = await scaffold("examples/hello/hello.clo", root);
  assert.equal(result.code, 0);

  const content = await readFile(join(root, "demo/hello.demo.ts"), "utf8");
  assert.match(content, /import \{ defineDemo \} from "\.\.\/tools\/demo\/src\/index\.ts";/);
  assert.match(content, /out: "demo\/out\/hello\.mp4"/);
  assert.match(content, /poster: "demo\/out\/hello\.png"/);
  assert.match(content, /title: "The smallest Clorraine program: print one templated line"/);
  assert.match(content, /file: "examples\/hello\/hello\.clo"/);
  assert.ok(content.includes("make -s run FILE=examples/hello/hello.clo PROFILE=hosted-amiga"));
  assert.ok(content.includes("frame: true"));
});

test("new falls back to the file stem when there is no leading comment", async () => {
  const root = await makeRoot();
  await writeSource(root, "examples/plain.clo", "fn main() {}\n");

  const result = await scaffold("examples/plain.clo", root);
  assert.equal(result.code, 0);

  const content = await readFile(join(root, "demo/plain.demo.ts"), "utf8");
  assert.match(content, /title: "plain"/);
});

test("new refuses to overwrite without --force and replaces with it", async () => {
  const root = await makeRoot();
  await writeSource(root, "examples/plain.clo", "fn main() {}\n");
  await mkdir(join(root, "demo"), { recursive: true });
  const target = join(root, "demo/plain.demo.ts");
  await writeFile(target, "// sentinel\n", "utf8");

  const refused = await scaffold("examples/plain.clo", root);
  assert.notEqual(refused.code, 0);
  assert.equal(await readFile(target, "utf8"), "// sentinel\n");

  const forced = await scaffold("examples/plain.clo", root, "--force");
  assert.equal(forced.code, 0);
  assert.match(await readFile(target, "utf8"), /export default defineDemo\(/);
});
