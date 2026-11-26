# Windows CLI (developer preview)

> **Next release.** Windows support ships with the first Clorraine release after
> v1.2.0; the v1.2.0 release has Linux assets only.

Clorraine runs on **Windows 11 x64** as a developer preview. One
PowerShell script installs the CLI for your Windows account and, with your
approval, the Amiga toolchain, FS-UAE, the VS Code extension, and a starter
project. It needs no Docker, WSL, or Windows build environment.

## Install

Open **Windows PowerShell** (not as administrator) and run:

```powershell
curl.exe -fsSLo clorraine-install.ps1 https://github.com/OpenLorraine/clorraine/releases/latest/download/install.ps1; powershell -ExecutionPolicy Bypass -File .\clorraine-install.ps1
```

The installer walks through the same five steps as on Linux:

1. **Install the CLI.** It verifies the release zip against its pinned size
   and SHA-256, checks every entry before unpacking, and installs `clc.exe`,
   `clorraine.exe`, and `clorraine-lsp.exe` under
   `%LOCALAPPDATA%\Programs\Clorraine`. It offers to add the CLI to your user
   `PATH`.
2. **Amiga toolchain.** It uses an AmigaPorts rc24 toolchain that is already
   installed, or downloads the pinned upstream setup and installs it into
   `C:\Program Files\Clorraine`. That step asks for administrator approval
   once (a UAC prompt); declining it keeps the CLI installed.
3. **Amiga emulator.** It finds FS-UAE (or downloads the pinned FS-UAE 3.2.35
   zip when you agree) and a licensed Kickstart 3.1 A1200 ROM, then runs
   `clorraine setup` once to validate the toolchain and remember all three
   paths.
4. **Say hello.** It runs the hello program on the PC with GCC and plays the
   demo-scene hello on an emulated Amiga 1200 in FS-UAE.
5. **Your first Amiga project.** It installs the VS Code extension from the
   same release and offers a starter project in the current folder, which it
   can open in VS Code.

