//! Edge cases of the version comparison and the notice text.

import { strict as assert } from "node:assert";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const load = createRequire(import.meta.url);
const guide = load(resolve(here, "../../../build/vscode/extension/out/guide.js")) as {
  serverOlderThan(server: string | undefined, extension: string): boolean;
  driftMessage(server: string, extension: string): string;
};

test("should compare each version part numerically", () => {
  assert.equal(guide.serverOlderThan("1.2.9", "1.3.0"), true);
  assert.equal(guide.serverOlderThan("0.99.99", "1.0.0"), true);
  assert.equal(guide.serverOlderThan("2.0.0", "1.99.99"), false);
  assert.equal(guide.serverOlderThan("01.2.0", "1.3.0"), true);
});

test("should ignore versions that are not plain major.minor.patch", () => {
  for (const server of ["", "1.2", "1.2.3.4", "1.2.3-rc1", " 1.2.0", "v1.2.0"]) {
    assert.equal(guide.serverOlderThan(server, "1.3.0"), false, server);
  }
  assert.equal(guide.serverOlderThan("1.2.0", "1.3.0-dev"), false);
});

test("should name both versions in the notice", () => {
  assert.equal(guide.driftMessage("1.1.2", "1.3.0"),
    "Clorraine CLI 1.1.2 is older than the extension 1.3.0. Run the installer to update.");
});
