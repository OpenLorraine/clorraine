/**
 * User-owned tool discovery for the Linux extension host. Workspace settings
 * never select executables, and development builds are considered only after
 * trust has been granted.
 */
import * as fs from "node:fs";
import { homedir } from "node:os";
import * as path from "node:path";

import * as vscode from "vscode";

/**
 * Machine-only path settings; language/workspace overrides are deliberately
 * ignored. `sdk.path` is not contributed any more; it is read only for the
 * one-time migration warning when `toolchain.path` is empty.
 */
export type PathSetting =
  | "lsp.path"
  | "driver.path"
  | "toolchain.path"
  | "sdk.path"
  | "fsUae.path"
  | "kickstart.path";

/**
 * Read a user setting without merging workspace-controlled configuration.
 * Nonempty paths retain their exact bytes, including spaces in filenames.
 * @throws When the user value is not a string.
 */
export function machinePath(key: PathSetting): string {
  const inspected = vscode.workspace.getConfiguration("clorraine").inspect<unknown>(key);
  // Only undefined means absent. An explicitly malformed null must not select
  // a different executable through the default-install fallback.
  const value = inspected?.globalValue !== undefined ? inspected.globalValue :
    inspected?.defaultValue !== undefined ? inspected.defaultValue : "";
  if (typeof value !== "string") {
    throw new Error(`Set clorraine.${key} to a valid path string in User Settings.`);
  }
  return value;
}

/** True for an ordinary executable, rather than an existing directory or data file. */
function executable(candidate: string): boolean {
  try {
    if (!fs.statSync(candidate).isFile()) {
      return false;
    }
    fs.accessSync(candidate, fs.constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

/** Search absolute PATH directories without a shell or current-directory lookup. */
function onPath(name: string): string | undefined {
  for (const directory of (process.env.PATH ?? "").split(path.delimiter)) {
    if (!path.isAbsolute(directory)) {
      continue;
    }
    const candidate = path.join(directory, name);
    if (executable(candidate)) {
      return candidate;
    }
  }
  return undefined;
}

/** Validate an explicit choice without falling back to a different installation. */
function explicit(value: string, setting: string): string {
  const candidate = path.isAbsolute(value) ? value :
    path.basename(value) === value ? onPath(value) : undefined;
  if (!candidate || !executable(candidate)) {
    throw new Error(`Invalid ${setting}: ${value}. Set ${setting} to a valid executable in User Settings.`);
  }
  return candidate;
}

/**
 * Resolve a driver or LSP in the extension host process, never outside it.
 * Call only in a trusted workspace, before task launch or server startup.
 * @throws When an explicit choice is invalid or no executable is available.
 * @example resolveTool("clorraine")
 */
export function resolveTool(name: "clorraine" | "clorraine-lsp"): string {
  if (!vscode.workspace.isTrusted) {
    throw new Error("Grant workspace trust before discovering Clorraine tools.");
  }
  const key = name === "clorraine-lsp" ? "lsp.path" : "driver.path";
  const configured = machinePath(key);
  if (configured) {
    return explicit(configured, `clorraine.${key}`);
  }
  if (name === "clorraine-lsp" && process.env.CLORRAINE_LSP) {
    return explicit(process.env.CLORRAINE_LSP, "CLORRAINE_LSP");
  }
  const installed = path.join(homedir(), ".local", "bin", name);
  if (executable(installed)) {
    return installed;
  }
  for (const folder of vscode.workspace.workspaceFolders ?? []) {
    if (folder.uri.scheme !== "file") {
      continue;
    }
    for (const mode of ["debug", "release"]) {
      const candidate = path.join(folder.uri.fsPath, "target", mode, name);
      if (executable(candidate)) {
        return candidate;
      }
    }
  }
  const found = onPath(name);
  if (found) {
    return found;
  }
  throw new Error(`No ${name} found. Install Clorraine or set clorraine.${key} in User Settings.`);
}
