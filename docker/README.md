# c-next Amiga SDK (Docker)

A self-contained, reproducibly built Amiga development toolchain for the
c-next project. One image provides everything needed to compile and test
Amiga programs from any modern host:

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
| Rust (stable) | Implementation language of the c-next compiler |
| amitools / `vamos` | Runs AmigaOS CLI binaries on the host without a ROM |
| host gcc/clang/cmake | Host-target builds and tests |

## Ownership policy

This image builds its toolchains **from pinned upstream sources**; it does not
derive from anyone else's prebuilt toolchain image.

* All pinned inputs live in [`versions.env`](versions.env).
* The upstream build system (`AmigaPorts/m68k-amigaos-gcc`) is cloned at an
  exact commit and builds binutils, GCC, gdb, libnix, NDK headers, and
  vasm/vbcc/vlink from their own pinned repositories.
* Downloads with published checksums (NDK 3.2, GMP, MPFR, MPC) are verified by
  the upstream Makefile.
* The exact revisions actually compiled are recorded in the image at
  `/opt/amiga/BUILD-PROVENANCE.txt` (`make provenance` prints it).

## Usage

Build the image (first build compiles the toolchain; expect 20-40 minutes):

```sh
make image
```

Interactive shell with this repository mounted at `/work`:

```sh
make shell
```

Verify the toolchain:

```sh
make smoke
```

Both `make smoke` and `make shell` run on the **host** from this `docker/`
directory. Inside a running container, re-verify with `cnext-smoke` directly.

Compile an Amiga program from inside the shell:

```sh
m68k-amigaos-gcc -mcrt=nix20 -O2 -o hello hello.c   # or: vc +aos68k ...
vamos hello                                         # run it, no ROM needed
```

## Updating the toolchain

1. Choose a newer commit of `AMIGA_GCC_REPO` and set it in `versions.env`.
2. Optionally adjust `TOOLCHAIN_TARGETS` (for example add `clib2` or `ixemul`).
3. Rebuild and rerun the smoke test:

```sh
make image smoke
```

If a tool arises that upstream does not build, add a script under `scripts/`
and call it from the builder stage rather than patching third-party images.

## Kickstart ROM policy

Kickstart ROMs are copyrighted and are **never** included in the image. Hosts
that need a full emulator (graphics, intuition, hardware timing) must supply
their own ROMs. Automated tests use `vamos`, which emulates the AmigaOS API
directly and needs no ROM.

## Roadmap

* Add FS-UAE (headless) for integration tests that need a real Kickstart.
* Vendor mirrored source archives into our own storage as an extra hedge
  against upstream repository disappearance.
* Add emulator-based test runner (`docker/scripts/run-emulator-tests.sh`).
