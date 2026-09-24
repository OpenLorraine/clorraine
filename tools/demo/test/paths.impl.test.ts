import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdir, mkdtemp, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { PathError, resolveRepoPath } from "../src/util/paths.ts";

async function makeRoot(): Promise<string> {
  return mkdtemp(join(tmpdir(), "clorraine-root-"));
}

test("rejects an absolute path", async () => {
  const root = await makeRoot();
  assert.throws(() => resolveRepoPath(root, "/etc/passwd", "file"), PathError);
});

test("accepts a nested repository path", async () => {
  const root = await makeRoot();
  await mkdir(join(root, "a", "b"), { recursive: true });
  assert.equal(resolveRepoPath(root, "a/b/c.mp4", "out"), join(root, "a", "b", "c.mp4"));
});

test("rejects a path that escapes through a parent directory", async () => {
  const root = await makeRoot();
  assert.throws(() => resolveRepoPath(root, "../outside.txt", "file"), PathError);
});

test("rejects a path that escapes through a symlink", async () => {
  const root = await makeRoot();
  await symlink(tmpdir(), join(root, "link"));
  assert.throws(() => resolveRepoPath(root, "link/x.txt", "file"), PathError);
});
