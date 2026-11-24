# Linux CLI (v1.2.0 candidate)

Clorraine's Linux CLI is the product path for writing, building, and running
Clorraine programs without Docker. The v1.2.0 candidate is still in progress:

| Area | State |
|---|---|
| `clorraine setup --toolchain` and config | Implemented in the candidate |
| `--toolchain` selection and builds | Implemented in the candidate |
| Linux installer, archive, and VSIX | Local candidate in `build/dist`; no publication |
| Verified FS-UAE runner | Implemented; needs FS-UAE 3.2.0+, a licensed KS3.1 A1200 ROM, and a native toolchain |
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

The installer is a local candidate, not a published download. Building from
source needs Rust stable; the pinned Docker SDK remains the development image.

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

## Local candidate and installer

`make dist` builds the release binaries in the pinned container, packages the
VSIX, writes the release documents, and creates the local candidate under
`build/dist`:

```sh
make dist
make dist-verify
```

The assets are the versioned archive, `install.sh`, the VSIX, and `SHA256SUMS`.
`make dist-check` runs the hermetic installer and producer tests. Nothing is
tagged, published, or uploaded.

The local acceptance run uses a fresh HOME and the pinned inputs:

```sh
make acceptance CANDIDATE=build/dist \
  TOOLCHAIN_ARCHIVE="$HOME/toolchains/m68k-amigaos-gcc-16.2-rc24-x86_64-linux.tar.xz" \
  KICKSTART="$HOME/amiga/roms/amiga-os-310-a1200.rom"
make acceptance-check EVIDENCE=build/acceptance/<stamp>/evidence.json \
  CANDIDATE=build/dist
```

Acceptance executes the self-built candidate installer with your privileges; do
not point it at an untrusted candidate. The checker proves the record's format
and the candidate/corpus bytes, not that the run happened; keep the raw
acceptance directory. Publication remains a separate approval.

Install the exact local bytes:

```sh
build/dist/install.sh --archive build/dist/clorraine-1.2.0-x86_64-linux.tar.gz \
  --checksums build/dist/SHA256SUMS --modify-path
```

The default prefix is `~/.local`, so the driver lands at
`~/.local/bin/clorraine`. The installer checks Linux x86_64, glibc 2.38+, and
the archive hash before extraction; it activates side by side with an atomic
`current` switch. `--uninstall` removes the active version and PATH block;
`--purge` also removes the retained versions and the CLI config. A network
install verifies the pinned archive size and SHA-256 before extraction and is
only meaningful after a release is published.

## FS-UAE

With a native toolchain selected, `run --emulator` boots an A1200 with your
licensed Kickstart 3.1 A1200 ROM and runs the program through a small compiled
wrapper. The wrapper records the return code in `clorraine-rc.txt` and writes
`clorraine-status.txt` only for a zero exit. The CLI accepts the run only for
rc 0 plus status `ok`, reads the guest output, then removes the private run. A
failed, unreadable, or timed-out run retains the private directory and prints
its path.

```sh
./target/debug/clorraine run -p hosted-amiga --emulator \
  --kickstart "$HOME/amiga/roms/amiga-os-310-a1200.rom" \
  examples/hello/hello.clo
```

FS-UAE is selected by `--fs-uae`, `CLORRAINE_FS_UAE`, then `PATH`; the ROM by
`--kickstart` then `CLORRAINE_KICKSTART`. The timeout defaults to 90 seconds and
is set with `CLORRAINE_EMULATOR_TIMEOUT_SECONDS` (1–3600). `--capture` writes a
successful guest's output to `out.txt` and prints its path; a failed guest
prints its output and retains the run, so no `out.txt` is written.

The local guest check runs hello, the four corpus programs, one A500-profile
corpus program, and a return-1 negative:

```sh
make fs-uae-check KICKSTART="$HOME/amiga/roms/amiga-os-310-a1200.rom" \
  TOOLCHAIN="$HOME/toolchains/m68k-amigaos-gcc-16.2"
```

`TOOLCHAIN` may be omitted when the CLI has a stored selection. The A500 profile
binary runs on the A1200 recipe; this is not a Kickstart 1.3 proof. The
A500/KS1.3 gates remain the SDK development gates.

## Editor

The VS Code extension is built separately:

```sh
make vscode-package
code --install-extension build/vscode/clorraine-1.2.0.vsix
```

The installer does not bundle or place the VSIX, and no marketplace release is
published. The extension discovers `~/.local/bin/clorraine` and uses
`clorraine.toolchain.path` for the upstream root. The removed
`clorraine.sdk.path` is read once with a warning when the new key is empty.

## Related

- [Known limitations](known-limitations.md)
- [Docker SDK](../docker/README.md) for the development toolchain and Amiga gates
