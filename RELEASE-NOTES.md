# Clorraine v1.1.0

Clorraine is a compiler for the frozen v1.0 language with the approved v1.1
language surfaces, plus a build/run driver for the `host`, `hosted-amiga`,
`hosted-amiga-13`, and `freestanding` profiles.

## Highlights

- **A500/Kickstart 1.3 support.** The `hosted-amiga-13` profile targets a stock
  A500: 68000 code, OCS video, 512 KB chip memory, and the libnix `-mcrt=nix13`
  runtime. The Amiga SDK registry covers Bundles 0-9, including native video,
  sprites, audio and input, text and layers, DOS and ILBM assets, Intuition,
  Exec and devices, Workbench services, optional services, and Picasso96 RTG.
- **`Box<T>`.** The built-in owning pointer has generated recursive drop glue and
  the `boxNew`, `boxSet`, and `boxWith` standard-library surface. Allocation
  failure uses the existing `E1` trap; Box is move-only and non-nullable.
- **Ordinary `freedBy` deleters.** A handle deleter can be an ordinary Clorraine
  function with the existing `take T -> void` signature, so application types can
  own a recursive resource without writing an `ffi` block.
- **C-hook trampolines.** A capturing lambda can cross a synchronous C hook. The
  compiler emits a per-argument trampoline and a caller-stack environment, so the
  C side still sees a plain function pointer. A stored callback invoked after the
  hook returns faults `E11215`.
- **Generated-C optimization and benchmarks.** The benchmark harness measures
  emitted size, m68k section sizes, host time, and the streamed `vamos`
  instruction count. On the sieve benchmark the optimized compiler reduces
  streamed guest instructions by 31.6%, `.text` size by 24.4%, and emitted C size
  by 36.6%.
- **Robustness closeout.** Six latent compiler defects and the enum conversion
  path are fixed. The verification suite now includes a host UBSan leg
  (`make ubsan`) and a deterministic differential fuzzer with committed seeds,
  seed goldens, and a host/m68k/sanitizer matrix.

## Verification

The v1.1.0 release gate runs `make check`, `make asan`, `make ubsan`,
`make corpus-amiga`, `make differential`, `make bench-shape`, and `make verify`
on the exact release tree. `make verify` prints `verify: all legs passed`.

The gate covers the specification check, the Rust workspace suites, the
sanitizer legs, the portable corpus on host and hosted-amiga, the FS-UAE
acceptance programs, the runtime-fault programs, and the cross-toolchain shape
fixture.

## Known limitations

The accepted v1.0 and v1.1 limitations are recorded in
`docs/known-limitations.md`, including the A500 media requirement, the RTG
scope, the v1.1 `Box<T>` and hook boundaries, and the compiler edge cases left
outside the robustness closeout.

## Licence

MIT. Third-party notices, including the Hamurabi port, are in
`THIRD_PARTY_NOTICES.md`.

No NDK, ROM, Workbench media, or other restrictively licensed third-party file
is redistributed in the source or binary release artifacts.
