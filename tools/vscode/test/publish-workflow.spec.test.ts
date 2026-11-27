//! Static checks of the Marketplace publish workflow.
//!
//! Publishing is permanent, so the workflow's safety properties are checked
//! from its text before it ever runs: manual dispatch only, a dry run by
//! default, the publish token confined to the publish job, pinned actions, no
//! dependency cache, and no expression interpolated into a shell script.

import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "..", "..", "..");
const workflowPath = join(repoRoot, ".github", "workflows", "publish-vscode.yml");

/** The workflow text split into lines; read lazily so a missing file fails each test. */
function lines(): string[] {
  return readFileSync(workflowPath, "utf8").split("\n");
}

/** Indentation width of a line, in spaces. */
function indent(line: string): number {
  return line.length - line.trimStart().length;
}

/**
 * The lines nested under the first `key:` found at `level` spaces of
 * indentation, starting the search at `from` and stopping at `to`.
 */
function block(all: string[], key: string, level: number, from = 0, to = all.length): string[] {
  const start = all.findIndex((line, i) =>
    i >= from && i < to && indent(line) === level && line.trim().startsWith(`${key}:`));
  assert.ok(start >= 0, `missing ${key}: at indent ${level}`);
  const body: string[] = [];
  for (let i = start + 1; i < to; i++) {
    const line = all[i];
    if (line.trim() !== "" && indent(line) <= level) {
      break;
    }
    body.push(line);
  }
  return body;
}

/** The body of one job under `jobs:`. */
function job(name: string): string[] {
  const all = lines();
  const jobsStart = all.findIndex((line) => line === "jobs:");
  assert.ok(jobsStart >= 0, "missing jobs:");
  return block(all, name, 2, jobsStart + 1);
}

/** The lines of the step named `name` in a job body, from its `- name:` line to the next step. */
function step(body: string[], name: string): string[] {
  const start = body.findIndex((line) => line.trim() === `- name: ${name}`);
  assert.ok(start >= 0, `missing step ${name}`);
  const level = indent(body[start]);
  const end = body.findIndex((line, i) => i > start && indent(line) === level && line.trim().startsWith("- "));
  return body.slice(start, end < 0 ? body.length : end);
}

/** Non-blank, non-comment lines directly under a block, trimmed. */
function entries(body: string[]): string[] {
  return body.map((line) => line.trim()).filter((line) => line !== "" && !line.startsWith("#"));
}

test("should trigger only on manual dispatch with a dry run by default", () => {
  const on = block(lines(), "on", 0);
  const triggers = on.filter((line) => indent(line) === 2 && line.trim() !== "")
    .map((line) => line.trim());
  assert.deepEqual(triggers, ["workflow_dispatch:"]);
  const inputs = block(on, "inputs", 4);
  const names = inputs.filter((line) => indent(line) === 6 && line.trim() !== "")
    .map((line) => line.trim());
  assert.deepEqual(names, ["dry_run:"]);
  const dryRun = entries(block(inputs, "dry_run", 6));
  assert.ok(dryRun.includes("type: boolean"), "dry_run must be a boolean");
  assert.ok(dryRun.includes("default: true"), "dry_run must default to true");
});

test("should confine the publish token to the publish job", () => {
  assert.deepEqual(entries(block(lines(), "permissions", 0)), ["contents: read"]);
  const build = job("build");
  assert.ok(!build.some((line) => line.includes("id-token")), "build must not request id-token");
  const publish = job("publish");
  assert.ok(entries(publish).includes("environment: marketplace"), "publish must use the marketplace environment");
  const permissions = entries(block(publish, "permissions", 4)).sort();
  assert.deepEqual(permissions, ["contents: read", "id-token: write"]);
});

test("should pin every action to a full commit SHA with a version comment", () => {
  const uses = lines().filter((line) => /^\s*-?\s*uses:/.test(line));
  assert.ok(uses.length > 0, "the workflow uses actions");
  for (const line of uses) {
    assert.match(line, /uses: [\w.-]+\/[\w.\/-]+@[0-9a-f]{40} # v\d/, line.trim());
  }
});

test("should not cache dependencies", () => {
  const text = lines().join("\n");
  assert.ok(!/^\s*cache:/m.test(text), "no cache: key");
  assert.ok(!text.includes("actions/cache"), "no actions/cache");
});

test("should publish only from a tag after re-checking the built archive", () => {
  const publish = job("publish");
  const flat = entries(publish);
  assert.ok(flat.includes("needs: build"), "publish needs build");
  const condition = flat.find((line) => line.startsWith("if:"));
  assert.ok(condition, "publish has an if: condition");
  assert.equal(
    condition,
    "if: ${{ !inputs.dry_run && startsWith(github.ref, 'refs/tags/v') }}",
    "publishing requires both an unticked dry run and a release tag",
  );
  const text = publish.join("\n");
  const hashCheck = text.indexOf("needs.build.outputs.sha256");
  const verify = step(publish, "Verify the archive").join("\n");
  assert.ok(verify.includes("needs.build.outputs.sha256"), "the verify step reads the built hash");
  assert.ok(verify.includes("sha256sum") && verify.includes("exit 1"), "the verify step hashes the download and fails on a mismatch");
  const install = text.indexOf("npm ci --ignore-scripts");
  const publishLine = publish.find((line) => line.includes("vsce publish"));
  assert.ok(hashCheck >= 0, "publish compares the downloaded archive with the built hash");
  assert.ok(install > hashCheck, "dependencies install after the hash check, without scripts");
  assert.ok(publishLine, "publish runs vsce publish");
  assert.ok(text.indexOf(publishLine) > install, "vsce publish runs last");
  assert.ok(publishLine.includes("--packagePath"), "publishes the built archive");
  assert.ok(publishLine.includes("--oidc"), "uses trusted publishing");
  assert.ok(!lines().join("\n").includes("--skip-duplicate"), "an existing version is never skipped silently");
});

test("should keep expressions out of shell scripts", () => {
  const all = lines();
  for (let i = 0; i < all.length; i++) {
    const match = /^(\s*)(-\s+)?run:\s*(.*)$/.exec(all[i]);
    if (!match) {
      continue;
    }
    const script = [match[3]];
    const base = indent(all[i]) + (match[2] ? match[2].length : 0);
    for (let j = i + 1; j < all.length && (all[j].trim() === "" || indent(all[j]) > base); j++) {
      script.push(all[j]);
    }
    assert.ok(!script.join("\n").includes("${{"), `run: at line ${i + 1} interpolates an expression`);
  }
});
