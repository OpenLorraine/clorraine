---
title: Project status
description: "What works in Clorraine today, what is still in progress, and how each claim is verified."
---

# Project status

<StatusBadge state="preview" />

Clorraine is a **developer preview**. The compiler core is usable for the
supported language and the tracked Amiga SDK surface. Installation, packaging,
editor distribution, and some platform support are still in progress.

This page records the state of each area and the evidence behind it. If a
statement here disagrees with a release note, the release note wins.

## What works today

| Area | State | Evidence |
|---|---|---|
| Language and compiler core | <StatusBadge state="ready" label="Usable" /> | Frozen specification, compile-fail, run, staged, and corpus suites |
| Ownership and drop checking | <StatusBadge state="ready" label="Implemented" /> | Non-lexical lifetimes and the v2 borrow model suites |
| Amiga SDK | <StatusBadge state="ready" label="81 / 81 tracked" /> | SDK registry, coverage table, FS-UAE acceptance gates |
| `host` profile | <StatusBadge state="ready" label="Usable" /> | Host corpus and Rust test suites |
| `hosted-amiga` profile | <StatusBadge state="ready" label="Usable" /> | m68k GCC builds, `vamos` runs, FS-UAE gates |
| `hosted-amiga-13` profile | <StatusBadge state="ready" label="Usable" /> | A500/Kickstart 1.3 builds and FS-UAE runs |
| `freestanding` profile | <StatusBadge state="ready" label="Provisional" /> | Freestanding build and run legs |
| Linux x86_64 binaries | <StatusBadge state="ready" label="Released" /> | [v1.1.2 release](https://github.com/OpenLorraine/clorraine/releases/tag/v1.1.2) |
| Language server | <StatusBadge state="ready" label="From source" /> | `clorraine-lsp`, built with `cargo build --workspace` |
| VS Code extension | <StatusBadge state="progress" label="Not published" /> | `make vscode-check` builds and audits a VSIX |
| Windows and macOS installers | <StatusBadge state="progress" /> | No release yet |
| Documentation site | <StatusBadge state="ready" label="Published" /> | [openlorraine.github.io/clorraine](https://openlorraine.github.io/clorraine/) |
| First Light demo | <StatusBadge state="ready" label="Live" /> | [Runs in Copperline](/examples/first-light) |
| Prism demo | <StatusBadge state="ready" label="Live" /> | [Runs in Copperline](/examples/prism) |
| Boing demo | <StatusBadge state="ready" label="Live" /> | [Runs in Copperline](/examples/graphics) |

## What this means

::: warning Developer preview

Clorraine is suitable for early adopters, experiments, and feedback. It is not
yet a production toolchain. Expect changes to installation, packaging, and
platform support.

:::

The compiler is not a toy: the language specification is frozen, the
implementation runs a large acceptance suite, and every Amiga gate boots a real
emulator. The unfinished parts are around distribution and convenience, not
around whether the compiler can build and run the supported programs.

## How the claims are checked

| Claim | Check |
|---|---|
| The language behaves as specified | `./spec/check.sh` and the specification test suites |
| Emitted C is valid C99 | Strict host and m68k builds, including `-Werror` legs |
| Ownership and drops are correct | Borrow-model suites, ASan and UBSan probes |
| The compiler does not crash on odd input | Deterministic differential fuzzing |
| Host and Amiga behavior agree | `make differential` and `make corpus-amiga` |
| Amiga demos run on real emulated hardware | FS-UAE gates for A500, A600, A1200, and A4000 profiles |
| Release artifacts are reproducible and gated | `make verify`, `make bench`, and `make vscode-check` |

The detailed list is on the [verification page](/project/verification).

## What is not ready

- **One-click installers.** The Linux x86_64 tarball is available. Windows and
  macOS packaging is not.
- **A published editor extension.** The VS Code client builds, packages, and
  passes a packaged-client smoke test, but it is not in a marketplace.
- **Broad platform testing.** The host CI runner is Linux. Amiga legs need the
  pinned SDK image, the emulators, and the licensed media described below.
- **The full AmigaOS API.** The SDK covers 81 tracked capabilities. That is a
  growing subset, not every library and every version.
- **A stable `freestanding` ABI.** The profile is implemented with a provisional
  ABI for LorraineOS.

## Amiga media

The A500, A600, A1200, and A4000 FS-UAE gates use licensed media supplied by the
person running the gate. No ROM or Workbench image is committed or distributed.
`vamos` runs need no ROM, and the default FS-UAE setup uses its built-in AROS
replacement ROM.

Read the [licensing and provenance page](/project/licensing) for the media and
NDK policy.

## Release and license state

The current release is **v1.1.2**, published under the MIT License. The
repository now uses the **Apache License 2.0** with the
[Clorraine Output Exception](/project/licensing) for future releases. Published
v1.1.2 artifacts are unchanged.

## Next

- [Roadmap](/guide/roadmap)
- [Known limitations](/guide/known-limitations)
- [Quick start](/guide/quick-start)
- [Release notes](https://github.com/OpenLorraine/clorraine/releases)
