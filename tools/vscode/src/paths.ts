/**
 * Host path rules shared by discovery and the Build/Run actions. Nothing here
 * imports the editor API, so the rules run in a plain Node test on any host.
 */
import * as path from "node:path";

/**
 * The spelling two paths are compared in. Windows file names ignore case and
 * accept either separator, and the language server, `fs.realpathSync`, and
 * the editor can differ in the drive letter's case, so a Windows path is
 * normalized and lowercased. Other hosts compare the exact bytes.
 * @example comparablePath("C:/A/b.clo", "win32") === "c:\\a\\b.clo"
 */
export function comparablePath(value: string, platform: NodeJS.Platform = process.platform): string {
  return platform === "win32" ? path.win32.normalize(value).toLowerCase() : value;
}

/**
 * True when `a` and `b` name the same file under the host's path rules.
 * @example samePath("c:\\A\\b.clo", "C:\\a\\B.clo", "win32") === true
 */
export function samePath(a: string, b: string, platform: NodeJS.Platform = process.platform): boolean {
  return comparablePath(a, platform) === comparablePath(b, platform);
}

/**
 * The file name of the executable called `name`: `name.exe` on Windows unless
 * it already ends in `.exe`, otherwise `name`.
 * @example executableName("clorraine", "win32") === "clorraine.exe"
 */
export function executableName(name: string, platform: NodeJS.Platform = process.platform): string {
  return platform === "win32" && !name.toLowerCase().endsWith(".exe") ? `${name}.exe` : name;
}

/**
 * Where the Clorraine installer puts a tool: `~/.local/bin/<name>` on Linux,
 * `%LOCALAPPDATA%\Programs\Clorraine\current\bin\<name>.exe` on Windows (the
 * `current` link follows upgrades). A missing or relative `LOCALAPPDATA` falls
 * back to `<home>\AppData\Local`, the Windows default.
 * @example installedToolPath("clorraine", "linux", {}, "/home/u") === "/home/u/.local/bin/clorraine"
 */
export function installedToolPath(
  name: string,
  platform: NodeJS.Platform,
  env: NodeJS.ProcessEnv,
  home: string,
): string {
  if (platform === "win32") {
    const local = env.LOCALAPPDATA && path.win32.isAbsolute(env.LOCALAPPDATA) ?
      env.LOCALAPPDATA : path.win32.join(home, "AppData", "Local");
    return path.win32.join(local, "Programs", "Clorraine", "current", "bin", executableName(name, platform));
  }
  return path.posix.join(home, ".local", "bin", name);
}
