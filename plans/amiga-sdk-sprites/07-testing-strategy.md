# Testing Strategy: Amiga SDK Bundle 2 — Sprites and Collision

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Specification Test Cases

| # | Scenario | Expected | Requirement |
|---|---|---|---|
| ST-1 | Full sprite surface compiles for `hosted-amiga` and `hosted-amiga-13` | exit 0; zero diagnostics; C emitted | R1–R9 |
| ST-2 | Host imports `std.amiga.sprite` | exit 1; `E11200`; no C | R9 |
| ST-3 | Full surface cross-builds with `nix13` and `nix20` | C compiles and links under strict warnings | R1–R9 |
| ST-4 | Emitted C for the sprite API | wrappers call the shim; sprite/animation drops are null-guarded and ordered | AR-11 |
| ST-5 | A500 sprite demo on the real KS1.3 ROM | 640x512 held display crop matches; `sprite-demo: collision 0x1000` line matches | R7, R8 |
| ST-6 | 1.3 still rejects `std.amiga.exec`; hello/Boing stay green | no regression | R9 |

## Test homes

| File | Cases |
|---|---|
| `compiler/tests/amiga_sprite_spec_test.rs` | ST-1–ST-3, ST-6 |
| `compiler/tests/amiga_sprite_impl_test.rs` | ST-4 |
| `examples/amiga/sprite-demo.clo` + `docker/scripts/a500-sprite-smoke.sh` | ST-5 |
| `make check`, `make verify`, `make sdk-registry` | full gate |

## Red phase

The spec tests are written before `stdlib/std.amiga.sprite.clo` and the shim
helpers exist; they fail with `E11102`/unresolved imports. The impl test fails
because the wrappers have no generated bodies. The red run is recorded in
`05-findings.md`.

## Fixtures

- `examples/amiga/sprite-demo.clo` — runtime acceptance.
- `docs/amiga-sdk/a500-sprites.png` — held display crop.
- `docs/amiga-sdk/a500-sprites-line.png` — collision-line crop.
- `build/amiga-sdk/sprite-smoke/` — gitignored runtime scratch; the decrypted
  ROM is removed on exit.

## Verification

| Command | Covers |
|---|---|
| `make check` | spec check, registry validator, Rust suites |
| `make sdk-a500-smoke` | hello baseline on real KS1.3 |
| `make sdk-a500-boing` | Bundle 1 Boing baseline |
| `make sdk-a500-sprites` | ST-5 on real KS1.3 |
| `make verify` | all of the above plus differential/corpus/fault |
| `make sdk-registry` | Bundle 2 rows, coverage, issue #3 body |

## Pass criteria

- All new spec/impl tests green; no existing expectation changes.
- The live sprite display and collision line match their goldens.
- `make verify` exits 0; registry Bundle 2 is 6/6 done.
