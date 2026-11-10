# Clorraine v1.1.2

Clorraine is a compiler for the frozen v1.0 language with the approved v1.1
language surfaces, plus a build/run driver for the `host`, `hosted-amiga`,
`hosted-amiga-13`, and `freestanding` profiles. v1.1.2 is a patch release; the
language and command-line surface are unchanged.

## Highlights

- **Eligible constant arrays become data.** A module-level fixed array with a
  2- or 4-byte non-float scalar leaf, a complete guard-free constant list, at
  least 16 elements and 64 bytes, and no `volatile` now emits one read-only
  data template and one copy instead of one store per element. Byte,
  short/empty lists, `const`, fill, multi-extent, struct/view/float, and
  freestanding arrays keep the stores; the conditions are recorded in `docs/known-limitations.md` §13.
  On the `lookup` benchmark, isolated under the previous flags, net
  instructions fell 13,653 → 9,470 (−30.6%), `.text`
  117,440 → 92,976 (−20.8%), and emitted C 474,430 → 340,503 bytes (−28.2%).
- **Much smaller release programs.** Release programs on Linux host,
  `hosted-amiga`, and `hosted-amiga-13` now use section garbage collection and
  release stripping:
  `-O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`. Measured
  with one compiler and the same sources, the five `hosted-amiga` scored
  benchmarks are `.text` 57.1–69.3% smaller and their stripped binaries
  64.2–74.7% smaller; host stripped release binaries fall 20.6–36.5%; the
  `hosted-amiga-13` audit set falls 49.1–88.9% in `.text`; calibration falls
  3,372 → 2,928 instructions on 2.0 and 3,348 → 2,947 on 1.3. The two GCC
  release rows in the frozen compile matrix match the new commands.
- **Release programs are stripped; tools and debug builds are not.** The
  `clc` and `clorraine` binaries keep their symbols. Debug builds keep
  symbols. Freestanding and non-Linux host program builds keep the `-O2` line
  and symbols. Use a debug build when you need to diagnose a release program.
- **No collision with user `ffi` symbols.** The new data template uses a
  collision-free symbol; it cannot shadow a user `ffi` name. The collision
  existed only in unreleased builds.

## Verification

The v1.1.2 release gate runs `make check`, `make asan`, `make ubsan`,
`make corpus-amiga`, `make differential`, `make bench-shape`,
`make bench-base COMMIT=v1.1.1`, `make bench`,
`make bench-compare REQUIRE="lookup:instructions:20 lookup:text:20"`,
`make bench-flags`, `make verify`, and `make vscode-check` on the gated
release commit. `make verify` prints `verify: all legs passed`.

Against v1.1.1, with the shipped flags applied to both trees, `lookup` net
instructions are 13,653 → 9,470 (−30.6%), `.text` 64,328 → 39,904 (−38.0%),
the binary 82,544 → 41,740 (−49.4%), and emitted C 474,430 → 340,503 bytes
(−28.2%). The other four scored benchmarks have identical net instructions,
`.text`, and emitted C. The flag audit's constructor proof shows the four
remaining libnix constructors still run on both m68k profiles.

## Known limitations

- Programs built by `clorraine build --release` on Linux host, `hosted-amiga`,
  or `hosted-amiga-13` are stripped; a debug build is the symbol-bearing
  artifact for diagnosis.
- `freestanding` and non-Linux host program builds keep the plain `-O2` line
  and symbols.
- The Linux x86_64 tarball contains `clc` and `clorraine`, not
  `clorraine-lsp`; build the server from source with
  `cargo build --workspace`.
- `--gc-sections` removes the unused libnix `__amigapath`/`environ`
  constructor objects and their init entries. No current standard-library API
  observes that state; a future API that does will need a targeted keep.
- The accepted limitations are recorded in `docs/known-limitations.md`,
  including the remaining volatile-copy shapes, the A500 media requirement,
  the RTG scope, and the v1.1 `Box<T>` and hook boundaries.

## Licence

MIT. Third-party notices, including the Hamurabi port, are in
`THIRD_PARTY_NOTICES.md`. The binary tarball includes that file and
`docs/known-limitations.md`, so the licence and limitations links in the
shipped README resolve; the other README links point to the source tree.

No NDK, ROM, Workbench media, or other restrictively licensed third-party file
is redistributed in the source or binary release artifacts.
