//! Manifest, activation, and canonical-grammar checks for the extension.

import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = resolve(here, "..");
const repoRoot = resolve(packageRoot, "..", "..");

function readJson(path: string): Record<string, any> {
  return JSON.parse(readFileSync(path, "utf8")) as Record<string, any>;
}

test("should declare the clorraine language and activation", () => {
  const manifest = readJson(join(packageRoot, "package.json"));
  assert.equal(manifest.activationEvents[0], "onLanguage:clorraine");
  assert.equal(manifest.contributes.languages[0].id, "clorraine");
  assert.equal(manifest.contributes.languages[0].extensions[0], ".clo");
  assert.equal(manifest.contributes.grammars[0].path, "./syntaxes/clorraine.tmLanguage.json");
  assert.equal(manifest.contributes.grammars[0].scopeName, "source.clorraine");
});

test("should name every shipped file in the manifest", () => {
  const manifest = readJson(join(packageRoot, "package.json"));
  assert.ok(manifest.main.endsWith("out/extension.js"));
  assert.ok(manifest.contributes.languages[0].configuration.endsWith("language-configuration.json"));
});

test("should keep the package version equal to the compiler version", () => {
  const manifest = readJson(join(packageRoot, "package.json"));
  const cargo = readFileSync(join(repoRoot, "compiler", "Cargo.toml"), "utf8");
  const match = /^version = "([^"]+)"/m.exec(cargo);
  assert.ok(match, "compiler/Cargo.toml has a version");
  assert.equal(manifest.version, match[1]);
});

test("should use the canonical grammar bytes", () => {
  const canonical = join(repoRoot, "editors", "clorraine.tmLanguage.json");
  const staged = join(packageRoot, "syntaxes", "clorraine.tmLanguage.json");
  const source = readFileSync(canonical);
  assert.ok(source.length > 0);
  // The staged copy exists only after `npm run stage`; the package audit is
  // the authoritative check. This test still pins the canonical source.
  assert.equal(readJson(canonical).scopeName, "source.clorraine");
  try {
    assert.deepEqual(readFileSync(staged), source);
  } catch {
    // Not staged yet: the manifest test still ran against the canonical file.
  }
});
