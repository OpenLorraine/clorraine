/**
 * Editor messages that point the user at the install guide: the error shown
 * when a Clorraine tool cannot be found or used, and the notice shown when the
 * installed tools are older than the extension.
 */
import * as vscode from "vscode";

import { driftMessage, installGuideUrl } from "./guide";

/** The action label offered with both messages. */
export const OPEN_INSTALL_GUIDE = "Open install guide";

/**
 * A tool lookup failed: nothing was found, or an explicit setting does not
 * name a usable executable. Either way the remedy is to install the tools or
 * point the setting at them, so these errors offer the install guide.
 */
export class MissingToolError extends Error {}

/** Opens the install guide for this host in the user's browser. */
async function openInstallGuide(): Promise<void> {
  await vscode.env.openExternal(vscode.Uri.parse(installGuideUrl(process.platform)));
}

/**
 * Shows `text` as an error with an "Open install guide" action and opens the
 * guide when the user chooses it. Resolves once the message is dismissed.
 */
export async function showMissingTool(text: string): Promise<void> {
  if (await vscode.window.showErrorMessage(text, OPEN_INSTALL_GUIDE) === OPEN_INSTALL_GUIDE) {
    await openInstallGuide();
  }
}

/**
 * Shows the informational notice that the installed tools (`server`) are
 * older than the extension, with the same install-guide action.
 */
export async function showUpdateNotice(server: string, extension: string): Promise<void> {
  if (await vscode.window.showInformationMessage(driftMessage(server, extension), OPEN_INSTALL_GUIDE) ===
      OPEN_INSTALL_GUIDE) {
    await openInstallGuide();
  }
}
