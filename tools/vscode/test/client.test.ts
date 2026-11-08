//! Headless tests for the in-repo LSP client.

import { strict as assert } from "node:assert";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { LspClient } from "../src/client.ts";

const here = dirname(fileURLToPath(import.meta.url));
const fake = join(here, "fixtures", "fake-lsp.mjs");
const crash = join(here, "fixtures", "crash-lsp.mjs");

function hover(line: number) {
  return {
    textDocument: { uri: "file:///tmp/a.clo" },
    position: { line, character: 0 },
  };
}

test("should initialize, request, and stop against a split-frame server", async () => {
  const exits: Array<number | null> = [];
  const client = new LspClient({
    command: process.execPath,
    args: [fake],
    onExit: (code) => exits.push(code),
  });
  const result = await client.start();
  assert.equal(result.serverInfo?.name, "fake");
  const hoverResult = await client.request<{ contents: { value: string } }>(
    "textDocument/hover",
    hover(0),
  );
  assert.match(hoverResult.contents.value, /func fake line 0/);
  await client.stop();
  assert.deepEqual(exits, [0], "a clean stop must exit zero");
});

test("should correlate two out-of-order responses", async () => {
  const client = new LspClient({ command: process.execPath, args: [fake] });
  await client.start();
  try {
    const first = client.request<{ contents: { value: string } }>(
      "textDocument/hover",
      hover(0),
    );
    const second = client.request<{ contents: { value: string } }>(
      "textDocument/hover",
      hover(1),
    );
    const [a, b] = await Promise.all([first, second]);
    assert.match(a.contents.value, /line 0/);
    assert.match(b.contents.value, /line 1/);
  } finally {
    await client.stop();
  }
});

test("should reject a pending request on a malformed response", async () => {
  const client = new LspClient({
    command: process.execPath,
    args: [fake],
    env: { ...process.env, FAKE_BAD_JSON: "1" },
  });
  await client.start();
  await assert.rejects(
    () => client.request("textDocument/hover", hover(0)),
    /malformed JSON/,
  );
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
