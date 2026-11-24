---
title: Quick start
description: "Build Clorraine from source, select an upstream Amiga toolchain, or use the pinned Amiga SDK in Docker."
---

# Quick start

> **No prebuilt release is currently published.** The Linux CLI candidate now
> includes toolchain setup, native builds, the verified FS-UAE runner, a local
> installer/archive, and a locally built VSIX. Nothing is tagged or uploaded.

Clorraine is a developer preview. v1.2.0 targets **Linux x86_64** only. WSL2,
Remote-WSL, Windows, and macOS are outside this release.

## Choose a path

| Option | What it gives you | Requirements |
|---|---|---|
| **Linux CLI candidate** | `clc`, `clorraine`, `clorraine-lsp`; toolchain setup and native builds | Linux x86_64, glibc 2.38+, Rust to build from source, the upstream rc24 toolchain |
| **Docker SDK** | Full m68k toolchain, `vamos`, FS-UAE, NDK headers, Amiga gates | Docker; network; a 30–90 minute first image build |
| **Verified FS-UAE runner** | A1200/KS3.1 guest run with rc/status evidence | Native toolchain, FS-UAE 3.2.0+, licensed Kickstart 3.1 A1200 ROM |
| **VSIX** | The VS Code client | Built locally with `make vscode-package`; not published |

No one-click installer or marketplace extension is published.

## Linux CLI candidate

The candidate uses a user-downloaded upstream toolchain. Clorraine validates it
and never bundles or mirrors it.

### 1. Build the CLI

```sh
git clone https://github.com/OpenLorraine/clorraine.git
cd clorraine
cargo build --workspace
```

The binaries are written to `target/debug/`. Building from source needs Rust
stable; the Docker SDK remains the development environment.

### 2. Download and select the toolchain

Use AmigaPorts `m68k-amigaos-gcc` v16.2-rc24:

| Field | Value |
|---|---|
| Asset | `m68k-amigaos-gcc-16.2-rc24-x86_64-linux.tar.xz` |
| Bytes | `69,868,548` |
| SHA-256 | `b93b070ac47d2f01d126f57c03a8405f71fb470c8397b0bda51041217d25c100` |

Extract it and select the **root** containing `bin/m68k-amigaos-gcc` and
`m68k-amigaos/`:

```sh
./target/debug/clorraine setup --toolchain "$HOME/toolchains/m68k-amigaos-gcc-16.2"
```

`setup` checks the layout, the `16.2-rc24` version, the NDK headers, and both
`nix20` and `nix13` link probes. It writes the canonical path only after every
check passes. The config is `$XDG_CONFIG_HOME/clorraine/config` when absolute,
otherwise `~/.config/clorraine/config`.

Selection order is `--toolchain`, `CLORRAINE_TOOLCHAIN`, the stored config, then
the marker-gated `PATH` fallback inside the pinned SDK container only.
`--sdk-root` and `CLORRAINE_SDK` are removed.

### 3. Build the local candidate (optional)

`make dist` builds the release binaries and VSIX and writes the candidate
archive, installer, and checksums under `build/dist`. Install the exact local
bytes with:

```sh
build/dist/install.sh --archive build/dist/clorraine-1.2.0-x86_64-linux.tar.gz \
  --checksums build/dist/SHA256SUMS
```

The default prefix is `~/.local`; add `--modify-path` to write the marked
`PATH` block to `~/.profile`. `make dist-verify` checks the candidate, and
`make dist-check` runs the hermetic installer tests.

### 4. Build a program

```sh
./target/debug/clorraine build -p hosted-amiga examples/hello/hello.clo
./target/debug/clorraine build -p freestanding \
  spec/tests/corpus/freestanding/program.clo
```

The portable toolchain check is:

```sh
make toolchain-check TOOLCHAIN="$HOME/toolchains/m68k-amigaos-gcc-16.2"
```

### 5. FS-UAE

