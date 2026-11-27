//! The install-guide address for each host and the language-server version
//! comparison behind the "CLI is older than the extension" notice.

import { strict as assert } from "node:assert";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const load = createRequire(import.meta.url);

/** The staged guide helpers, as the packaged extension ships them. */
const guide = load(resolve(here, "../../../build/vscode/extension/out/guide.js")) as {
  installGuideUrl(platform: NodeJS.Platform): string;
  serverOlderThan(server: string | undefined, extension: string): boolean;
};

const quickStart = "https://openlorraine.github.io/clorraine/guide/quick-start";

test("should open the Linux install guide on Linux", () => {
  assert.equal(guide.installGuideUrl("linux"), `${quickStart}#install-on-linux`);
});

test("should open the Windows install guide on Windows", () => {
  assert.equal(guide.installGuideUrl("win32"), `${quickStart}#install-on-windows`);
});

test("should open the quick start on any other host", () => {
  assert.equal(guide.installGuideUrl("darwin"), quickStart);
});

test("should report a server older than the extension", () => {
  assert.equal(guide.serverOlderThan("1.2.0", "1.3.0"), true);
  assert.equal(guide.serverOlderThan("1.9.0", "1.10.0"), true);
});

test("should not report an equal, newer, missing, or unparsable server version", () => {
  assert.equal(guide.serverOlderThan("1.3.0", "1.3.0"), false);
  assert.equal(guide.serverOlderThan("1.4.0", "1.3.0"), false);
  assert.equal(guide.serverOlderThan(undefined, "1.3.0"), false);
  assert.equal(guide.serverOlderThan("dev", "1.3.0"), false);
  assert.equal(guide.serverOlderThan("1.10.0", "1.9.0"), false);
});
