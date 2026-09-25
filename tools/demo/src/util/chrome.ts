import { spawn } from "node:child_process";
import type { ChildProcess } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { isAbsolute, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import type { FrameSource, FrameSpec } from "../renderer/env.ts";
import { messageOf, warn } from "./log.ts";
import { run } from "./proc.ts";

/** The Chrome executable to use: `CHROME_BIN` when set, else `google-chrome`. */
export function resolveChrome(): string {
  return process.env.CHROME_BIN ?? "google-chrome";
}

/** Throws a clear error when the Chrome executable cannot run. */
export async function assertChromeAvailable(command: string): Promise<void> {
  const result = await run(command, ["--version"], { stdio: "ignore" });
  if (result.code !== 0) {
    throw new Error(
      `Chrome is required to render cards and code but could not run (${command}). ` +
        `Install Chrome or set CHROME_BIN.`,
    );
  }
}

interface Pending {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
}

/** A minimal Chrome DevTools Protocol client over the built-in WebSocket. */
class Cdp {
  private readonly socket: WebSocket;
  private readonly pending = new Map<number, Pending>();
  private readonly listeners = new Map<string, Array<(params: unknown) => void>>();
  private nextId = 1;

  private constructor(socket: WebSocket) {
    this.socket = socket;
    socket.addEventListener("message", (event) => this.onMessage(String(event.data)));
  }

  static async connect(url: string): Promise<Cdp> {
    const socket = new WebSocket(url);
    await new Promise<void>((open, reject) => {
      socket.addEventListener("open", () => open(), { once: true });
      socket.addEventListener(
        "error",
        () => reject(new Error("failed to connect to the Chrome DevTools endpoint")),
        { once: true },
      );
    });
    return new Cdp(socket);
  }

  send(method: string, params: Record<string, unknown> = {}): Promise<unknown> {
    const id = this.nextId++;
    return new Promise((resolvePromise, reject) => {
      this.pending.set(id, { resolve: resolvePromise, reject });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }

  once(event: string): Promise<void> {
    return new Promise((resolvePromise) => {
      const list = this.listeners.get(event) ?? [];
      const handler = (): void => {
        const index = list.indexOf(handler);
        if (index >= 0) {
          list.splice(index, 1);
        }
        resolvePromise();
      };
      list.push(handler);
      this.listeners.set(event, list);
    });
  }

  close(): void {
    this.socket.close();
  }

  private onMessage(raw: string): void {
    const message = JSON.parse(raw) as {
      id?: number;
      method?: string;
      params?: unknown;
      result?: unknown;
      error?: { message: string };
    };
    if (typeof message.id === "number") {
      const pending = this.pending.get(message.id);
      this.pending.delete(message.id);
      if (pending === undefined) {
        return;
      }
      if (message.error !== undefined) {
        pending.reject(new Error(message.error.message));
      } else {
        pending.resolve(message.result);
      }
      return;
    }
    if (typeof message.method === "string") {
      for (const handler of [...(this.listeners.get(message.method) ?? [])]) {
        handler(message.params);
      }
    }
  }
}

function pageUrl(assetsDir: string, spec: FrameSpec): string {
  const file = isAbsolute(spec.page) ? spec.page : resolve(assetsDir, spec.page);
  const url = pathToFileURL(file);
  if (spec.query !== undefined) {
    url.search = new URLSearchParams(spec.query).toString();
  }
  return url.href;
}

async function waitForPort(userDataDir: string): Promise<number> {
  const portFile = join(userDataDir, "DevToolsActivePort");
  for (let attempt = 0; attempt < 100; attempt++) {
    if (existsSync(portFile)) {
      const first = (await readFile(portFile, "utf8")).split("\n")[0];
      const port = Number(first);
      if (Number.isInteger(port) && port > 0) {
        return port;
      }
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error("Chrome did not expose a DevTools port");
}

async function pageWebSocketUrl(port: number): Promise<string> {
  const response = await fetch(`http://127.0.0.1:${port}/json`);
  const targets = (await response.json()) as Array<{ type: string; webSocketDebuggerUrl?: string }>;
  const page = targets.find((target) => target.type === "page" && target.webSocketDebuggerUrl !== undefined);
  if (page?.webSocketDebuggerUrl === undefined) {
    throw new Error("Chrome exposed no page target");
  }
  return page.webSocketDebuggerUrl;
}

async function waitForReady(cdp: Cdp, url: string): Promise<void> {
  for (let attempt = 0; attempt < 100; attempt++) {
    const result = (await cdp.send("Runtime.evaluate", {
      expression: "window.__ready === true",
      returnByValue: true,
    })) as { result?: { value?: unknown } };
    if (result.result?.value === true) {
      return;
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`the demo page did not finish preparing (no window.__ready): ${url}`);
}

/** Resolves once the child has exited, or after a short grace period. */
function waitForExit(child: ChildProcess): Promise<void> {
  if (child.exitCode !== null || child.signalCode !== null) {
    return Promise.resolve();
  }
  return new Promise((resolvePromise) => {
    const done = (): void => {
      clearTimeout(timer);
      resolvePromise();
    };
    const timer = setTimeout(done, 3000);
    child.once("close", done);
  });
}

/**
 * Removes the temporary profile, retrying because Chrome may still be flushing files
 * for a moment after it is killed (which makes the first `rm` fail with ENOTEMPTY).
 */
async function removeProfile(dir: string): Promise<void> {
  for (let attempt = 0; attempt < 5; attempt++) {
    try {
      await rm(dir, { recursive: true, force: true });
      return;
    } catch {
      await new Promise((resolvePromise) => setTimeout(resolvePromise, 200));
    }
  }
  warn(`could not remove the temporary Chrome profile ${dir}`);
}

/** Options for {@link ChromeFrameSource}. */
export interface ChromeFrameSourceOptions {
  /** Directory holding the HTML pages (card.html, frame.html, …). */
  assetsDir: string;
  /** Directory under which per-scene frame directories are written. */
  framesDir: string;
  /** Chrome executable override. */
  chrome?: string;
}

/** Captures frames from HTML pages with one headless Chrome session per scene. */
export class ChromeFrameSource implements FrameSource {
  private readonly options: ChromeFrameSourceOptions;

  constructor(options: ChromeFrameSourceOptions) {
    this.options = options;
  }

  async capture(spec: FrameSpec): Promise<string[]> {
    const chrome = this.options.chrome ?? resolveChrome();
    const userDataDir = await mkdtemp(join(tmpdir(), "clorraine-chrome-"));
    const child: ChildProcess = spawn(
      chrome,
      [
        "--headless=new",
        "--disable-gpu",
        "--hide-scrollbars",
        "--force-device-scale-factor=1",
        "--no-first-run",
        "--no-default-browser-check",
        `--user-data-dir=${userDataDir}`,
        "--remote-debugging-port=0",
        `--window-size=${spec.width},${spec.height}`,
      ],
      { stdio: ["ignore", "ignore", "pipe"] },
    );

    let chromeStderr = "";
    child.stderr?.setEncoding("utf8");
    child.stderr?.on("data", (chunk: string) => {
      chromeStderr += chunk;
    });
    const spawnFailed = new Promise<never>((_, reject) => child.once("error", reject));

    let cdp: Cdp | undefined;
    try {
      const port = await Promise.race([waitForPort(userDataDir), spawnFailed]);
      cdp = await Cdp.connect(await pageWebSocketUrl(port));
      await cdp.send("Page.enable");
      await cdp.send("Runtime.enable");
      await cdp.send("Emulation.setDeviceMetricsOverride", {
        width: spec.width,
        height: spec.height,
        deviceScaleFactor: spec.scale,
        mobile: false,
      });

      const frameDir = join(this.options.framesDir, spec.id);
      await mkdir(frameDir, { recursive: true });

      const loaded = cdp.once("Page.loadEventFired");
      const url = pageUrl(this.options.assetsDir, spec);
      await cdp.send("Page.navigate", { url });
      await loaded;
      await waitForReady(cdp, url);

      const frames: string[] = [];
      for (let index = 0; index < spec.count; index++) {
        const time = index / spec.fps;
        await cdp.send("Runtime.evaluate", { expression: `window.render(${time})` });
        const shot = (await cdp.send("Page.captureScreenshot", { format: "png", captureBeyondViewport: false })) as {
          data: string;
        };
        const frame = join(frameDir, `${String(index).padStart(5, "0")}.png`);
        await writeFile(frame, Buffer.from(shot.data, "base64"));
        frames.push(frame);
      }
      return frames;
    } catch (error) {
      const tail = chromeStderr.trim().split("\n").slice(-3).join(" ");
      throw new Error(`${messageOf(error)}${tail.length > 0 ? ` | Chrome: ${tail}` : ""}`);
    } finally {
      cdp?.close();
      child.kill("SIGKILL");
      await waitForExit(child);
      await removeProfile(userDataDir);
    }
  }
}
