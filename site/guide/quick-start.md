---
title: Quick start
description: "Install Clorraine on Linux with the v1.2.0 candidate installer, or build from source or use the pinned Amiga SDK in Docker."
---

# Quick start

Clorraine is a developer preview. The **Linux x86_64** path installs prebuilt
compiler binaries, the Amiga SDK and the `vamos` runtime without compiling
dependencies. One-click installers for Windows and macOS are in progress and
are not shown here as if they existed.

## Choose a path

| Option | What it gives you | Requirements |
|---|---|---|
| **Linux x86_64 installer** | `clc`, `clorraine`, `clorraine-lsp`, Amiga SDK and `vamos` | Ubuntu 22.04/24.04 x86_64; the system prerequisites below |
| **Docker SDK** | Full m68k toolchain, `vamos`, FS-UAE, NDK headers | Docker; network; a 30–90 minute first image build |
| **Build from source** | All three binaries, including the language server | Rust stable; no external crates |
| **One-click installers** | Planned for Linux, macOS, and Windows | Not available yet |

## Linux installer

The v1.2.0 installer is the current release target. Its artifacts remain
candidates until the explicit approval recorded in
[RELEASE.md](https://github.com/OpenLorraine/clorraine/blob/main/RELEASE.md) is
supplied. Once the distribution is published, install with:

```sh
curl -LO https://github.com/OpenLorraine/clorraine/releases/download/v1.2.0/install.sh
sh install.sh --release v1.2.0
export PATH="$HOME/.clorraine/bin:$PATH"
clorraine --version
```

The default root is `~/.clorraine`. Use `--root <path>` to select another
user-owned directory. The installer verifies the helper length and SHA-256,
checks the host distribution and Python ABI, builds a complete candidate, and
activates it only after the installed tools pass their smoke checks. A failed
update leaves the previous working version active.

System prerequisites:

```sh
sudo apt install gcc libc6-dev python3 python3-venv curl ca-certificates
```

The installer prints the exact `code --install-extension ...` command for the
packaged VSIX. Run that printed command; the VSIX lives beside the installed
version, not at a fixed path guessed from the root. It does not edit shell
startup files or install the extension automatically. FS-UAE and licensed
Kickstart/Workbench media remain user-provided; the SDK never downloads or
copies them.

The previous published release, v1.1.2, remains available under its MIT terms
at <https://github.com/OpenLorraine/clorraine/releases/tag/v1.1.2>.

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

Run it with the host profile:

```sh
clorraine run hello.clo
```

Output:

```text
Hello, Amiga, from Clorraine!
```

`clorraine` emits one C99 translation unit, compiles it with the host C
compiler, and runs the result. Use `clorraine emit hello.clo -p host` to read the
generated C.

## VS Code

The installer places `clorraine.vsix` beside the installed version and prints
the exact installation command. Copy and run that printed command, for example:

```sh
# Use the exact path printed by the installer, not a guessed root path:
code --install-extension /path/printed/by/installer/clorraine.vsix
```

Do not assume a fixed path under the root; use the path the installer printed.

The extension adds **Clorraine: Build**, **Clorraine: Run** and **Clorraine: Run
in FS-UAE**, and discovers the installed driver and language server without a
terminal `PATH`. For Remote-WSL, install the VSIX in the Linux remote host; the
extension README documents moving a generated Amiga executable to a Windows
FS-UAE installation. Automatic Windows emulator launch is not supported.

## Amiga SDK in Docker

The SDK image is the reproducible Amiga environment. It builds GCC, binutils,
libnix, the NDK headers, vbcc, `vamos`, and FS-UAE support from pinned upstream
sources. The first build compiles the toolchain and takes roughly 30–90 minutes.

```sh
git clone https://github.com/OpenLorraine/clorraine.git
cd clorraine
make setup FETCH=1
```

`make setup FETCH=1` detects the host tools, builds the pinned image if it is
missing, and re-checks the result. To see the plan without changing anything:

```sh
make setup DRY=1
```

Then run a program through the driver inside the SDK:

```sh
make run FILE=examples/hello/hello.clo
make run FILE=examples/hello/hello.clo PROFILE=hosted-amiga
```

The second command builds for `hosted-amiga` and runs the program under `vamos`.
No Kickstart ROM is needed for a console program on that path.

For a graphics demo, boot FS-UAE on the host:

```sh
make run FILE=examples/amiga/boing.clo PROFILE=hosted-amiga-13 EMULATOR=1
```

The A500 path needs the licensed Kickstart 1.3 ROM in `~/amiga/roms`. If you only
want to try the toolchain, the `vamos` path above needs no media at all.

## What you need for the Amiga path

| Requirement | Provided by | Notes |
|---|---|---|
| `m68k-amigaos-gcc` and NDK headers | Docker SDK | Built from pinned public sources |
| `vamos` | Docker SDK | Runs CLI binaries on the host without a ROM |
| FS-UAE | Docker SDK and host | Graphics and audio need an X display |
| Kickstart ROM | You | Only for FS-UAE with original media; AROS is the default |
| Workbench media | You | Only for the advanced Workbench gates |

The SDK never downloads or copies licensed media. See the
[media and NDK policy](/project/licensing#media-and-the-ndk).

## Build from source

The Rust workspace has one member and no external crates:

```sh
git clone https://github.com/OpenLorraine/clorraine.git
cd clorraine
cargo build --workspace
```

The binaries are written to `target/debug/`:

| Binary | Purpose |
|---|---|
| `clc` | Compile `.clo` sources to one C99 file |
| `clorraine` | Build, run, emit, document, and explain diagnostics |
| `clorraine-lsp` | Stdio language server for editors |

Run the fast verification gate from the repository root:

```sh
make check
```

`make check` runs the spec check, the Rust test suites, the SDK installer tests,
the benchmark harness tests, and the fuzz smoke matrix in the pinned container.
The full Amiga acceptance gate is `make verify`; it needs the SDK, an X display,
and the licensed media.

## If something fails

- Read the [known limitations](/guide/known-limitations).
- Search or open an issue in
  [GitHub Issues](https://github.com/OpenLorraine/clorraine/issues).
- Run `clorraine --help` for the exact command-line surface.
- For Docker, X11, ROM, and emulator problems, see
  [docker/README.md](https://github.com/OpenLorraine/clorraine/blob/main/docker/README.md).

Include the command, the profile, the compiler version (`clorraine --version`),
and the generated C when you report a code-generation problem. A small,
complete source file is the most useful reproduction.
