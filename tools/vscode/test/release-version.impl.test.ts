//! Edge cases of the release version guard: usage errors, unreadable or
//! malformed manifests, dependency tables that also carry versions, and the
//! GitHub Actions output file.

import { strict as assert } from "node:assert";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { after, test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const script = resolve(here, "..", "scripts", "release-version.mjs");
const work = mkdtempSync(join(tmpdir(), "release-version-impl-"));
after(() => rmSync(work, { recursive: true, force: true }));

let fixture = 0;

/** Writes `text` to a fresh file under the work directory and returns its path. */
function file(name: string, text: string): string {
  const path = join(work, `${fixture++}-${name}`);
  writeFileSync(path, text);
  return path;
}

/** Runs the guard with raw arguments and an optional extra environment. */
function run(args: string[], env: Record<string, string> = {}) {
  const base = { ...process.env };
  delete base.GITHUB_OUTPUT;
  return spawnSync(process.execPath, [script, ...args], { encoding: "utf8", env: { ...base, ...env } });
}

const manifest = () => file("package.json", '{"version": "1.3.0"}\n');

test("should read the package version, not a dependency version", () => {
  const cargo = file("Cargo.toml",
    '[workspace]\nmembers = ["x"]\n\n[package]\nname = "c"\nversion = "1.3.0"\n\n[dependencies]\nfoo = { version = "9.9.9" }\n');
  const result = run(["--ref", "refs/tags/v1.3.0", "--cargo", cargo, "--manifest", manifest()]);
  assert.equal(result.status, 0, result.stderr);
});

test("should fail as a usage error when a required option is missing", () => {
  const result = run(["--ref", "refs/heads/main", "--cargo", "x"]);
  assert.equal(result.status, 2);
  assert.equal(result.stderr.trim(), "release-version: --manifest is required");
});

test("should fail as a usage error on an unknown argument or a missing value", () => {
  assert.equal(run(["--tag", "v1"]).stderr.trim(), "release-version: unknown argument --tag");
  assert.equal(run(["--ref"]).status, 2);
});

test("should name an unreadable file", () => {
  const missing = join(work, "absent.toml");
  const result = run(["--ref", "refs/heads/main", "--cargo", missing, "--manifest", manifest()]);
  assert.equal(result.status, 2);
  assert.equal(result.stderr.trim(), `release-version: cannot read ${missing}`);
});

test("should reject a Cargo manifest without a package version", () => {
  const cargo = file("Cargo.toml", '[workspace]\nversion = "1.3.0"\n');
  const result = run(["--ref", "refs/heads/main", "--cargo", cargo, "--manifest", manifest()]);
  assert.equal(result.status, 2);
  assert.match(result.stderr, /no \[package\] version in /);
});

test("should reject an extension manifest that is not JSON or has no version", () => {
  const cargo = file("Cargo.toml", '[package]\nversion = "1.3.0"\n');
  const broken = run(["--ref", "refs/heads/main", "--cargo", cargo, "--manifest", file("package.json", "{")]);
  assert.equal(broken.status, 2);
  assert.match(broken.stderr, /is not valid JSON/);
  const empty = run(["--ref", "refs/heads/main", "--cargo", cargo, "--manifest", file("package.json", "{}")]);
  assert.equal(empty.status, 2);
  assert.match(empty.stderr, /no version in /);
});

test("should append the version to the GitHub Actions output file", () => {
  const cargo = file("Cargo.toml", '[package]\nversion = "1.3.0"\n');
  const output = file("github-output", "existing=1\n");
  const result = run(["--ref", "refs/tags/v1.3.0", "--cargo", cargo, "--manifest", manifest()],
    { GITHUB_OUTPUT: output });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(readFileSync(output, "utf8"), "existing=1\nversion=1.3.0\n");
});

test("should check the repository's own manifests as a dry run", () => {
  const repo = resolve(here, "..", "..", "..");
  const result = run(["--ref", "refs/heads/main", "--cargo", join(repo, "compiler", "Cargo.toml"),
    "--manifest", join(repo, "tools", "vscode", "package.json")]);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /^release-version: \d+\.\d+\.\d+\n$/);
});
