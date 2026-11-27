#!/usr/bin/env node
/**
 * A stdio LSP peer whose `serverInfo.version` comes from GUIDE_SERVER_VERSION
 * (the literal `none` omits the version). It answers only what the client
 * needs to start, restart, and stop, and counts initialize requests in
 * GUIDE_EVENTS so a test can see a restart happen.
 */
const fs = require("node:fs");

let buffer = Buffer.alloc(0);
let shutdown = false;
const deadline = setTimeout(() => process.exit(3), 10000);

/** Writes one framed JSON-RPC message. */
function send(value) {
  const body = JSON.stringify({ jsonrpc: "2.0", ...value });
  process.stdout.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
}

/** Handles one request or notification from the client. */
function handle(message) {
  if (message.method === "initialize") {
    fs.appendFileSync(process.env.GUIDE_EVENTS, "initialize\n");
    const version = process.env.GUIDE_SERVER_VERSION;
    const serverInfo = version === "none" ? { name: "clorraine-lsp" } : { name: "clorraine-lsp", version };
    send({ id: message.id, result: { capabilities: { hoverProvider: true }, serverInfo } });
  } else if (message.method === "shutdown") {
    shutdown = true;
    send({ id: message.id, result: null });
  } else if (message.method === "exit") {
    clearTimeout(deadline);
    process.exit(shutdown ? 0 : 1);
  } else if (message.id !== undefined && message.method !== undefined) {
    send({ id: message.id, result: null });
  }
}

process.stdin.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  for (;;) {
    const end = buffer.indexOf("\r\n\r\n");
    if (end < 0) return;
    const match = /content-length:\s*(\d+)/i.exec(buffer.subarray(0, end).toString("ascii"));
    if (!match) process.exit(2);
    const start = end + 4;
    const length = Number(match[1]);
    if (buffer.length < start + length) return;
    const body = buffer.subarray(start, start + length).toString("utf8");
    buffer = buffer.subarray(start + length);
    handle(JSON.parse(body));
  }
});
