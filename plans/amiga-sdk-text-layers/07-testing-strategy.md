# Testing Strategy: Amiga SDK Bundle 4 — Text, Fonts, Layers

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Specification Test Cases

| # | Scenario | Expected | Requirement |
|---|---|---|---|
| ST-1 | Full text/layers surface compiles for `hosted-amiga` and `hosted-amiga-13` | exit 0; zero diagnostics; C emitted | R1–R8 |
| ST-2 | Host imports `std.amiga.text` / `std.amiga.layers` | exit 1; `E11200`; no C | R7 |
| ST-3 | Full surface cross-builds with `nix13` and `nix20` | C compiles and links under strict warnings | R1–R8 |
| ST-4 | Emitted C for the text/layer API | wrappers call the shim; font/layer drops are null-guarded and ordered | AR-11 |
| ST-5 | A500 scroller on the real KS1.3 ROM | 408x438 held display crop matches; `text: ok width 128 height 8 layer 160x64 checks 510` line matches | R5, R6 |
| ST-6 | 1.3 still rejects `std.amiga.exec`; earlier A500 gates stay green | no regression | R7 |
| ST-7 | Deferred rows | `SDK-TXT-003` and `SDK-TXT-005` are `deferred` with evidence | R9, R11 |

## Test homes

| File | Cases |
|---|---|
| `compiler/tests/amiga_text_spec_test.rs` | ST-1–ST-3, ST-6 |
| `compiler/tests/amiga_text_impl_test.rs` | ST-4 |
| `examples/amiga/scroller.clo` + `docker/scripts/a500-text-smoke.sh` | ST-5 |
| `make sdk-registry` / coverage | ST-7 |
| `make check`, `make verify` | full gate |

## Red phase

The spec tests are written before the modules and shim exist; they fail with
`E11102`/unresolved imports. The impl test fails because the wrappers have no
generated bodies. The red run is recorded in `05-findings.md`.

## Fixtures

- `examples/amiga/scroller.clo` — runtime acceptance.
- `docs/amiga-sdk/a500-text.png` — held scrolling screen.
- `docs/amiga-sdk/a500-text-line.png` — text-line crop.
- `build/amiga-sdk/text-smoke/` — gitignored runtime scratch; the decrypted ROM
  is removed on exit.

## Verification

| Command | Covers |
|---|---|
| `make check` | spec check, registry validator, Rust suites |
| `make sdk-a500-smoke` | hello baseline |
| `make sdk-a500-boing` | Boing baseline |
| `make sdk-a500-sprites` | sprite baseline |
| `make sdk-a500-paddle` | audio/input baseline |
| `make sdk-a500-text` | ST-5 on real KS1.3 |
| `make verify` | all of the above plus differential/corpus/fault |
| `make sdk-registry` | Bundle 4 rows, coverage, issue #5 body |

## Pass criteria

- All new spec/impl tests green; no existing expectation changes.
- The held layer frame and line match their goldens.
- `make verify` exits 0; Bundle 4 is 4/6 done with diskfont and regions deferred.
