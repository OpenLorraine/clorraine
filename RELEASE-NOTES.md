# Clorraine v1.2.0

> **Candidate release record.** v1.2.0 is the current release target. Its
> artifacts are prepared as drafts and remain private until the explicit
> publication approval recorded in `RELEASE.md` is supplied. v1.1.2 remains the
> latest published release and is unchanged under its MIT terms.

Clorraine is a compiler for the frozen v1.0 language with the approved v1.1
language surfaces, plus a build/run driver for the `host`, `hosted-amiga`,
`hosted-amiga-13`, and `freestanding` profiles. v1.2.0 adds a native Linux
installation path and editor actions without changing the language rules.

## Highlights

- **One native Linux installer.** `install.sh` downloads a hash-pinned helper,
  verifies its exact length and SHA-256, and runs it with isolated Python. The
  installer places the three compiler binaries, a reusable prebuilt Amiga SDK,
  the NDK preparation helpers and a CPython 3.10/3.12 `vamos` wheel set under
  `~/.clorraine` by default. It never compiles installation dependencies.
- **Transactional updates and recovery.** Installation builds a complete
  candidate before activation, keeps the previous working version and refuses a
  concurrent writer. `--root` and `--release` select a custom root or an exact
  distribution tag explicitly.
- **Linux desktop editor.** The VS Code extension adds Build, Run and Run in
  FS-UAE actions. They save the active `.clo` entry, ask the language server for
  the canonical saved-source closure, and pass the entry and its imported sources
  directly to the driver. Machine-scoped settings find `~/.clorraine/bin`
  without a terminal `PATH`. v1.2.0 supports the Linux desktop only; WSL2,
  Remote-WSL and Windows are outside this release.
- **Bounded release automation.** Two manually dispatched workflows build
  support and compiler/editor/distribution drafts. They never publish on a
  build; promotion needs a separately reviewed manifest hash.

## Verification

The v1.2.0 candidate gate is `make verify`, `make vscode-check`, `make
site-check`, the clean Ubuntu 22.04/24.04 installation runs, the native FS-UAE
and legacy Amiga gates on private displays, and the Linux desktop editor record.
The final candidate also needs the release specification suite and an
independent acceptance review before any promotion. An unresolved
critical/major finding, a changed approved byte or a missing post-publication
check blocks the corresponding stage; it is never replaced by a host-only or
miniature-fixture result. WSL2/Remote-WSL and Windows are not acceptance gates
because v1.2.0 does not claim them.

## Known limitations

- Native Windows, macOS, WSL2/Remote-WSL and ARM artifacts are not part of this
  release. Windows and WSL users are outside the v1.2.0 support claim.
- FS-UAE and licensed Kickstart/Workbench media are user-provided. No emulator,
  ROM or Workbench content is redistributed.
- Automatic Windows emulator launch, native emulator capture and automatic
  shell-profile or extension changes are outside this delivery. The release
  makes no WSL2/Remote-WSL or Windows support claim.
- Support assets retain third-party terms. The exact `libamiga.a` input and its
  unresolved legal review are recorded in `THIRD_PARTY_NOTICES.md`; public
  promotion stays separately authorised.
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
