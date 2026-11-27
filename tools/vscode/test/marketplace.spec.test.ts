//! The Marketplace listing: manifest metadata, the files the package must
//! carry, the audit that guards them, and the user-facing listing README.
//!
//! `npm test` stages the package first, so the staged-tree checks see exactly
//! what `vsce package` would pack. Audit negatives build small archives from
//! the staged tree with one property broken at a time.

import { strict as assert } from "node:assert";
import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { after, test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = resolve(here, "..");
const repoRoot = resolve(packageRoot, "..", "..");
const staged = join(repoRoot, "build", "vscode", "extension");
const audit = join(packageRoot, "scripts", "audit_vsix.py");
const work = mkdtempSync(join(process.env.CODEOPS_TMPDIR ?? tmpdir(), "marketplace-spec-"));
after(() => rmSync(work, { recursive: true, force: true }));

const quickStart = "https://openlorraine.github.io/clorraine/guide/quick-start";

/** A parsed JSON file. */
function json(path: string): Record<string, any> {
  return JSON.parse(readFileSync(path, "utf8"));
}

/** Width and height from a PNG's IHDR chunk, or undefined when the bytes are not a PNG. */
function pngSize(path: string): { width: number; height: number } | undefined {
  const bytes = readFileSync(path);
  const signature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  if (bytes.length < 24 || !bytes.subarray(0, 8).equals(signature) || bytes.toString("ascii", 12, 16) !== "IHDR") {
    return undefined;
  }
  return { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
}

/**
 * Zips the staged tree under `extension/` with one mutation applied and runs
 * the audit on it. Mutations: none, drop=<path>, replace=<path>:<text>,
 * manifest=<key>:<json>.
 */
function auditWith(mutation: string): { status: number | null; output: string } {
  const archive = join(work, `${mutation.replace(/[^a-z0-9]+/gi, "-") || "clean"}.vsix`);
  const build = spawnSync("python3", ["-c", `
import json, pathlib, sys, zipfile
staged, archive, mutation = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3]
kind, _, arg = mutation.partition("=")
with zipfile.ZipFile(archive, "w") as out:
    for path in sorted(p for p in staged.rglob("*") if p.is_file()):
        rel = path.relative_to(staged).as_posix()
        data = path.read_bytes()
        if kind == "drop" and rel == arg:
            continue
        if kind == "replace" and rel == arg.split(":", 1)[0]:
            data = arg.split(":", 1)[1].encode()
        if kind == "manifest" and rel == "package.json":
            manifest = json.loads(data)
            key, value = arg.split(":", 1)
            manifest[key] = json.loads(value)
            data = json.dumps(manifest).encode()
        out.writestr("extension/" + rel, data)
`, staged, archive, mutation], { encoding: "utf8" });
  assert.equal(build.status, 0, build.stderr);
  const result = spawnSync("python3", [audit, "--vsix", archive], { encoding: "utf8" });
  return { status: result.status, output: `${result.stdout}${result.stderr}` };
}

test("should carry the Marketplace listing metadata in the manifest", () => {
  const manifest = json(join(packageRoot, "package.json"));
  assert.equal(manifest.icon, "images/icon.png");
  assert.match(manifest.homepage, /^https:\/\//);
  assert.match(manifest.bugs?.url, /^https:\/\//);
  assert.ok(Array.isArray(manifest.keywords) && manifest.keywords.length > 0 && manifest.keywords.length <= 30);
  assert.equal(typeof manifest.galleryBanner?.color, "string");
  assert.equal(manifest.preview, true);
  assert.deepEqual(manifest.extensionKind, ["workspace"]);
  assert.equal(manifest.capabilities?.untrustedWorkspaces?.supported, "limited");
  assert.equal(typeof manifest.capabilities?.untrustedWorkspaces?.description, "string");
  assert.deepEqual(manifest.activationEvents, ["onLanguage:clorraine"]);
  const commands: { command: string }[] = manifest.contributes.commands;
  assert.ok(commands.some((item) => item.command === "clorraine.driverPath"), "the driver command is contributed");
  const palette: { command: string; when: string }[] = manifest.contributes.menus?.commandPalette ?? [];
  assert.ok(palette.some((item) => item.command === "clorraine.driverPath" && item.when === "false"),
    "the driver command is hidden from the Command Palette");
});

test("should stage the changelog and a PNG icon of at least 128 pixels, but not the developer notes", () => {
  assert.ok(existsSync(join(staged, "CHANGELOG.md")), "CHANGELOG.md is staged");
  const icon = join(staged, "images", "icon.png");
  assert.ok(existsSync(icon), "images/icon.png is staged");
  const size = pngSize(icon);
  assert.ok(size, "the icon is a PNG");
  assert.ok(size.width >= 128 && size.height >= 128, `icon is ${size.width}x${size.height}`);
  assert.ok(!existsSync(join(staged, "DEVELOPMENT.md")), "developer notes are not packaged");
  const cargo = readFileSync(join(repoRoot, "compiler", "Cargo.toml"), "utf8");
  assert.equal(json(join(staged, "package.json")).version, /^version = "([^"]+)"/m.exec(cargo)?.[1]);
});

test("should pass the audit for the staged package and reject a broken listing", () => {
  const clean = auditWith("none");
  assert.equal(clean.status, 0, clean.output);
  for (const mutation of [
    "drop=CHANGELOG.md",
    "drop=images/icon.png",
    "replace=images/icon.png:<svg xmlns='http://www.w3.org/2000/svg'/>",
    "manifest=preview:false",
  ]) {
    const result = auditWith(mutation);
    assert.notEqual(result.status, 0, `the audit accepted ${mutation}`);
    assert.match(result.output, /audit_vsix: /, mutation);
  }
});

test("should present a self-contained listing README", () => {
  const readme = readFileSync(join(staged, "README.md"), "utf8");
  assert.ok(readme.includes("Linux x86_64 and Windows 11 x64; macOS is not supported yet"), "platforms line");
  assert.ok(readme.includes("trademarks of Amiga Corporation"), "trademark line");
  assert.ok(readme.includes(`${quickStart}#install-on-linux`), "Linux install guide link");
  assert.ok(readme.includes(`${quickStart}#install-on-windows`), "Windows install guide link");
  assert.ok(!readme.includes("make vscode-package"), "developer notes moved out of the listing");
  const targets = [...readme.matchAll(/\]\(([^)\s]+)\)/g)].map((match) => match[1]);
  assert.ok(targets.length > 0, "the README links somewhere");
  for (const target of targets) {
    assert.match(target, /^https:\/\//, `link ${target} is not absolute https`);
  }
});
