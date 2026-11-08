//! Manifest, activation, and canonical-grammar checks for the extension.
//!
//! `npm test` stages the package first, so these assertions run against the
//! exact manifest and grammar bytes that `vsce` packages.

import { strict as assert } from "node:assert";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

interface LanguageContribution {
  id: string;
  extensions: string[];
  configuration: string;
}

interface GrammarContribution {
  language: string;
  scopeName: string;
  path: string;
}

interface Manifest {
  activationEvents: string[];
  main: string;
  version: string;
  contributes: {
    languages: LanguageContribution[];
    grammars: GrammarContribution[];
  };
}

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = resolve(here, "..");
const repoRoot = resolve(packageRoot, "..", "..");
const stagedRoot = join(repoRoot, "build", "vscode", "extension");

function readJson(path: string): Manifest {
  return JSON.parse(readFileSync(path, "utf8")) as Manifest;
}

function staged(relative: string): string {
  assert.ok(relative.startsWith("./"), `manifest path ${relative} is relative`);
  return join(stagedRoot, relative.slice(2));
}

test("should declare the clorraine language and activation", () => {
  const manifest = readJson(join(stagedRoot, "package.json"));
  assert.equal(manifest.activationEvents[0], "onLanguage:clorraine");
  assert.equal(manifest.contributes.languages[0].id, "clorraine");
  assert.equal(manifest.contributes.languages[0].extensions[0], ".clo");
  assert.equal(manifest.contributes.grammars[0].scopeName, "source.clorraine");
});

test("should name every shipped file in the staged manifest", () => {
  const manifest = readJson(join(stagedRoot, "package.json"));
  assert.ok(existsSync(staged(manifest.main)), `missing ${manifest.main}`);
  assert.ok(
    existsSync(staged(manifest.contributes.languages[0].configuration)),
    "missing language configuration",
  );
  assert.ok(
    existsSync(staged(manifest.contributes.grammars[0].path)),
    "missing grammar",
  );
});

test("should keep the staged package version equal to the compiler version", () => {
  const manifest = readJson(join(stagedRoot, "package.json"));
  const cargo = readFileSync(join(repoRoot, "compiler", "Cargo.toml"), "utf8");
  const match = /^version = "([^"]+)"/m.exec(cargo);
  assert.ok(match, "compiler/Cargo.toml has a version");
  assert.equal(manifest.version, match[1]);
});

test("should use the canonical grammar bytes", () => {
  const canonical = readFileSync(
    join(repoRoot, "editors", "clorraine.tmLanguage.json"),
  );
  const stagedGrammar = readFileSync(
    staged(readJson(join(stagedRoot, "package.json")).contributes.grammars[0].path),
  );
  assert.deepEqual(stagedGrammar, canonical);
});