To check the installer before running it, download `install.ps1` and
`SHA256SUMS` from the
[latest release](https://github.com/OpenLorraine/clorraine/releases/latest)
and compare the hash:

```powershell
curl.exe -fsSLO https://github.com/OpenLorraine/clorraine/releases/latest/download/install.ps1
curl.exe -fsSLO https://github.com/OpenLorraine/clorraine/releases/latest/download/SHA256SUMS
(Get-FileHash -Algorithm SHA256 .\install.ps1).Hash.ToLower()
Select-String 'install.ps1' .\SHA256SUMS
```

## The execution policy and the UAC prompt

Windows blocks unsigned scripts by default. The install command bypasses the
execution policy for that one `powershell` process only. Inside a PowerShell
window you can do the same for the current window with:

```powershell
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
```

Do not change the execution policy permanently; Clorraine never asks you to.

- If the script was downloaded with a browser, Windows marks it as coming from
  the internet. Run `Unblock-File .\clorraine-install.ps1` first, or use the
  `curl.exe` command above.
- If your organization sets the execution policy through Group Policy, the
  bypass has no effect and PowerShell reports that running scripts is
  disabled. Ask your administrator; the installer cannot work around it.
- The only administrator step is the rc24 toolchain setup. The UAC prompt
  names Windows PowerShell; it copies the verified setup into a folder only
  administrators can change, checks it again there, and runs it silently.
  Everything else runs as you.

## Requirements

| | Required | Notes |
|---|---|---|
| Windows 11 x64 | Yes | Windows 10 prints "untested" and continues; ARM64 and 32-bit Windows are refused |
| Windows PowerShell 5.1 (64-bit) | Yes | Part of Windows; PowerShell 7 is untested |
| GCC | For the PC hello | [w64devkit](https://github.com/skeeto/w64devkit) or MSYS2 UCRT64 on `PATH`, or `CLORRAINE_CC`; Clang cannot build debug programs yet |
| FS-UAE 3.2.0+ | For Amiga runs | The installer can download the pinned 3.2.35 zip into its own folder |
| A licensed 512 KB Kickstart 3.1 A1200 ROM | For Amiga runs | Never bundled or downloaded; KS3.0 is untested |
| VS Code | Optional | The extension is installed when VS Code is detected |

The toolchain pin is AmigaPorts `m68k-amigaos-gcc` v16.2-rc24
(`m68k-amigaos-gcc-16.2-rc24-x86_64-w64-mingw32ucrt-setup.exe`, 48,065,003
bytes, SHA-256
`9dd6973539bb2d514fc1b1e6c0a107cac6f30a9c4076cf4014cbe13013c95512`). The
FS-UAE pin is `FS-UAE_3.2.35_Windows_x86-64.zip` (49,123,213 bytes, SHA-256
`b402e2802d518a085c878be96c5c0a656fa3dec57dcf0019d226f3285069063e`). Both are
downloaded from their upstream releases; Clorraine does not mirror or
redistribute them. `setup` rejects other toolchain versions.

## What goes where

| Path | Contents |
|---|---|
| `%LOCALAPPDATA%\Programs\Clorraine\current\bin\` | `clc.exe`, `clorraine.exe`, `clorraine-lsp.exe`; this folder is the `PATH` entry |
| `%LOCALAPPDATA%\Programs\Clorraine\versions\` | Installed versions; `current` is a junction to the active one |
| `%LOCALAPPDATA%\Programs\Clorraine\examples\` | `hello.clo` and the Amiga intro `hello-amiga.clo` |
| `%LOCALAPPDATA%\Programs\Clorraine\fs-uae\3.2.35\` | FS-UAE, when the installer downloaded it |
| `C:\Program Files\Clorraine\amiga-gcc-16.2-rc24\` | The rc24 toolchain, installed for all users; `rc24-setup.log` sits beside it |
| `%APPDATA%\clorraine\config` | The remembered toolchain, Kickstart, and FS-UAE paths |

The installer looks for a ROM in `%USERPROFILE%\amiga\roms`, your
`Documents\FS-UAE\Kickstarts`, and the Amiga Forever shared folder
`%PUBLIC%\Documents\Amiga Files\Shared\rom`, and for FS-UAE on `PATH`, under
its own folder, and in `C:\Program Files\FS-UAE`.

## Everyday commands

```powershell
clorraine new my-game                              # a starter project with VS Code tasks
clorraine run hello.clo                            # build and run on the PC
clorraine run -p hosted-amiga --emulator hello.clo # build and run on the emulated A1200
clorraine build -p hosted-amiga hello.clo          # build an Amiga executable
```

Open a new terminal after the install so the `PATH` entry applies. Without the
entry, run `& "$env:LOCALAPPDATA\Programs\Clorraine\current\bin\clorraine.exe"`.
`clorraine new [DIR]` behaves as on Linux: it never writes into a folder that
is not empty, and the default folder is `amiga-starter`.

## Configuration

`setup` validates and stores the toolchain, and optionally the emulator:

```powershell
clorraine setup --toolchain "C:\Program Files\Clorraine\amiga-gcc-16.2-rc24" `
  --kickstart "$env:USERPROFILE\amiga\roms\amiga-os-310-a1200.rom" `
  --fs-uae "$env:LOCALAPPDATA\Programs\Clorraine\fs-uae\3.2.35\FS-UAE\Windows\x86-64\fs-uae.exe"
```

The resolution order is the same as on Linux (`--toolchain`,
`CLORRAINE_TOOLCHAIN`, the config; likewise for the Kickstart and FS-UAE, with
`PATH` last for FS-UAE). The config file is always
`%APPDATA%\clorraine\config`; `HOME` and `XDG_CONFIG_HOME` are ignored on
Windows, so Git Bash and MSYS2 shells share it. `CLORRAINE_CC` names the C
compiler for PC builds; without it the CLI uses the first of `gcc.exe`,
`cc.exe`, and `clang.exe` on `PATH`.

## Emulator runs

`run --emulator` works as on Linux: it boots an Amiga 1200 with Kickstart 3.1
in FS-UAE from a private folder under `%TEMP%`, ends a run after 90 seconds
(`CLORRAINE_EMULATOR_TIMEOUT_SECONDS`, 1–3600), and keeps the private folder
of a failed or timed-out run. The FS-UAE window opens on your desktop.

## VS Code

The installer installs the `openlorraine.clorraine` extension from the release
when VS Code (`code`, `codium`, or `code-insiders`) is found. The extension
finds `clorraine.exe` and `clorraine-lsp.exe` in
`%LOCALAPPDATA%\Programs\Clorraine\current\bin`, so it works without the
`PATH` entry. Diagnostics, navigation, and the **Run in FS-UAE** and **Run**
buttons behave as on Linux.

## Scripted installs and removal

Without a console the installer asks nothing, keeps its defaults, and never
asks for administrator approval or downloads FS-UAE unless told to. The
options are listed by `.\clorraine-install.ps1 -Help`, among them `-Prefix`,
`-ModifyPath`, `-Toolchain`, `-NoToolchain`, `-InstallToolchain`,
`-ReplaceToolchain`, `-FsUae`, `-Kickstart`, `-InstallFsUae`, `-NoEmulator`,
`-NoHello`, `-Starter DIR`, `-NoStarter`, `-VSCode`, and `-NoVSCode`.

`.\clorraine-install.ps1 -Uninstall` removes the `current` link and the `PATH`
entry; `-Uninstall -Purge` also removes the installed versions, the examples,
the FS-UAE it downloaded, and the CLI config. Neither touches files you put
in the install folder. The rc24 toolchain is removed under **Settings > Apps >
Installed apps** (`amiga-gcc`).

## Limits

- Windows 11 x64 only; Windows 10 and PowerShell 7 are untested.
- Clang cannot build debug programs for the PC yet; use GCC.
- The CLI cannot start the rc24 compiler from an SSH logon; use a terminal in
  your signed-in desktop session.
- Stopping `run --emulator` with Ctrl+C leaves the FS-UAE window open and its
  `%TEMP%\clorraine-emulator-*` folder behind; close the window and delete the
  folder.
- Network-share (UNC) workspaces, paths longer than 260 characters, and a
  `%TEMP%` path with a comma are not supported.
- The binaries and scripts are not code-signed yet, so SmartScreen or
  Defender may ask before the first run.
- The VS Code extension is not in a marketplace yet.
- The full list is in [known limitations](known-limitations.md).

## Related

- [Linux CLI](linux-cli.md)
- [Known limitations](known-limitations.md)
- [Releases](https://github.com/OpenLorraine/clorraine/releases)
