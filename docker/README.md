# Clorraine SDK (Docker)

A self-contained, reproducibly built Amiga development environment. One image
provides the cross toolchain, the emulator, and the host tooling needed to
develop the Clorraine language, LorraineOS, and Amiga programs.

| Component | Purpose |
|---|---|
| `m68k-amigaos-gcc` / `g++` (GCC 16.2) | Primary development and debug compiler |
| `m68k-amigaos-gdb` | Source-level debugging for GCC-built binaries |
| binutils 2.46 (AmigaOS port, DWARF2) | Assembler, linker, Hunk executable support |
| libnix (Kickstart 2.0+ and 1.3) | Recommended runtime (`-mcrt=nix20`, `-mcrt=nix13`) |
| newlib, clib2 | Alternative C runtimes |
| NDK 3.2 headers + inlines | AmigaOS API development |
| vbcc + vasm + vlink | Release compiler for the smallest/fastest 68k code |
| sfdc, fd2sfd, fd2pragma | NDK header/inline generation tools |
| Rust (stable) | Implementation language of the Clorraine compiler |
| amitools / `vamos` | Runs AmigaOS CLI binaries on the host without a ROM |
| FS-UAE + xvfb | Runs programs with graphics/audio; headless mode for CI |
| host gcc/clang/gdb + cmake | Host-target builds and debugging of generated C |

## Ownership policy

The image builds its toolchains **from pinned upstream sources**; it does not
derive from anyone else's prebuilt toolchain image.

* All pinned inputs live in [`versions.env`](versions.env).
* Upstream ([`AmigaPorts/m68k-amigaos-gcc`](https://github.com/AmigaPorts/m68k-amigaos-gcc))
  is cloned at an exact commit and builds binutils, GCC, gdb, libnix, NDK
  headers, and vasm/vbcc/vlink from their own pinned repositories.
* Downloads with published checksums (NDK 3.2, GMP, MPFR, MPC) are verified by
  the upstream build system.
* The revisions actually compiled are recorded in the image at
  `/opt/amiga/BUILD-PROVENANCE.txt` (`make provenance` prints it).

## Quick start

```sh
make image     # first time only; compiles the toolchain from source (slow)
make init      # creates docker/.env and ~/amiga/{roms,adf,hdf,config}
make smoke     # verifies GCC, G++, NDK headers, vbcc, vamos
make shell     # interactive shell, this repository mounted at /work
```

`make image` needs Docker and network access. The toolchain build takes roughly
30-90 minutes depending on the machine; later image changes reuse the cached
toolchain stage.

## Daily workflows

| Command | What it does |
|---|---|
| `make shell` | One-shot interactive shell (`/work` = repository) |
| `make up` / `make down` | Persistent `dev` container; attach with `docker compose exec dev bash` |
| `make exec CMD="cargo test"` | Run any command in the SDK; exit code propagates |
| `make gui` | Shell with X11 access (for the emulator or GUI programs) |
| `make emulator` | FS-UAE with `KICKSTART`, `AMIGA_MODEL`, `FS_UAE_ARGS` from `.env` |
| `make run PROG=build/hello` | Assembles `build/run/`, boots it, shows the window |
| `make run-vamos PROG=build/hello` | Runs a CLI program instantly, no emulator/ROM |
| `make smoke` | Toolchain smoke test |
| `make provenance` | Pinned build revisions |

Boot media for `make run` is controlled by `RUNNER` in `.env`:

* `dir` (default) mounts `build/run/` as a directory hard drive, so rebuilds
  need no image step.
* `adf` generates a bootable floppy with `xdftool` (`boot install`), which is
  deterministic and suited to automation.

Program output is visible in the emulator window by default (with a short
"Clorraine runner" banner around it). For automation, set `RUN_CAPTURE=1` to
redirect output to `build/run/out.txt` on the host instead.

## Host data folder

`AMIGA_DATA` (default `~/amiga`) is mounted **read-only** at `/amiga`:

```
~/amiga/
  roms/     kickstart ROMs
  adf/      floppy images
  hdf/      hard disk images
  config/   fs-uae configs (see examples/default.fs-uae)
```

The repository is mounted **read-write** at `/work` because builds and tests
write artifacts there (both `build/` and `target/` are gitignored).

## Kickstart ROM policy

Kickstart ROMs are copyrighted and are **never** included in the image or the
repository. `KICKSTART=internal` (the default) uses FS-UAE's built-in AROS
replacement ROM, which requires no ROM files and is enough for most programs.
To use your own ROMs, place them in `~/amiga/roms` and set `KICKSTART` and
`AMIGA_MODEL` in `docker/.env` (`make init` suggests values for detected ROMs).

## X11 troubleshooting

The emulator window needs access to your X server. The SDK mounts
`/tmp/.X11-unix` and your X cookie (detected into `XAUTHORITY` by `make init`).

| Symptom | Fix |
|---|---|
| "cannot open display" | Check `DISPLAY` in `docker/.env`; verify with `make gui` then `xdpyinfo` |
| Cookie error | Fix `XAUTHORITY` in `docker/.env`; find it with `echo $XAUTHORITY` |
| Still refused (remote/Wayland) | Run `xhost +local:` on the host (grants local clients; less secure) |

## Flexibility

Anything machine-specific (extra mounts, ports, devices, audio) goes in
`docker/compose.override.yaml`, which is gitignored. Start from
[`compose.override.yaml.example`](compose.override.yaml.example).

Every service accepts arbitrary commands, for example:

```sh
docker compose -p clorraine --env-file .env -f compose.yaml run --rm dev bash -lc 'm68k-amigaos-gcc --version'
```

## Updating the toolchain

1. Choose a newer commit of `AMIGA_GCC_REPO` and set it in `versions.env`.
2. Optionally adjust `TOOLCHAIN_TARGETS` (for example add `clib2` or `ixemul`).
3. Rebuild and verify: `make image smoke`.

## Roadmap

* `make run-ci`: headless emulator runs (xvfb) with output capture for CI.
* Source-level debugging of Clorraine programs (`#line` source maps + debug
  adapter) built on the existing gdb/emulator paths.
* Evaluate `vscode-amiga-debug` for interactive 68k debugging.
