<p align="center">
  <img src="site/public/logo.svg" alt="Clorraine" width="72" height="72">
</p>

# Clorraine

> **Developer preview.** Clorraine is under active development. The v1.2.0
> Linux build is a public prerelease; expect changes. Windows support ships
> with the next release; macOS is on the way.

Clorraine is a safe, C-like language for the Amiga and LorraineOS. It has
Rust-style ownership and borrow checking, compiles to plain C, and runs on the
existing Amiga toolchains. The language and LorraineOS are designed together.

## Install on Linux

Linux x86_64, glibc 2.38 or newer. One command:

```sh
curl -fsSL https://github.com/OpenLorraine/clorraine/releases/download/v1.2.0/install.sh -o clorraine-install.sh && sh clorraine-install.sh
```

The installer walks you through five steps:

1. installs `clc`, `clorraine`, and `clorraine-lsp` (default `~/.local`) and
   offers to add them to your PATH;
2. sets up the rc24 Amiga toolchain, downloaded from AmigaPorts or an existing
   copy;
3. finds FS-UAE and your licensed Kickstart ROM and remembers them;
4. runs the hello program, then boots an Amiga 1200 in FS-UAE and plays a
   demo-scene intro: copper gradients, raster bars, and a chrome logo;
5. installs the VS Code extension and creates a starter Amiga project in the
   current folder that runs in FS-UAE with one click.

Make another project at any time:

```sh
clorraine new my-game
cd my-game && code .
```

Open `main.clo`, change the message or the colours, and press **Run in FS-UAE**
in the editor title bar. You supply the licensed Kickstart ROM; Clorraine never
bundles or mirrors it. The
[release page](https://github.com/OpenLorraine/clorraine/releases/tag/v1.2.0)
has the details, including checksum verification.

## Install on Windows

Windows 11 x64, from the next release on (`vX.Y.Z` below is that release). In
Windows PowerShell:

```powershell
curl.exe -fsSLo clorraine-install.ps1 https://github.com/OpenLorraine/clorraine/releases/download/vX.Y.Z/install.ps1; powershell -ExecutionPolicy Bypass -File .\clorraine-install.ps1
```

The same five steps install the CLI for your account; only the rc24
toolchain setup asks for administrator approval. See the
[Windows CLI guide](docs/windows-cli.md). macOS is on the way.

## See what it can do

| Boing: Neon Horizon | First Light |
|:---:|:---:|
| ![A chrome ball above a neon sunset grid](site/public/screenshots/boing.webp) | ![A lighthouse and the sun over a dithered sea](site/public/screenshots/first-light.webp) |
| **A500 · 32 colours · play in the browser** | **A500 · 16 colours · play in the browser** |
| **Prism** | **Clorraine Studio** |
| ![A brick grid, paddle, and ball in an arcade frame](site/public/screenshots/prism.webp) | ![The Clorraine Studio window on a Workbench 3.1 screen](site/public/screenshots/gui31-showcase.png) |
| **A500 · arcade · keyboard or joystick** | **A1200 · Kickstart 3.1 · native Workbench** |

[Play the A500 demos in your browser →](https://openlorraine.github.io/clorraine/examples/)

## Learn more

Everything below goes deeper than this page:

- [Documentation](https://openlorraine.github.io/clorraine/)
- [Linux CLI guide](docs/linux-cli.md)
- [Windows CLI guide](docs/windows-cli.md)
- [Known limitations](docs/known-limitations.md)
- [Language specification](spec/README.md)
- [C to Clorraine guide](docs/c-to-clorraine.md)
- [Examples](examples/)
- [All releases](https://github.com/OpenLorraine/clorraine/releases)
- [#clorraine on the AmigaDev Discord](https://discord.gg/7ckPjharmD)

## License

Clorraine is open source under the [Apache License 2.0](LICENSE), with the
[Clorraine Output Exception](LICENSE-EXCEPTION.md).

Built by [OpenLorraine](https://github.com/OpenLorraine). LorraineOS is a
separate project.
