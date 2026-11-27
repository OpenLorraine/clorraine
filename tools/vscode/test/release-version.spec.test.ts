//! Behaviour of the release version guard used by the publish workflow.
//!
//! A Marketplace version can be published only once, so the guard must refuse
//! any mismatch between the release tag, the compiler version, and the
//! extension manifest before anything is packaged or published.

import { strict as assert } from "node:assert";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { after, test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const script = resolve(here, "..", "scripts", "release-version.mjs");
const work = mkdtempSync(join(tmpdir(), "release-version-spec-"));
after(() => rmSync(work, { recursive: true, force: true }));

let fixture = 0;

/** Runs the guard against a fresh Cargo manifest and extension manifest. */
function guard(ref: string, cargo: string, manifest: string, publish = false) {
  const dir = join(work, String(fixture++));
  const cargoPath = `${dir}-Cargo.toml`;
  const manifestPath = `${dir}-package.json`;
  writeFileSync(cargoPath, `[package]\nname = "clorraine"\nversion = "${cargo}"\nedition = "2021"\n`);
  writeFileSync(manifestPath, `${JSON.stringify({ name: "clorraine", version: manifest }, null, 2)}\n`);
  const args = [script, "--ref", ref, "--cargo", cargoPath, "--manifest", manifestPath];
  if (publish) {
    args.push("--publish");
  }
  const env = { ...process.env };
  delete env.GITHUB_OUTPUT;
  return spawnSync(process.execPath, args, { encoding: "utf8", env });
}

test("should accept a release tag that matches both versions", () => {
  const result = guard("refs/tags/v1.3.0", "1.3.0", "1.3.0", true);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout.trim(), "release-version: 1.3.0");
});

test("should refuse a tag that does not match the compiler version", () => {
  const result = guard("refs/tags/v1.3.0", "1.2.0", "1.2.0", true);
  assert.equal(result.status, 1);
  assert.equal(result.stderr.trim(), "release-version: tag v1.3.0 does not match version 1.2.0");
});

test("should refuse a revision tag", () => {
  const result = guard("refs/tags/v1.3.0-r1", "1.3.0", "1.3.0", true);
  assert.equal(result.status, 1);
  assert.equal(result.stderr.trim(), "release-version: tag v1.3.0-r1 does not match version 1.3.0");
});

test("should allow a branch only for a dry run", () => {
  const dry = guard("refs/heads/main", "1.3.0", "1.3.0");
  assert.equal(dry.status, 0, dry.stderr);
  const publish = guard("refs/heads/main", "1.3.0", "1.3.0", true);
  assert.equal(publish.status, 1);
  assert.equal(publish.stderr.trim(), "release-version: publishing requires a release tag, got refs/heads/main");
});

test("should refuse a compiler version that differs from the extension manifest", () => {
  const result = guard("refs/heads/main", "1.3.0", "1.2.0");
  assert.equal(result.status, 1);
  assert.equal(
    result.stderr.trim(),
    "release-version: compiler version 1.3.0 does not match extension manifest 1.2.0",
  );
});
