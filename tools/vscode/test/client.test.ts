//! Headless tests for the in-repo LSP client.

import { strict as assert } from "node:assert";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { LspClient } from "../src/client.ts";

const here = dirname(fileURLToPath(import.meta.url));
const fake = join(here, "fixtures", "fake-lsp.mjs");
const crash = join(here, "fixtures", "crash-lsp.mjs");

test("should initialize, request, and stop against a split-frame server", async () => {
  const client = new LspClient({
    command: process.execPath,
    args: [fake],
  });
  const result = await client.start();
  assert.equal(result.serverInfo?.name, "fake");
  const hover = await client.request<{ contents: { value: string } }>(
    "textDocument/hover",
    { textDocument: { uri: "file:///tmp/a.clo" }, position: { line: 0, character: 0 } },
  );
  assert.match(hover.contents.value, /func fake/);
  await client.stop();
});

test("should reject start when the server exits immediately", async () => {
  const exits: Array<number | null> = [];
  const client = new LspClient({
    command: process.execPath,
    args: [crash],
    onExit: (code) => exits.push(code),
  });
  await assert.rejects(() => client.start(), /fake crash|exited/);
  assert.deepEqual(exits, [1]);
});

test("should reject a pending request when the server dies mid-request", async () => {
  const client = new LspClient({
    command: process.execPath,
    args: [fake],
    env: { ...process.env, FAKE_DIE_ON_HOVER: "1" },
  });
  await client.start();
  await assert.rejects(
    () =>
      client.request("textDocument/hover", {
        textDocument: { uri: "file:///tmp/a.clo" },
        position: { line: 0, character: 0 },
      }),
    /exited|not running/,
  );
});
