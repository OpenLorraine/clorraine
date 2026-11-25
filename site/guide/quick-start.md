---
title: Quick start
description: "Install the Clorraine v1.2.0 developer preview on Linux with one command, create your first Amiga project, and run it in FS-UAE."
---

# Quick start

> **Developer preview.** Clorraine v1.2.0 is a public developer preview for
> **Linux x86_64**. Expect changes. **Windows and macOS are on the way.**

## Install on Linux

One command downloads and runs the installer:

```sh
curl -fsSL https://github.com/OpenLorraine/clorraine/releases/download/v1.2.0/install.sh -o clorraine-install.sh && sh clorraine-install.sh
```

The installer walks you through five steps:

1. **Install the CLI.** It verifies the download and installs `clc`,
   `clorraine`, and `clorraine-lsp` under `~/.local` (or a folder you choose),
   and offers to add them to your `PATH`.
2. **Amiga toolchain.** It uses an existing AmigaPorts rc24 toolchain or
   downloads the pinned one (about 70 MB), verifies it, and sets it up.
3. **Amiga emulator.** It finds FS-UAE and your licensed Kickstart 3.1 A1200 ROM
   and remembers them, so later runs need no extra options.
4. **Say hello.** It asks before running anything, then runs the hello program
   on your PC and boots an Amiga 1200 in FS-UAE to play a short demo-scene
   intro.
5. **Your first Amiga project.** It installs the VS Code extension and offers to
   create a starter project in the current folder and open it in VS Code.

### What you need

| | Required | Notes |
|---|---|---|
| Linux x86_64 with glibc 2.38 or newer | Yes | Ubuntu 24.04 or newer, or an equivalent distribution |
| A C compiler (`cc`) | Yes | Used for the hello program on your PC |
| FS-UAE 3.2.0 or newer | For the Amiga | Ubuntu 24.04's 3.1.66 package is too old |
| A licensed Kickstart 3.1 A1200 ROM | For the Amiga | Clorraine never bundles or downloads it |
| VS Code | Optional | The installer adds the Clorraine extension |

### Verify the download first

To check the installer before running it, download it with the checksum file
from the [v1.2.0 release](https://github.com/OpenLorraine/clorraine/releases/tag/v1.2.0):

```sh
curl -fsSLO https://github.com/OpenLorraine/clorraine/releases/download/v1.2.0/install.sh
curl -fsSLO https://github.com/OpenLorraine/clorraine/releases/download/v1.2.0/SHA256SUMS
sha256sum -c SHA256SUMS --ignore-missing
sh install.sh
```

A literal `curl … | sh` is not supported; the installer asks questions on your
terminal.

### Scripted installs and removal

Without a terminal the installer asks nothing and uses its defaults. Options such
as `--prefix`, `--toolchain`, `--kickstart`, `--fs-uae`, `--no-hello`, and
`--starter DIR` are listed by:

```sh
sh clorraine-install.sh --help
```

To remove Clorraine, run `sh clorraine-install.sh --uninstall`; add `--purge` to
also remove the installed versions and the CLI configuration.

## Your first project

If you let the installer create a starter project, open it. Otherwise create
one now:

```sh
clorraine new my-game
cd my-game
code .
```

Open `main.clo` and press **Run in FS-UAE** in the editor title bar, or press
**Ctrl+Shift+B**. An Amiga 1200 boots in FS-UAE and your message bounces across
a copper sky. Change `MESSAGE` or the colours at the top of `main.clo`, save, and
run it again. Click the left mouse button in the Amiga window to quit.

From a terminal in the project folder:

```sh
clorraine run -p hosted-amiga --emulator main.clo
```

If you open a new terminal before `clorraine` is on your `PATH`, run
`. ~/.profile` first or use `~/.local/bin/clorraine`.

## Your first program

Create `hello.clo`:

```clorraine
module demo.Hello;

import { println } from std.io;

func main(): i32 {
    let who: str = "Amiga";
    println($"Hello, {who}, from Clorraine!");
    return 0;
}
```

Run it on your PC:

```sh
clorraine run hello.clo
```

Output:

```text
Hello, Amiga, from Clorraine!
```

Run the same program on the Amiga:

```sh
clorraine run -p hosted-amiga --emulator hello.clo
```

`clorraine` emits one C99 translation unit, compiles it, and runs the result.
Use `clorraine emit hello.clo` to read the generated C.

## Windows and macOS

The v1.2.0 developer preview supports Linux x86_64 only. Installers for Windows
and macOS are on the way.

## If something fails

- Read the [known limitations](/guide/known-limitations).
- Run `clorraine --help` for the exact command-line surface.
- Search or open an issue in
  [GitHub Issues](https://github.com/OpenLorraine/clorraine/issues).

Include the command, the profile, `clorraine --version`, and the installer
output when reporting a problem.
