#!/usr/bin/env node
/**
 * External LSP peer for editor-action tests. It returns a supplied saved-file
 * selection, not a dependency parser or a compiler result.
 */
const fs = require("node:fs");

const config = JSON.parse(fs.readFileSync(process.env.ACTION_LSP_CONFIG, "utf8"));
let buffer = Buffer.alloc(0);
let shutdown = false;
const deadline = setTimeout(() => process.exit(3), 8000);

/** Append small observations so the parent can order saves and requests. */
function record(message) {
  fs.appendFileSync(config.events, `${JSON.stringify(message)}\n`);
}

/** Send real JSON-RPC framing over the child process's standard output. */
function send(id, result, error) {
  const body = JSON.stringify({ jsonrpc: "2.0", id, result, error });
  process.stdout.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
}

/** Delay only selection replies, allowing deterministic editor changes in flight. */
function select(message) {
  const reply = () => {
    if (config.selectionFailure) {
      send(message.id, undefined, { code: -32603, message: "Saved-source selection failed; retry after fixing imports." });
    } else {
      send(message.id, { files: config.files });
    }
  };
  if (!config.pendingChange) return reply();
  const timer = setInterval(() => {
    if (fs.existsSync(config.release)) {
      clearInterval(timer);
      reply();
    }
  }, 5);
}

/** Implement initialization, the read-only request, and clean shutdown only. */
function handle(message) {
  record(message);
  if (message.method === "initialize") {
    const experimental = config.capability === "missing"
      ? {} : { clorraineSourceFilesVersion: config.capability ?? 1 };
    send(message.id, {
      capabilities: { hoverProvider: true, experimental },
      serverInfo: { name: "action-fixture", version: "1.1.2" },
    });
  } else if (message.method === "clorraine/sourceFiles") {
    select(message);
  } else if (message.method === "shutdown") {
    shutdown = true;
    send(message.id, null);
  } else if (message.method === "exit") {
    clearTimeout(deadline);
    process.exit(shutdown ? 0 : 1);
  } else if (message.id !== undefined) {
    send(message.id, undefined, { code: -32601, message: "Unsupported fixture request" });
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
    const length = Number(match[1]);
    if (buffer.length < start + length) return;
    const message = JSON.parse(buffer.subarray(start, start + length).toString());
    buffer = buffer.subarray(start + length);
    handle(message);
  }
});
