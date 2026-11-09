# Clorraine v1.1.1

Clorraine is a compiler for the frozen v1.0 language with the approved v1.1
language surfaces, plus a build/run driver for the `host`, `hosted-amiga`,
`hosted-amiga-13`, and `freestanding` profiles. v1.1.1 is a patch release.

## Highlights

- **Volatile nested fills emit valid C.** A function-body nested fixed-array
  fill into a `volatile` module array or a `volatile` struct field now stores
  each element through the volatile lvalue instead of calling `memcpy` with a
  volatile destination. The fix covers fill rows, explicit prefix rows
  (including impure values evaluated once), and run-time indexed elements.
  Accepted source such as `volatile let g: i32[8][9]` with `g = [; [; 0]];`
  now builds under strict C on host, m68k GCC, vbcc, and the freestanding
  profile.
- **Generated-C matrix win.** The matrix benchmark's nested zero fill is now
  one `memset`, reducing streamed guest instructions by 9.6% and `.text` by
  5.0% on that benchmark. No scored benchmark regresses.

## Verification

The v1.1.1 release gate runs `make check`, `make asan`, `make ubsan`,
`make corpus-amiga`, `make differential`, `make bench-shape`,
`make bench-compare`, `make verify`, and `make vscode-check` on the exact
release tree. `make verify` prints `verify: all legs passed`.

## Known limitations

The accepted limitations are recorded in `docs/known-limitations.md`, including
the remaining volatile-copy shapes (whole-array copies, volatile sources
through a row temporary, and by-value rvalue members), the A500 media
requirement, the RTG scope, and the v1.1 `Box<T>` and hook boundaries.

## Licence

MIT. Third-party notices, including the Hamurabi port, are in
`THIRD_PARTY_NOTICES.md`.

No NDK, ROM, Workbench media, or other restrictively licensed third-party file
is redistributed in the source or binary release artifacts.
