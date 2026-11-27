/**
 * Install-guide addresses and the language-server version comparison. Nothing
 * here imports the editor API, so the rules run in a plain Node test.
 */

/** The site's quick start, which holds one install section per platform. */
const QUICK_START = "https://openlorraine.github.io/clorraine/guide/quick-start";

/** A plain `major.minor.patch` version; anything else is not compared. */
const PLAIN_VERSION = /^(\d+)\.(\d+)\.(\d+)$/;

/**
 * The install guide for the host the extension runs on: the Linux or Windows
 * section of the quick start, or the quick start itself on other hosts, where
 * the page explains what is available.
 * @example installGuideUrl("win32") === "https://openlorraine.github.io/clorraine/guide/quick-start#install-on-windows"
 */
export function installGuideUrl(platform: NodeJS.Platform): string {
  switch (platform) {
    case "linux":
      return `${QUICK_START}#install-on-linux`;
    case "win32":
      return `${QUICK_START}#install-on-windows`;
    default:
      return QUICK_START;
  }
}

/**
 * True when the language server reports a plain version that is numerically
 * older than the extension's. A missing or non-plain server version (for
 * example a development build) is never reported, and neither is a newer
 * server: the extension keeps working with older servers through capability
 * checks, so this only decides whether to suggest an update.
 * @example serverOlderThan("1.9.0", "1.10.0") === true
 */
export function serverOlderThan(server: string | undefined, extension: string): boolean {
  const ours = PLAIN_VERSION.exec(extension);
  const theirs = server === undefined ? null : PLAIN_VERSION.exec(server);
  if (!ours || !theirs) {
    return false;
  }
  for (let part = 1; part <= 3; part++) {
    const difference = Number(theirs[part]) - Number(ours[part]);
    if (difference !== 0) {
      return difference < 0;
    }
  }
  return false;
}

/**
 * The notice shown when the installed tools are older than the extension.
 * @example driftMessage("1.2.0", "1.3.0")
 */
export function driftMessage(server: string, extension: string): string {
  return `Clorraine CLI ${server} is older than the extension ${extension}. Run the installer to update.`;
}
