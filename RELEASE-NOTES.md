# Clorraine v1.3.0 developer preview

This is a public **developer preview** for **Linux x86_64** and **Windows 11
x64**. It is not the latest stable release; `v1.1.2` remains Latest.

## Install

Linux:

```sh
curl -fsSL https://github.com/OpenLorraine/clorraine/releases/download/v1.3.0/install.sh -o clorraine-install.sh && sh clorraine-install.sh
```

Windows, in Windows PowerShell (not as administrator):

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/OpenLorraine/clorraine/releases/download/v1.3.0/install.ps1 | iex"
```

Both installers take you from nothing to your own Amiga program in five steps:
the CLI, the rc24 Amiga toolchain, FS-UAE with your licensed Kickstart 3.1
ROM, a hello on the PC and on an emulated Amiga 1200, and a starter project
with the VS Code extension. On Windows, installing rc24 asks for administrator
approval once. Clorraine never bundles or mirrors the toolchain or the ROM.

## What's new

- **Windows 11 x64.** The CLI, the language server, and a PowerShell
  installer with the same steps as on Linux. See the
  [Windows CLI guide](https://github.com/OpenLorraine/clorraine/blob/v1.3.0/docs/windows-cli.md).
- **The VS Code extension is on the Visual Studio Marketplace** as
  [`openlorraine.clorraine`](https://marketplace.visualstudio.com/items?itemName=openlorraine.clorraine).
  The installers install it from there and VS Code keeps it up to date.
- **Portable starter projects.** The starter's build tasks ask the extension
  for the driver, so a project works on any machine with Clorraine installed.
- **Clearer editor help.** Missing tools offer an "Open install guide" action,
  and the extension tells you when the installed CLI is older than itself.
- **Amiga fixes.** `savePublicScreen` and `showAlert` no longer crash before a
  screen is open (#59), a clipboard write after a read no longer hangs (#60),
  clip regions and AmigaGuide messages can no longer outlive their owner (#65),
  and `drawText` clips at the screen edge (#57).

## Upgrading from v1.2.0

Run the installer again. It installs v1.3.0 beside v1.2.0 and switches to it,
and it reinstalls the VS Code extension from the Marketplace: the v1.2.0
installer installed it from a file, which VS Code never updates.

## Known limitations

The extension is on the Visual Studio Marketplace only, so VSCodium and other
Open VSX editors cannot install it yet. macOS is on the way. The full list is
in [known limitations](https://openlorraine.github.io/clorraine/guide/known-limitations).

## Assets

PENDING-CHECKSUMS: the release steps replace this line with the asset table
from `build/dist/SHA256SUMS` before publication.
