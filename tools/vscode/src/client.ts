//! A minimal LSP client over a child process.
//!
//! The extension has no runtime npm dependency, so this module owns the
//! framing, request correlation, notification routing, and shutdown sequence.
//! It does not import the `vscode` module: the extension adapter in
//! `extension.ts` maps these plain values into the editor API, and the headless
//! tests can drive the same client from Node.

import { ChildProcess, spawn } from "node:child_process";

/** A zero-based protocol position. */
export interface Position {
  line: number;
  character: number;
}

/** A protocol range. */
export interface Range {
  start: Position;
  end: Position;
}

/** One diagnostic published by the server. */
export interface Diagnostic {
  range: Range;
  severity?: number;
  code?: string | number;
  source?: string;
  message: string;
}

/** The `textDocument/publishDiagnostics` params. */
export interface PublishDiagnosticsParams {
  uri: string;
  version?: number;
  diagnostics: Diagnostic[];
}

/** A definition location. */
export interface Location {
  uri: string;
  range: Range;
}

/** A hover result. */
export interface Hover {
  contents: string | { kind: string; value: string };
  range?: Range;
}

/** A completion item. */
export interface CompletionItem {
  label: string;
  kind?: number;
  detail?: string;
}

/** One document symbol. */
export interface DocumentSymbol {
  name: string;
  kind: number;
  range: Range;
  selectionRange: Range;
}

/** The initialize result this client needs. */
export interface InitializeResult {
  capabilities: Record<string, unknown>;
  serverInfo?: { name: string; version?: string };
}

/** A JSON-RPC error response. */
export class LspError extends Error {
  readonly code: number;

  constructor(code: number, message: string) {
    super(message);
    this.name = "LspError";
    this.code = code;
  }
}

/** Options for one client instance. */
export interface LspClientOptions {
  command: string;
  args?: string[];
  cwd?: string;
  env?: NodeJS.ProcessEnv;
  rootUri?: string;
  initializationOptions?: unknown;
  onDiagnostics?: (params: PublishDiagnosticsParams) => void;
  onNotification?: (method: string, params: unknown) => void;
  onExit?: (code: number | null) => void;
}

interface Pending {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
}

interface WireMessage {
  id?: number | string;
  method?: string;
  params?: unknown;
  result?: unknown;
  error?: { code: number; message: string };
}

const MAX_BODY = 32 * 1024 * 1024;
const STOP_TIMEOUT_MS = 2000;

/** A stdio language-server client. */
export class LspClient {
  private child: ChildProcess | undefined;
  private buffer = Buffer.alloc(0);
  private pending = new Map<string, Pending>();
  private nextId = 1;
  private stderr = "";
  private disposed = false;
  private initialized = false;

  private readonly options: LspClientOptions;

  constructor(options: LspClientOptions) {
    this.options = options;
  }

  /** Starts the server and completes the initialize handshake. */
  async start(): Promise<InitializeResult> {
    if (this.child) {
      throw new Error("the client is already started");
    }
    if (!this.options.command) {
      throw new Error("no language-server command was configured");
    }
    const child = spawn(this.options.command, this.options.args ?? [], {
      cwd: this.options.cwd,
      env: this.options.env ?? process.env,
      stdio: ["pipe", "pipe", "pipe"],
    });
    this.child = child;
    child.stdout?.on("data", (chunk: Buffer) => this.receive(chunk));
    child.stderr?.on("data", (chunk: Buffer) => {
      this.stderr = (this.stderr + chunk.toString("utf8")).slice(-4096);
    });
    child.on("exit", (code) => {
      this.disposed = true;
      const error = new Error(
        this.stderr.trim() || `clorraine-lsp exited with code ${code ?? "null"}`,
      );
      for (const pending of this.pending.values()) {
        pending.reject(error);
      }
      this.pending.clear();
      this.options.onExit?.(code);
    });
    child.on("error", (error) => {
      for (const pending of this.pending.values()) {
        pending.reject(error);
      }
      this.pending.clear();
    });
    const result = (await this.request("initialize", {
      processId: process.pid,
      rootUri: this.options.rootUri ?? null,
      capabilities: {},
      initializationOptions: this.options.initializationOptions ?? {},
    })) as InitializeResult;
    this.initialized = true;
    this.notify("initialized", {});
    return result;
  }