With a native toolchain selected, `run --emulator` boots an A1200 with your
licensed 512 KB Kickstart 3.1 A1200 ROM. A compiled wrapper records the guest
return code and writes a status marker only for zero; the CLI accepts the run
only for rc 0 plus status `ok`, reads the output, and removes the private run.
Failed or timed-out runs print their output and retain the directory; only a
successful guest is captured with `--capture`.

```sh
./target/debug/clorraine run -p hosted-amiga --emulator \
  --kickstart "$HOME/amiga/roms/amiga-os-310-a1200.rom" \
  examples/hello/hello.clo
```

FS-UAE 3.2.0 or newer is required; Ubuntu 24.04's 3.1.66 package is too old.
AROS is manual-only and is not an acceptance path. `make fs-uae-check` runs the
real guest matrix locally; the A500/Kickstart 1.3 gates remain in the Docker
development environment.

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
./target/debug/clorraine run hello.clo
```

Output:

```text
Hello, Amiga, from Clorraine!
```

`clorraine` emits one C99 translation unit, compiles it with the host C
compiler, and runs the result. Use `clorraine emit hello.clo -p host` to read the
generated C.

## VS Code

The extension is a separate local artifact; the installer does not place it and
no marketplace release is published.

```sh
make vscode-package
code --install-extension build/vscode/clorraine-1.2.0.vsix
```

The extension adds **Clorraine: Build**, **Clorraine: Run**, and **Clorraine: Run
in FS-UAE**. It discovers `~/.local/bin` and uses `clorraine.toolchain.path` for
the upstream root; set `clorraine.driver.path` if a custom installation is not
discovered. The removed `clorraine.sdk.path` is read once with a warning.
v1.2.0 supports the Linux desktop only.

## Amiga SDK in Docker

The SDK image is the reproducible Amiga development environment. It builds GCC,
binutils, libnix, the NDK headers, vbcc, `vamos`, and FS-UAE support from pinned
upstream sources. The first build takes roughly 30–90 minutes.

```sh
make setup FETCH=1
make run FILE=examples/hello/hello.clo
make run FILE=examples/hello/hello.clo PROFILE=hosted-amiga
```

The second command builds for `hosted-amiga` and runs under `vamos`; no Kickstart
ROM is needed for that console path. For graphics:

```sh
make run FILE=examples/amiga/boing.clo PROFILE=hosted-amiga-13 EMULATOR=1
```

The A500 path needs the licensed Kickstart 1.3 ROM in `~/amiga/roms`.

## What you need for the Amiga path

| Requirement | Provided by | Notes |
|---|---|---|
| `m68k-amigaos-gcc` and NDK headers | You, or the Docker SDK | rc24 for the native CLI; built from pinned sources in Docker |
| `vamos` | Docker SDK or your `PATH` | Runs CLI binaries without a ROM |
| FS-UAE | You, or the Docker SDK | 3.2.0+ for the native path; an X display |
| Kickstart 3.1 A1200 ROM | You | Required for the verified Linux emulator path; KS3.0 is untested |
| Kickstart 1.3 / Workbench media | You | Only for the A500 and advanced Docker gates |

The SDK never downloads or copies licensed media. See the
[media and NDK policy](/project/licensing#media-and-the-ndk).

## Build from source

```sh
cargo build --workspace
make check
```

`make check` runs the spec check, workspace tests, installer tests, benchmark
harness tests, and the fuzz smoke matrix in the pinned container. The full Amiga
acceptance gate is `make verify`; it needs the SDK, an X display, and the
licensed media.

## If something fails

- Read the [known limitations](/guide/known-limitations).
- Search or open an issue in
  [GitHub Issues](https://github.com/OpenLorraine/clorraine/issues).
- Run `clorraine --help` for the exact command-line surface.
- For Docker, X11, ROM, and emulator problems, see
  [docker/README.md](https://github.com/OpenLorraine/clorraine/blob/main/docker/README.md).

Include the command, profile, compiler version (`clorraine --version`), and the
generated C when reporting a code-generation problem.
