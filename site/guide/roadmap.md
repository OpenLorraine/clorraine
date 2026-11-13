---
title: Roadmap
description: "What Clorraine has delivered, what is next, and what is being explored. No dates are promised."
---

# Roadmap

This is the public roadmap. It describes direction, not a delivery contract.
Internal planning records are not published here.

## Delivered

| Area | State |
|---|---|
| Frozen v1.0 specification and golden test corpus | Done |
| Compiler, ownership checker, and C99 emitter | Done |
| Four target profiles: `host`, `hosted-amiga`, `hosted-amiga-13`, `freestanding` | Done |
| Amiga SDK: 81 tracked capabilities across 11 bundles | Done |
| v1.1 surfaces: `Box<T>`, ordinary `freedBy` deleters, C-hook trampolines | Done |
| A500/Kickstart 1.3 baseline with OCS and 512 KB chip RAM | Done |
| Linux x86_64 binary release | v1.1.2 |
| VS Code extension packaging and packaged-client smoke test | Done from source |
| Language server over stdio | Done from source |
| Apache 2.0 licensing with an output exception | Done |

## Next

| Item | Why it matters |
|---|---|
| Linux, macOS, and Windows installers | Remove the Docker prerequisite for the first run |
| Published VS Code extension | Make editor setup a single install |
| Clearer NDK setup and diagnostics | Make the Amiga boundary easier to understand |
| Broader host-platform testing and CI | Catch platform-specific issues before release |
| More AmigaOS API coverage | Grow the tracked surface beyond the current 81 capabilities |

## Exploring

- More complete RTG support beyond Picasso96 on UAEGFX and the emulated
  Picasso IV.
- Additional Workbench and optional-service coverage where the API can be
  proved on real media.
- A stable freestanding ABI for LorraineOS.
- More examples and tutorials from the community.

## How work is accepted

A change ships when the relevant gates are green. The fast gate is `make check`.
The full acceptance gate is `make verify`, which includes the Amiga builds,
emulator runs, differential checks, and runtime-fault programs. Performance
changes also pass the benchmark shape and comparison gates.

See [Verification](/project/verification) for the full picture.

## Releases

Release notes and artifacts are published on
[GitHub Releases](https://github.com/OpenLorraine/clorraine/releases). The
current release is **v1.1.2**.
