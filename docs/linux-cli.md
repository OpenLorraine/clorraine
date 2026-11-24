# Linux CLI (v1.2.0 candidate)

Clorraine's Linux CLI is the product path for writing, building, and running
Clorraine programs without Docker. The v1.2.0 candidate is still in progress:

| Area | State |
|---|---|
| `clorraine setup --toolchain` and config | Implemented in the candidate |
| `--toolchain` selection and builds | Implemented in the candidate |
| Linux installer and VSIX publication | Not published; installer is a later phase |
| Verified FS-UAE runner | Next phase; native emulator selection is currently unavailable |
| macOS, Windows, WSL | Outside v1.2.0 |

No release is published from the candidate branch. Build from source for now.

## Host requirements

- Linux x86_64 with glibc 2.38 or newer (Ubuntu 24.04 is the minimum baseline).
- The upstream AmigaPorts toolchain:
  `m68k-amigaos-gcc` v16.2-rc24.
- FS-UAE 3.2.0 or newer for the FS-UAE path. Ubuntu 24.04's `fs-uae`
  package is 3.1.66, which is below the requirement; install the official
  3.2.35 build.
- A licensed 512 KB Kickstart 3.1 A1200 ROM for emulator runs. KS3.0 is not
  tested. The CLI does not treat FS-UAE's built-in AROS as a verified run.

The installer is not published yet. Building from source needs Rust stable; the
pinned Docker SDK remains the development image.

## Toolchain pin

Download and extract the upstream Linux asset yourself. Clorraine does not
mirror or redistribute it.

| Field | Value |
|---|---|
| URL | `https://github.com/AmigaPorts/m68k-amigaos-gcc/releases/download/v16.2-rc24/m68k-amigaos-gcc-16.2-rc24-x86_64-linux.tar.xz` |
| Bytes | `69,868,548` |
| SHA-256 | `b93b070ac47d2f01d126f57c03a8405f71fb470c8397b0bda51041217d25c100` |

The extracted directory is the `--toolchain` root; it contains
`bin/m68k-amigaos-gcc` and `m68k-amigaos/`.

## Build from source and select it

```sh
git clone https://github.com/OpenLorraine/clorraine.git
cd clorraine
cargo build --workspace

# Point at the extracted rc24 root, not sdk/amiga.
./target/debug/clorraine setup --toolchain "$HOME/toolchains/m68k-amigaos-gcc-16.2"

./target/debug/clorraine build --profile hosted-amiga examples/hello/hello.clo
./target/debug/clorraine build --profile freestanding \
  spec/tests/corpus/freestanding/program.clo
```

`setup` validates the compiler's layout, its `16.2-rc24` version, the NDK
headers, and a two-runtime link probe (`nix20` and `nix13`). It stores the
canonical root only after all checks pass.

The config file is `$XDG_CONFIG_HOME/clorraine/config` when
`XDG_CONFIG_HOME` is absolute, otherwise `~/.config/clorraine/config`. Selection
order is:

1. `--toolchain PATH`
2. `CLORRAINE_TOOLCHAIN`
3. the stored config
4. the marker-gated `PATH` fallback inside the pinned SDK container only

A user host with a compiler on `PATH` but no selection is told to run `setup`;
it does not silently use that compiler. `--sdk-root` and `CLORRAINE_SDK` are
removed.

To check a toolchain without running a guest:

```sh
make toolchain-check TOOLCHAIN="$HOME/toolchains/m68k-amigaos-gcc-16.2"
```

## FS-UAE

The verified guest runner is the next phase. It will boot an A1200 with a
licensed KS3.1 A1200 ROM, run the program through a small wrapper that records
the return code, and write a status marker only for a zero exit. The CLI reads
that result before removing the private run.

Until that runner lands, a native toolchain selection reports that the verified
runner is unavailable, and the marked SDK container keeps its legacy handoff.
Neither path proves guest success; do not use it as acceptance evidence. The A500/Kickstart 1.3
gates remain the SDK development gates; the Linux CLI's emulator recipe is
A1200/KS3.1 and is not a Kickstart 1.3 proof.

## Editor

The VS Code extension is built separately:

```sh
make vscode-package
code --install-extension build/vscode/clorraine-1.2.0.vsix
```

The installer does not bundle or place the VSIX, and no marketplace release is
published. The extension update that follows this candidate uses
`~/.local/bin` and `clorraine.toolchain.path`; until it ships, set
`clorraine.driver.path` to the built `clorraine` binary if discovery fails.

## Related

- [Known limitations](known-limitations.md)
- [Docker SDK](../docker/README.md) for the development toolchain and Amiga gates
