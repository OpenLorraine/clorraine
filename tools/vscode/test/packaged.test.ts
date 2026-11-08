//! Exercise the packaged client against the real language server.
//!
//! `make vscode-check` sets `CLORRAINE_VSIX` and `CLORRAINE_LSP`. Without them
//! the test skips, so a plain `npm test` remains a fast unit run.

import { strict as assert } from "node:assert";
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

import type { LspClient as ClientType } from "../src/client.ts";

const here = dirname(fileURLToPath(import.meta.url));
const packageRoot = resolve(here, "..");
const repoRoot = resolve(packageRoot, "..", "..");

test("should run the packaged client against the real server", async (context) => {
  const vsix = process.env.CLORRAINE_VSIX;
  const server = process.env.CLORRAINE_LSP;
  if (!vsix || !server) {
    context.skip("CLORRAINE_VSIX and CLORRAINE_LSP are set by make vscode-check");
    return;
  }
  const extracted = join(repoRoot, "build", "vscode", "extracted");
  const audit = spawnSync(
    "python3",
    [
      join(packageRoot, "scripts", "audit_vsix.py"),
      "--vsix",
      vsix,
      "--extract",
      extracted,
    ],
    { cwd: repoRoot, encoding: "utf8" },
  );
  assert.equal(audit.status, 0, audit.stderr || audit.stdout);
  const extensionRoot = join(extracted, "extension");
  const manifest = JSON.parse(
    readFileSync(join(extensionRoot, "package.json"), "utf8"),
  ) as {
    main: string;
    version: string;
    contributes: {
      languages: Array<{ configuration: string }>;
      grammars: Array<{ path: string }>;
    };
  };
  assert.equal(manifest.main, "./out/extension.js");
  assert.ok(existsSync(join(extensionRoot, manifest.main.slice(2))));
  const configuration = manifest.contributes.languages[0].configuration;
  assert.equal(configuration, "./language-configuration.json");
  assert.ok(existsSync(join(extensionRoot, configuration.slice(2))));
  const grammar = manifest.contributes.grammars[0].path;
  assert.ok(grammar.startsWith("./"));
  assert.deepEqual(
    readFileSync(join(extensionRoot, grammar.slice(2))),
    readFileSync(join(repoRoot, "editors", "clorraine.tmLanguage.json")),
    "the packaged grammar must equal the canonical grammar",
  );
  const cargo = readFileSync(join(repoRoot, "compiler", "Cargo.toml"), "utf8");
  const version = /^version = "([^"]+)"/m.exec(cargo)?.[1];
  assert.equal(manifest.version, version);

  const require = createRequire(import.meta.url);
  const packaged = require(join(extracted, "extension", "out", "client.js")) as {
    LspClient: new (options: Record<string, unknown>) => ClientType;
  };
  let diagnostics: { uri: string; diagnostics: unknown[] } | undefined;
  let resolveDiagnostics!: (value: { uri: string; diagnostics: unknown[] }) => void;
  const received = new Promise<{ uri: string; diagnostics: unknown[] }>(
    (resolvePromise) => {
      resolveDiagnostics = resolvePromise;
    },
  );
  const client = new packaged.LspClient({
    command: server,
    rootUri: pathToFileURL(join(here, "fixtures", "workspace")).toString(),
    initializationOptions: { profile: "host" },
    onDiagnostics: (params: { uri: string; diagnostics: unknown[] }) => {
      diagnostics = params;
      resolveDiagnostics(params);
    },
  });
  await client.start();
  const uri = pathToFileURL(join(here, "fixtures", "workspace", "main.clo")).toString();
  client.notify("textDocument/didOpen", {
    textDocument: {
      uri,
      languageId: "clorraine",
      version: 1,
      text: `module app.vscode;\n\n/// Greets the caller.\nfunc greet(): i32 {\n    return 1;\n}\n\nfunc main(): i32 {\n    let x: i32 = null;\n    return x;\n}\n`,
    },
  });
  const published = await received;
  assert.equal(published.uri, uri);
  assert.ok(published.diagnostics.length >= 1);
  const hover = await client.request<{ contents: { value: string } }>(
    "textDocument/hover",
    { textDocument: { uri }, position: { line: 3, character: 5 } },
  );
  assert.match(hover.contents.value, /greet/);
  await client.stop();
  assert.ok(diagnostics);
});
