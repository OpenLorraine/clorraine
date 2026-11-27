//! Audit edge cases beyond the listing specification: a small icon, the
//! maintainer notes, and each manifest listing property on its own.

import { strict as assert } from "node:assert";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { after, test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = resolve(here, "..");
const staged = resolve(packageRoot, "..", "..", "build", "vscode", "extension");
const audit = join(packageRoot, "scripts", "audit_vsix.py");
const work = mkdtempSync(join(process.env.CODEOPS_TMPDIR ?? tmpdir(), "audit-impl-"));
after(() => rmSync(work, { recursive: true, force: true }));

let counter = 0;

/**
 * Builds an archive from the staged tree after `edit` (Python source run with
 * `files`, a dict of relative path → bytes, and `manifest`, the parsed
 * package.json) and returns the audit's exit status and output.
 */
function auditAfter(edit: string): { status: number | null; output: string } {
  const archive = join(work, `${counter++}.vsix`);
  const build = spawnSync("python3", ["-c", `
import json, pathlib, struct, sys, zipfile, zlib
staged, archive, edit = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3]
files = {p.relative_to(staged).as_posix(): p.read_bytes() for p in staged.rglob("*") if p.is_file()}
manifest = json.loads(files["package.json"])
def png(width, height):
    chunk = lambda kind, data: struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    return b"\\x89PNG\\r\\n\\x1a\\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)) + chunk(b"IEND", b"")
exec(edit)
files["package.json"] = json.dumps(manifest).encode()
with zipfile.ZipFile(archive, "w") as out:
    for name, data in sorted(files.items()):
        out.writestr("extension/" + name, data)
`, staged, archive, edit], { encoding: "utf8" });
  assert.equal(build.status, 0, build.stderr);
  const result = spawnSync("python3", [audit, "--vsix", archive], { encoding: "utf8" });
  return { status: result.status, output: `${result.stdout}${result.stderr}` };
}

/** Asserts the audit refuses the edited archive with a message containing `expected`. */
function refuses(edit: string, expected: string): void {
  const result = auditAfter(edit);
  assert.equal(result.status, 1, result.output);
  assert.ok(result.output.includes(expected), `${expected} not in: ${result.output}`);
}

test("should accept a minimal valid icon of exactly 128 pixels", () => {
  const result = auditAfter('files["images/icon.png"] = png(128, 128)');
  assert.equal(result.status, 0, result.output);
});

test("should refuse an icon smaller than 128 pixels in either direction", () => {
  refuses('files["images/icon.png"] = png(64, 256)', "needs at least 128x128");
  refuses('files["images/icon.png"] = png(256, 127)', "needs at least 128x128");
});

test("should refuse packaged maintainer notes", () => {
  refuses('files["DEVELOPMENT.md"] = b"notes"', "forbidden entry: extension/DEVELOPMENT.md");
});

test("should refuse each missing manifest listing property", () => {
  refuses('manifest["icon"] = "images/icon.svg"', "icon is not images/icon.png");
  refuses('manifest["extensionKind"] = ["ui"]', "extensionKind");
  refuses('manifest.pop("capabilities")', "untrustedWorkspaces");
  refuses('manifest["contributes"]["commands"] = [c for c in manifest["contributes"]["commands"] if c["command"] != "clorraine.driverPath"]',
    "does not contribute clorraine.driverPath");
  refuses('manifest["contributes"]["menus"].pop("commandPalette")', "does not hide clorraine.driverPath");
});
