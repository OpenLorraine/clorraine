---
title: Project brief
description: "A one-page introduction to Clorraine for Amiga platform partners: status, independence, license, compatibility, media and NDK handling, roadmap, and contact."
outline: false
---

# Clorraine — project brief

**Developer preview · v1.1.2 · Updated 13 November 2026**

Clorraine is a compiler for 68K Amiga systems. It compiles a C-like language
with Rust-style ownership to plain C99 and uses the AmigaPorts m68k GCC
toolchain. The compiler is usable today for the supported language and the
tracked Amiga SDK surface; installation, packaging, and platform support are in
progress.

This page is the brief. It is written to be forwarded without a call.

## At a glance

| | |
|---|---|
| **Project** | Clorraine, an independent compiler for 68K Amiga systems |
| **Owner** | OpenLorraine |
| **Status** | Developer preview; v1.1.2 is the current release |
| **Language** | C-like syntax, Rust-style ownership, no garbage collector |
| **Output** | One C99 translation unit, built with the AmigaPorts m68k GCC toolchain |
| **Targets** | `host`, `hosted-amiga`, `hosted-amiga-13`, `freestanding` |
| **License** | Apache License 2.0 with the Clorraine Output Exception; v1.1.2 was published under MIT |
| **Media** | No Kickstart ROMs or Workbench media are distributed |

## Independence

Clorraine is an independent implementation. It does not include or derive from
proprietary AmigaOS source code, Kickstart ROMs, or Workbench media. It is not
affiliated with or endorsed by Amiga Corporation or Hyperion Entertainment.

The compiler is written from scratch in Rust. Its Rust workspace uses the
standard library only and has no external crate dependencies. The Amiga
toolchain is built from pinned upstream sources.

## License

The compiler, runtime, standard library, and repository documentation are
licensed under the **Apache License 2.0**. The
[Clorraine Output Exception](/project/licensing) is an additional permission
that makes compiled programs, generated C, and embedded runtime and
standard-library code distributable under your own terms, including proprietary
terms.

Apache 2.0 permits commercial use, modification, redistribution, forking, and
bundling the compiler. The [trademark policy](https://github.com/OpenLorraine/clorraine/blob/main/TRADEMARKS.md)
governs use of the Clorraine name and logo.

Published **v1.1.2** artifacts remain available under the MIT License. The
Apache 2.0 terms apply to future releases.

Third-party notices, including the Hamurabi port and the Picasso96 developer
definitions, are recorded in
[THIRD_PARTY_NOTICES.md](https://github.com/OpenLorraine/clorraine/blob/main/THIRD_PARTY_NOTICES.md).

## Compatibility

Clorraine aims to support a growing subset of the AmigaOS 3.1 API. It is not an
AmigaOS replacement. It is a language and compiler that produces programs for
AmigaOS and for the future LorraineOS.

The tracked SDK surface currently covers **81 capabilities across 11 bundles**,
including native video, sprites, audio and input, text and layers, DOS and
assets, Intuition, Exec and devices, Workbench services, and optional services
such as RTG. Each capability has an acceptance gate.

The A500/Kickstart 1.3 profile is a first-class target: 68000, OCS, 512 KB chip
RAM, and `-mcrt=nix13`. It is not a reduced afterthought.

## Media and NDK

Clorraine does not distribute Kickstart ROMs or Workbench media. The repository
contains no ROM or Workbench image.

- `vamos` runs AmigaOS CLI binaries on the host without a ROM.
- FS-UAE's built-in AROS replacement ROM is the default emulator setup.
- Licensed Kickstart and Workbench media is supplied by the person running the
  advanced acceptance gates and is never committed.
- The SDK image builds its toolchain from pinned public upstream sources. It
  verifies downloaded inputs, including NDK 3.2, against published checksums.
  Clorraine does not publish or redistribute the NDK archive.

## Roadmap

- One-click installers for Linux, macOS, and Windows.
- A published VS Code extension; the packaged VSIX and its smoke test already
  pass from source.
- Clearer NDK setup and diagnostics.
- Broader host-platform testing and CI coverage.
- Continued expansion of the tracked AmigaOS API surface.

No delivery dates are promised. The project ships when the acceptance gates are
green.

## Contact

- **Project and licensing:** [OpenLorraine on GitHub](https://github.com/OpenLorraine/clorraine)
- **Issues and technical questions:** [GitHub Issues](https://github.com/OpenLorraine/clorraine/issues)
- **Documentation:** [openlorraine.github.io/clorraine](https://openlorraine.github.io/clorraine/)

A dedicated licensing address is being prepared. Until it is published, use the
repository to request a private channel.
