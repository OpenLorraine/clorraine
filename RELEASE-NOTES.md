# Clorraine v1.0.0

Clorraine is a compiler for the frozen v1.0 language and a build/run driver for the
`host`, `hosted-amiga`, and `freestanding` profiles.

## Highlights

- `clc` compiles `.clo` sources to one valid C99 translation unit.
- `clorraine` builds and runs programs through the profile toolchain, including the
  FS-UAE handoff for `hosted-amiga`.
- The embedded standard library covers memory, text, formatting, console/file I/O, and the
  Amiga modules `std.amiga.{custom,dos,exec,graphics,interrupt,intuition}`.
- Developer tooling: `--explain CODE` on both binaries, `--diagnostics=json` NDJSON on stderr,
  and the canonical `editors/clorraine.tmLanguage.json` grammar.
- The specification chapters 00-17 and appendices A/B are `stable (v1.0)`.

## Verification

`make check`, `make verify` (differential, FS-UAE corpus, six runtime faults), `make asan`
(24 suites), and `make differential` (3/3) pass for this release.

## Known limitations

See `docs/known-limitations.md` for the three accepted v1.0 limitations. The
`std.amiga.exec` message lifetime contract and the pre-existing compiler residuals are recorded
in the production-readiness findings ledger.

## Licence

MIT. Third-party notices, including the Hamurabi port, are in `THIRD_PARTY_NOTICES.md`.
No NDK, ROM, or other restrictively licensed third-party file is redistributed.
