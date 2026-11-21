#!/usr/bin/env node
/**
 * A real executable identity and bounded stdio LSP peer. Every installed copy
 * reports the same version: tests must distinguish its actual executed path.
 * It supplies saved source paths, not discovery, compilation or action logic.
 */
const fs = require("node:fs");
const { pathToFileURL } = require("node:url");
const config = JSON.parse(fs.readFileSync(process.env.DISCOVERY_CONFIG, "utf8"));
const executable = fs.realpathSync(process.argv[1]);
let buffer = Buffer.alloc(0);
let profile;
let shutdown = false;

/** Record only small process/protocol observations in the owned scratch tree. */
function record(value) {
  fs.appendFileSync(config.events, `${JSON.stringify({ executable, ...value })}\n`);
}

record({ kind: "exec", args: process.argv.slice(2) });
if (process.argv[2] === "--version") {
  console.log(`${executable.endsWith("clorraine-lsp") ? "clorraine-lsp" : "clorraine"} 1.1.2`);
  process.exit(0);
}
const deadline = setTimeout(() => process.exit(3), 10000);

/** Send actual Content-Length framing, including unsolicited diagnostics. */
function send(value) {
  const body = JSON.stringify({ jsonrpc: "2.0", ...value });
  process.stdout.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
}

/** Diagnostics expose the profile received over the real client connection. */
function diagnostics() {
  send({ method: "textDocument/publishDiagnostics", params: {
    uri: pathToFileURL(config.entry).href,
    diagnostics: [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
      severity: 2, message: `profile:${profile}`, source: "discovery-peer" }],
  } });
}

/** Implement only the external protocol needed to observe discovery/lifecycle. */
function handle(message) {
  record({ kind: "protocol", ...message });
  if (message.method === "initialize") {
    profile = message.params?.initializationOptions?.profile;
    send({ id: message.id, result: {
      capabilities: { hoverProvider: true, experimental: { clorraineSourceFilesVersion: 1 } },
      serverInfo: { name: executable, version: "1.1.2" },
    } });
  } else if (message.method === "initialized" || message.method === "textDocument/didOpen") {
    diagnostics();
  } else if (message.method === "workspace/didChangeConfiguration") {
    const settings = message.params?.settings;
    profile = settings?.clorraine?.profile ?? settings?.profile ?? profile;
    diagnostics();
  } else if (message.method === "clorraine/sourceFiles") {
    send({ id: message.id, result: { files: [config.entry] } });
  } else if (message.method === "shutdown") {
    shutdown = true;
    send({ id: message.id, result: null });
  } else if (message.method === "exit") {
    clearTimeout(deadline);
    process.exit(shutdown ? 0 : 1);
  } else if (message.id !== undefined) {
    send({ id: message.id, error: { code: -32601, message: "Unsupported peer request" } });
  }
}

process.stdin.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  if (buffer.length > 1024 * 1024) process.exit(4);
  for (;;) {
    const end = buffer.indexOf("\r\n\r\n");
    if (end < 0) return;
    const match = /Content-Length:\s*(\d+)/i.exec(buffer.subarray(0, end).toString());
    if (!match || Number(match[1]) > 1024 * 1024) process.exit(4);
    const start = end + 4;
    const size = Number(match[1]);
    if (buffer.length < start + size) return;
    const message = JSON.parse(buffer.subarray(start, start + size).toString());
    buffer = buffer.subarray(start + size);
    handle(message);
  }
});
