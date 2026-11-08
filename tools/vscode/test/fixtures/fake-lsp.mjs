// A tiny LSP server used by the client tests. It is not the real Clorraine
// server; it only proves framing, correlation, notifications, and shutdown.

import { stdin } from "node:process";

let buffer = Buffer.alloc(0);
let shutdown = false;

function send(message) {
  const body = JSON.stringify(message);
  process.stdout.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
}

function sendSplit(message) {
  const body = JSON.stringify(message);
  const header = `Content-Length: ${Buffer.byteLength(body)}\r\n\r\n`;
  process.stdout.write(header);
  setTimeout(() => process.stdout.write(body), 10);
}

stdin.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  for (;;) {
    const end = buffer.indexOf("\r\n\r\n");
    if (end < 0) {
      return;
    }
    const header = buffer.subarray(0, end).toString("ascii");
    const match = /content-length:\s*(\d+)/i.exec(header);
    if (!match) {
      process.exit(2);
    }
    const length = Number(match[1]);
    const start = end + 4;
    if (buffer.length < start + length) {
      return;
    }
    const body = buffer.subarray(start, start + length).toString("utf8");
    buffer = buffer.subarray(start + length);
    const message = JSON.parse(body);
    if (message.method === "initialize") {
      sendSplit({
        jsonrpc: "2.0",
        id: message.id,
        result: { capabilities: { hoverProvider: true }, serverInfo: { name: "fake" } },
      });
    } else if (message.method === "textDocument/hover") {
      if (process.env.FAKE_DIE_ON_HOVER === "1") {
        process.exit(1);
      }
      if (process.env.FAKE_BAD_JSON === "1") {
        const body = "nope!";
        process.stdout.write(`Content-Length: ${body.length}\r\n\r\n${body}`);
      } else {
        const line = message.params?.position?.line ?? 0;
        const reply = () =>
          send({
            jsonrpc: "2.0",
            id: message.id,
            result: {
              contents: { kind: "markdown", value: `func fake line ${line}` },
            },
          });
        if (line === 0) {
          setTimeout(reply, 50);
        } else {
          reply();
        }
      }
    } else if (message.method === "shutdown") {
      shutdown = true;
      send({ jsonrpc: "2.0", id: message.id, result: null });
    } else if (message.method === "exit") {
      process.exit(shutdown ? 0 : 1);
    }
  }
});