  /** Sends one request and waits for its response. */
  request<T = unknown>(method: string, params: unknown): Promise<T> {
    if (this.disposed) {
      return Promise.reject(new Error("the language server is not running"));
    }
    const id = this.nextId++;
    return new Promise<T>((resolve, reject) => {
      this.pending.set(String(id), {
        resolve: (value) => resolve(value as T),
        reject,
      });
      this.write({ jsonrpc: "2.0", id, method, params });
    }).catch((error: unknown) => {
      this.pending.delete(String(id));
      throw error;
    });
  }

  /** Sends one notification. */
  notify(method: string, params: unknown): void {
    if (this.disposed) {
      return;
    }
    this.write({ jsonrpc: "2.0", method, params });
  }

  /** Shuts the server down, then kills it if it does not exit. */
  async stop(): Promise<void> {
    if (!this.child || this.disposed) {
      return;
    }
    try {
      await Promise.race([
        this.request("shutdown", null),
        new Promise((resolve) => setTimeout(resolve, STOP_TIMEOUT_MS)),
      ]);
    } catch {
      // A server that is already gone needs no shutdown.
    }
    this.notify("exit", null);
    await new Promise<void>((resolve) => {
      const timer = setTimeout(() => {
        this.child?.kill();
        resolve();
      }, STOP_TIMEOUT_MS);
      this.child?.once("exit", () => {
        clearTimeout(timer);
        resolve();
      });
    });
    this.disposed = true;
  }

  /** Sends one framed message. */
  private write(message: unknown): void {
    if (!this.child?.stdin?.writable) {
      return;
    }
    const body = JSON.stringify(message);
    this.child.stdin.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
  }

  /** Consumes one stdout chunk and dispatches every complete frame. */
  private receive(chunk: Buffer): void {
    this.buffer = Buffer.concat([this.buffer, chunk]);
    for (;;) {
      const headerEnd = this.buffer.indexOf("\r\n\r\n");
      if (headerEnd < 0) {
        if (this.buffer.length > MAX_BODY) {
          this.buffer = Buffer.alloc(0);
          this.child?.kill();
        }
        return;
      }
      const header = this.buffer.subarray(0, headerEnd).toString("ascii");
      let length: number | undefined;
      for (const line of header.split("\r\n")) {
        const [name, value] = line.split(":", 2);
        if (name?.trim().toLowerCase() === "content-length") {
          length = Number.parseInt(value?.trim() ?? "", 10);
        }
      }
      if (length === undefined || !Number.isFinite(length) || length < 0) {
        this.buffer = Buffer.alloc(0);
        this.child?.kill();
        return;
      }
      if (length > MAX_BODY) {
        this.buffer = Buffer.alloc(0);
        this.child?.kill();
        return;
      }
      const bodyStart = headerEnd + 4;
      if (this.buffer.length < bodyStart + length) {
        return;
      }
      const body = this.buffer.subarray(bodyStart, bodyStart + length).toString("utf8");
      this.buffer = this.buffer.subarray(bodyStart + length);
      this.dispatch(body);
    }
  }

  /** Routes one decoded response or notification. */
  private dispatch(body: string): void {
    let message: WireMessage;
    try {
      message = JSON.parse(body) as WireMessage;
    } catch {
      return;
    }
    if (message.method !== undefined) {
      if (message.method === "textDocument/publishDiagnostics") {
        this.options.onDiagnostics?.(message.params as PublishDiagnosticsParams);
      } else {
        this.options.onNotification?.(message.method, message.params);
      }
      return;
    }
    const key = String(message.id);
    const pending = this.pending.get(key);
    if (!pending) {
      return;
    }
    this.pending.delete(key);
    if (message.error) {
      pending.reject(new LspError(message.error.code, message.error.message));
    } else {
      pending.resolve(message.result);
    }
  }

  /** True after the initialize handshake completed. */
  get ready(): boolean {
    return this.initialized && !this.disposed;
  }
}
