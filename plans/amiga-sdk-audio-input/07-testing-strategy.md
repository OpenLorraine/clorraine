# Testing Strategy: Amiga SDK Bundle 3 — Audio and Input

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Specification Test Cases

| # | Scenario | Expected | Requirement |
|---|---|---|---|
| ST-1 | Full audio/input/timer surface compiles for `hosted-amiga` and `hosted-amiga-13` | exit 0; zero diagnostics; C emitted | R1–R9 |
| ST-2 | Host imports `std.amiga.audio` / `std.amiga.input` / `std.amiga.timer` | exit 1; `E11200`; no C | R9 |
| ST-3 | Full surface cross-builds with `nix13` and `nix20` | C compiles and links under strict warnings | R1–R9 |
| ST-4 | Emitted C for the audio/input/timer API | wrappers call the shim; audio/sample/timer drops are null-guarded and ordered | AR-12 |
| ST-5 | A500 paddle game on the real KS1.3 ROM | held display crop matches; `paddle: ok hits <N> audio 15 test 15 decode 127 input 0` line matches | R7, R8 |
| ST-6 | 1.3 still rejects `std.amiga.exec`; hello/Boing/sprites stay green | no regression | R9 |
| ST-7 | Deferred rows | `SDK-AUD-006` and `SDK-AUD-008` are `deferred`, not done | R11 |

## Test homes

| File | Cases |
|---|---|
| `compiler/tests/amiga_audio_spec_test.rs` | ST-1–ST-3, ST-6 |
| `compiler/tests/amiga_audio_impl_test.rs` | ST-4 |
| `examples/amiga/paddle.clo` + `docker/scripts/a500-paddle-smoke.sh` | ST-5 |
| `tools/amiga-sdk/registry_test.sh` / coverage | ST-7 |
| `make check`, `make verify` | full gate |

## Red phase

The spec tests are written before the modules and shim exist; they fail with
`E11102`/unresolved imports. The impl test fails because the wrappers have no
generated bodies. The red run is recorded in `05-findings.md`.

## Fixtures

- `examples/amiga/paddle.clo` — runtime acceptance.
- `docs/amiga-sdk/a500-paddle.png` — held game screen.
- `docs/amiga-sdk/a500-paddle-line.png` — paddle-line crop.
- `build/amiga-sdk/paddle-smoke/` — gitignored runtime scratch; the decrypted
  ROM is removed on exit.

## Verification

| Command | Covers |
|---|---|
| `make check` | spec check, registry validator, Rust suites |
| `make sdk-a500-smoke` | hello baseline |
| `make sdk-a500-boing` | Bundle 1 baseline |
| `make sdk-a500-sprites` | Bundle 2 baseline |
| `make sdk-a500-paddle` | ST-5 on real KS1.3 |
| `make verify` | all of the above plus differential/corpus/fault |
| `make sdk-registry` | Bundle 3 rows, coverage, issue #4 body |

## Pass criteria

- All new spec/impl tests green; no existing expectation changes.
- The live paddle screen and line match their goldens.
- `make verify` exits 0; Bundle 3 is 6/8 done with 2 deferred.
