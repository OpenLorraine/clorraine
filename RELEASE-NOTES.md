# Clorraine v1.2.0

> **Candidate release record.** v1.2.0 is the current Linux x86_64 CLI
> candidate. Toolchain setup, native builds, the verified FS-UAE runner, a local
> installer/archive, and a locally built VSIX are implemented. No release asset
> is published. v1.1.2 remains the latest published release under its MIT
> terms.

Clorraine is a compiler for the frozen v1.0 language with the approved v1.1
language surfaces, plus a build/run driver for the `host`, `hosted-amiga`,
`hosted-amiga-13`, and `freestanding` profiles. v1.2.0 adds a Linux CLI that
validates a user-supplied upstream toolchain and builds without Docker.

## Candidate highlights

- **User-supplied toolchain.** `clorraine setup --toolchain PATH` validates the
  AmigaPorts `m68k-amigaos-gcc` v16.2-rc24 root, checks both `nix20` and `nix13`
  link probes, and stores the canonical path. Clorraine does not bundle, mirror,
  or redistribute the toolchain.
- **One selection contract.** `--toolchain` and `CLORRAINE_TOOLCHAIN` override
  the stored config. `--sdk-root` and `CLORRAINE_SDK` are removed. Host builds
  need no toolchain.
- **Native build path.** `clc` and `clorraine` build the existing profiles with
  the selected absolute compiler. The `make toolchain-check` target builds
  hello, the portable corpus, and the freestanding program with rc24.
- **Linux desktop editor candidate.** The VS Code extension remains a separate
  local VSIX. The installer does not place it and no marketplace release is
  published.
- **Verified FS-UAE runner.** A native `run --emulator` validates FS-UAE 3.2.0+
  and a licensed 512 KB Kickstart 3.1 A1200 ROM, stages a private A1200
  directory drive, and runs the program through a compiled wrapper. Success is
  rc 0 plus status `ok`; a successful guest's output is printed or captured, and
  a failed guest prints its output and is retained with its path. Both Amiga
  profiles run on the A1200 recipe.
- **Local installer and archive.** `make dist` writes the versioned archive,
  `install.sh`, the VSIX, and `SHA256SUMS` under `build/dist`; the installer
  checks the host and archive, activates side by side, and supports uninstall
  and purge. `make dist-verify` and the hermetic `make dist-check` pass.
- **Real guest check.** `make fs-uae-check KICKSTART=PATH [TOOLCHAIN=PATH]` runs
  hello, the four portable corpus programs, one A500-profile corpus program, and
  a return-1 negative, and rejects a surviving FS-UAE or Xvfb process.
- **Legacy SDK container.** The pinned Docker SDK keeps its marker-gated PATH
  lookup and FS-UAE host handoff for development. A user host with a compiler on
  PATH but no selection is told to run `setup`.

The crash-safe installer with separate VSIX and the legacy-pipeline removal are
the next phases. No tag, release, upload, or publication has occurred.

## Verification

The candidate checks are `cargo test --workspace`, 19 FS-UAE runner spec cases,
`make toolchain-check TOOLCHAIN=<rc24 root>`, `make fs-uae-check
KICKSTART=<KS3.1 A1200> TOOLCHAIN=<rc24 root>`, `make dist`, `make dist-verify`,
`make dist-check`, `make check`, and `make site-check`. The real guest check passed hello, the four corpus programs,
the A500-profile corpus program, and the rc-1 negative on FS-UAE 3.2.35 with the
licensed 524,299-byte KS3.1 A1200 ROM. The full acceptance gate is `make verify`,
which needs the pinned SDK image, an X display, and the licensed media. The
installer and archive acceptance evidence are added in later phases.

## Known limitations

- Linux x86_64 with glibc 2.38 or newer; Ubuntu 24.04 is the minimum baseline.
  Ubuntu 22.04 cannot run the rc24 host tools.
- The supported toolchain is rc24; other versions are rejected by `setup`.
- FS-UAE 3.2.0 or newer. Ubuntu 24.04's 3.1.66 package is below the requirement.
- The tested guest media is a licensed 512 KB Kickstart 3.1 A1200 ROM. KS3.0 is
  untested; AROS is manual-only and not an acceptance path.
- The verified `--emulator` runner requires a native toolchain, FS-UAE 3.2.0+,
  and a licensed 512 KB Kickstart 3.1 A1200 ROM. It is an A1200/KS3.1 recipe,
  not a Kickstart 1.3 proof; the A500/KS1.3 gates remain in the SDK. A failed
  guest leaves its private run on disk for inspection.
- macOS, Windows, WSL, and ARM hosts are outside v1.2.0.
- The accepted language and platform limitations remain in
  `docs/known-limitations.md`.

## Licence

First-party v1.2.0 artifacts use the Apache License 2.0 with the Clorraine
Output Exception. Required first-party terms are `LICENSE`,
`LICENSE-EXCEPTION.md`, `NOTICE`, `TRADEMARKS.md` and
`THIRD_PARTY_NOTICES.md`. Third-party components keep their original terms; the
notices do not relicense them.

## Historical release: v1.1.2

> **Historical release.** v1.1.2 was published under the MIT License. The
> repository now uses the Apache License 2.0 with the Clorraine Output
> Exception. The published v1.1.2 artifacts are unchanged and remain available
> under MIT.

v1.1.2 is a patch release; the language and command-line surface are unchanged.
Its highlights were:

- **Eligible constant arrays become data.** A module-level fixed array with a
  2- or 4-byte non-float scalar leaf, a complete guard-free constant list, at
  least 16 elements and 64 bytes, and no `volatile` emits one read-only data
  template and one copy instead of one store per element. On the `lookup`
  benchmark, net instructions fell 13,653 → 9,470 (−30.6%), `.text`
  117,440 → 92,976 (−20.8%), and emitted C 474,430 → 340,503 bytes (−28.2%).
- **Much smaller release programs.** Release programs on Linux host,
  `hosted-amiga`, and `hosted-amiga-13` use section garbage collection and
  release stripping. The five `hosted-amiga` scored benchmarks are `.text`
  57.1–69.3% smaller and their stripped binaries 64.2–74.7% smaller; host
  stripped release binaries fall 20.6–36.5%.
- **No collision with user `ffi` symbols.** The data template uses a
  collision-free symbol.

The v1.1.2 gate ran `make check`, `make asan`, `make ubsan`,
`make corpus-amiga`, `make differential`, `make bench-shape`,
`make bench-base COMMIT=v1.1.1`, `make bench`,
`make bench-compare REQUIRE="lookup:instructions:20 lookup:text:20"`,
`make bench-flags`, `make verify`, and `make vscode-check`. The published
Linux x86_64 tarball contained `clc` and `clorraine`, not `clorraine-lsp`.
Licence terms and asset identities are recorded in `RELEASE.md` and
`CHANGELOG.md` as history.

No NDK, ROM, Workbench media, or other restrictively licensed third-party file
was redistributed in the v1.1.2 source or binary release artifacts.
