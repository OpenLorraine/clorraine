# Clorraine for VS Code

Language support for [Clorraine](https://openlorraine.github.io/clorraine/), a
systems programming language for Amiga-compatible computers:

- compiler diagnostics as you type, from the Clorraine language server;
- hover, go to definition, completion, and document symbols;
- syntax highlighting for `.clo` files;
- **Build**, **Run**, and **Run in FS-UAE** for the active program, from the
  editor title bar or the Command Palette.

Linux x86_64 and Windows 11 x64; macOS is not supported yet.

## Requirements

The extension needs the Clorraine command-line tools; it does not contain a
compiler. Install them with the one-line installer:

- [Install on Linux](https://openlorraine.github.io/clorraine/guide/quick-start#install-on-linux)
- [Install on Windows](https://openlorraine.github.io/clorraine/guide/quick-start#install-on-windows)

The installer also offers to install this extension and to create a starter
project whose build task runs your program in FS-UAE (`Ctrl+Shift+B`).
Running Amiga programs in FS-UAE needs FS-UAE and your own licensed Kickstart
ROM; the installer explains both.

If the tools are missing, the extension says so and offers **Open install
guide**. If the installed tools are older than the extension, it shows one
notice per session; rerun the installer to update them.

## Commands

| Command | Result |
|---|---|
| **Clorraine: Build** | Compile the active entry file and the saved files it imports |
| **Clorraine: Run** | Build and run it: natively for `host`, or through `vamos` for an Amiga profile |
| **Clorraine: Run in FS-UAE** | Build an Amiga program and run it in FS-UAE |

The extension saves the active entry before building. Save any other changed
files it imports yourself.

## Settings

Path settings are machine settings: set them in **User Settings**. Leave them
empty to use the installed tools.

| Setting | Use |
|---|---|
| `clorraine.profile` | Compiler profile for diagnostics and actions: `hosted-amiga` (default), `hosted-amiga-13`, `host`, or `freestanding` |
| `clorraine.lsp.path` | Language server executable |
| `clorraine.driver.path` | Clorraine driver executable |
| `clorraine.toolchain.path` | Amiga toolchain root containing `bin/m68k-amigaos-gcc` |
| `clorraine.fsUae.path` | FS-UAE executable for Run in FS-UAE |
| `clorraine.kickstart.path` | Your licensed Kickstart ROM for Run in FS-UAE |

An invalid path setting is reported as an error; the extension never falls
back to a different installation.

## Restricted Mode

In an untrusted workspace the extension does not start the language server,
look for tools, save files, or run tasks. Grant workspace trust only for
source folders you trust.

## Links

- [Documentation](https://openlorraine.github.io/clorraine/)
- [Known limitations](https://openlorraine.github.io/clorraine/guide/known-limitations)
- [Report a problem](https://github.com/OpenLorraine/clorraine/issues)
- [#clorraine on the AmigaDev Discord](https://discord.gg/7ckPjharmD)

## License

Clorraine is licensed under the Apache License 2.0 with the Clorraine Output
Exception; the package includes the license, the exception, the notice, and
the trademark policy.

Clorraine is an independent project. Amiga, AmigaOS, Kickstart, and Workbench
are trademarks of Amiga Corporation. Not affiliated with or endorsed by Amiga
Corporation or Hyperion Entertainment.
