// Stage a VS Code extension package under build/vscode/extension.
//
// The canonical grammar lives in editors/ and is copied here, never edited in
// place. The staged package.json takes its version from compiler/Cargo.toml so
// one file is the version of record.

import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = resolve(here, "..");
const repoRoot = resolve(packageRoot, "..", "..");
const stage = join(repoRoot, "build", "vscode", "extension");

const cargo = readFileSync(join(repoRoot, "compiler", "Cargo.toml"), "utf8");
const match = /^version = "([^"]+)"/m.exec(cargo);
if (!match) {
  throw new Error("compiler/Cargo.toml has no version");
}
const version = match[1];

rmSync(stage, { recursive: true, force: true });
mkdirSync(join(stage, "syntaxes"), { recursive: true });
cpSync(join(packageRoot, "package.json"), join(stage, "package.json"));
cpSync(join(packageRoot, "README.md"), join(stage, "README.md"));
cpSync(join(repoRoot, "LICENSE"), join(stage, "LICENSE"));
cpSync(join(packageRoot, ".vscodeignore"), join(stage, ".vscodeignore"));
cpSync(
  join(packageRoot, "language-configuration.json"),
  join(stage, "language-configuration.json"),
);
cpSync(join(packageRoot, "out"), join(stage, "out"), { recursive: true });
cpSync(
  join(repoRoot, "editors", "clorraine.tmLanguage.json"),
  join(stage, "syntaxes", "clorraine.tmLanguage.json"),
);

const manifest = JSON.parse(readFileSync(join(stage, "package.json"), "utf8"));
manifest.version = version;
writeFileSync(join(stage, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`staged ${stage} at version ${version}`);
