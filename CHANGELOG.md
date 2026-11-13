# Changelog

## [Unreleased]

### Changed

- The repository license changed from MIT to the OSI-approved Apache License
  2.0, with the Clorraine Output Exception for compiled programs and embedded
  runtime and standard-library code. Clorraine is now open source: forks,
  bundling, and commercial distribution are allowed. The trademark policy is
  in TRADEMARKS.md. Releases published before the change remain available under
  MIT.

## [1.1.2] - 2026-11-11

### Changed

- Eligible module-level constant scalar array initializers (2- or 4-byte
  non-float leaf, complete guard-free constant list, at least 16 elements and
  64 bytes, no `volatile`) emit one read-only data template and one copy
  instead of one store per element. Byte, short/empty lists, `const`, fill,
  multi-extent, struct/view/float, and freestanding arrays keep the stores. Isolated under the
  old flags, `lookup` net instructions fell 13,653 → 9,470 (−30.6%), `.text`
  117,440 → 92,976 (−20.8%), and emitted C 474,430 → 340,503 bytes (−28.2%).
  Against v1.1.1 on `hosted-amiga`, with the shipped flags applied to both
  trees, `lookup` `.text` fell 64,328 → 39,904 (−38.0%) and the binary
  82,544 → 41,740 (−49.4%).
- Release programs on Linux host, `hosted-amiga`, and `hosted-amiga-13` build
  with `-O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`; the two
  GCC release rows in `spec/17-codegen.md` §13 match. Release programs are
  stripped; debug builds, freestanding, and non-Linux host program builds keep
  symbols. The same-compiler flag audit measures the five `hosted-amiga`
  scored benchmarks at `.text` 57.1–69.3% and binaries 64.2–74.7% smaller;
  host binaries fall 20.6–36.5%; the `hosted-amiga-13` audit set falls
  49.1–88.9% in `.text`. Calibration falls 3,372 → 2,928 instructions on 2.0
  and 3,348 → 2,947 on 1.3. Scored net instructions are unchanged by the
  flags.
- The constant-array template uses a collision-free symbol, so it cannot
  shadow a user `ffi` symbol. The collision existed only in unreleased builds.
- The Linux x86_64 tarball contains `clc` and `clorraine`, not
  `clorraine-lsp`; build the server from source with
  `cargo build --workspace`.

### Verification

`make check`, `make asan`, `make ubsan`, `make corpus-amiga`,
`make differential`, `make bench-shape`, `make bench-base COMMIT=v1.1.1`,
`make bench`, `make bench-compare REQUIRE="lookup:instructions:20
lookup:text:20"`, `make bench-flags`, `make verify`, and `make vscode-check`
pass for v1.1.2. `make verify` prints `verify: all legs passed`.

## [1.1.1] - 2026-11-09

### Fixed

- A function-body nested fixed-array fill into `volatile` storage now stores
  each element through the volatile lvalue instead of calling `memcpy`. The
  fix covers fill rows, place-valued and impure prefix rows, and run-time
  indexed elements. Accepted source such as `volatile let g: i32[8][9]` with
  `g = [; [; 0]];` now builds under strict C on host, m68k GCC, vbcc, and the
  freestanding profile.

### Changed

- The generated-C benchmark closeout: the matrix benchmark's nested zero fill
  compacts to one `memset`, reducing streamed guest instructions by 9.6% and
  `.text` by 5.0% on that benchmark, with no scored regression.

### Verification

`make check`, `make asan`, `make ubsan`, `make corpus-amiga`,
`make differential`, `make bench-shape`, `make bench-compare`, and
`make verify` pass for v1.1.1. `make verify` prints `verify: all legs passed`.

## [1.1.0] - 2026-11-08

### Added

- `hosted-amiga-13` profile for a stock A500 with Kickstart 1.3: 68000 code,
  OCS video, 512 KB chip memory, and the libnix `-mcrt=nix13` C runtime.
- Amiga SDK registry and Bundles 0-9: native video, sprites, audio/input,
  text/fonts/layers, DOS and ILBM, Intuition, Exec and devices, Workbench
  services, optional services, and Picasso96 RTG.
- Built-in `Box<T>` with generated recursive drop glue and the `boxNew`,
  `boxSet`, and `boxWith` standard-library surface.
- Ordinary Clorraine functions as `freedBy` deleters, so application types can
  own a recursive resource without an `ffi` block.
- C-hook trampolines for capturing lambdas: a synchronous `ffi` hook can call a
  capturing callback through a caller-stack environment and a plain C function
  pointer.
- Deterministic differential fuzzer with committed seeds, seed goldens, and a
  host/m68k/sanitizer matrix.
- UBSan verification leg (`make ubsan`) with an active-leg overflow probe.
- Benchmark harness and generated-C optimization suite: emitted size, m68k
  section sizes, host time, and the streamed `vamos` instruction count.

### Changed

- Generated C is optimized against measured benchmarks. The sieve benchmark
  reduces streamed guest instructions by 31.6%, `.text` size by 24.4%, and
  emitted C size by 36.6%.
- The A500/Kickstart 1.3 profile is part of the standard verification matrix.
- The compiler robustness closeout fixes six latent defects and the enum
  conversion path, and adds cross-toolchain shape and fault gates.

### Specification

- Approved v1.1 amendments: `Box<T>` (`P7a`), ordinary `freedBy` deleters
  (`X1`/`X1a`), and synchronous C-hook trampolines (`L5`).
- `E11215` faults a stored capturing callback invoked after its hook returns.
- The A500/Kickstart 1.3 capability baseline is recorded in the profile
  chapter.

### Verification

`make check`, `make asan`, `make ubsan`, `make corpus-amiga`,
`make differential`, `make bench-shape`, and `make verify` pass for v1.1.0.
`make verify` prints `verify: all legs passed`.

## [1.0.0] - 2026-11-04

### Added

- v1.0 specification freeze: chapters 00-17 and appendices A/B are `stable (v1.0)`.
- `std.amiga.exec`: private message ports, `NewMessage`/`Message`, signals, put/get/reply,
  and release operations.
- Hosted-amiga acceptance programs: `exec-self-test`, `message-drop`, and the MIT Hamurabi port.
- Developer tooling: `--explain CODE`, `--diagnostics=json`, and the canonical TextMate grammar
  under `editors/`.
- MIT licence, release checklist, known-limitations document, and third-party notices.
- Host-only CI workflow, `make asan`, and `make differential`.

### Fixed

- Release-blocking ownership, drop, nullability, and emission residuals from the
  production-readiness hardening phases.
- Driver behavior on a bare host: host-profile builds and `emit` no longer require the SDK marker.
- `make differential` argument forwarding: `DIFF_ONLY`/`OUTDIR` are passed through the container
  environment, never interpolated into the shell command.

### Verification

`make check`, `make verify`, `make asan`, and `make differential` pass for v1.0.0.
