# Testing Strategy: Amiga SDK Bundle 1 — Native Video

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Specification Test Cases

| # | Scenario | Expected | Requirement |
|---|---|---|---|
| ST-1 | Import `std.amiga.video` and `std.math` from a `hosted-amiga` program | exit 0; zero diagnostics; C emitted | R1–R12 |
| ST-2 | Import `std.amiga.video` and `std.math` from a `hosted-amiga-13` program | exit 0; zero diagnostics; C emitted | R1–R12, R14 |
| ST-3 | Import `std.amiga.video` from a `host` program | exit 1; `E11200`; no C | AR-2, AR-11 |
| ST-4 | Cross-build the full native-video program for `hosted-amiga-13` with `-mcrt=nix13 -Wall -Wextra -Werror` | C compiles and links | R1–R12 |
| ST-5 | Cross-build the same program for `hosted-amiga` with `-mcrt=nix20` | C compiles and links; 3.1 unchanged | R11 |
| ST-6 | Host program prints `sin(0)`, `sin(64)`, `sin(128)`, `sin(192)`, `cos(0)`, `sqrt(0)`, `sqrt(15)`, `sqrt(16)`, `sqrt(1000000)` | `0 256 0 -256 256 0 3 4 1000` | R8 |
| ST-7 | Emitted C for a program using the video API | calls `cl_std_amiga_video_*` wrappers; every `Bitmap`/`Display`/`Copper`/`ChipBuffer` drop is null-guarded | AR-12 |
| ST-8 | `hosted-amiga-13` imports `std.amiga.exec` | still `E11200`; no regression | R11 |
| ST-9 | A500 Boing run on the real KS1.3 ROM | held display crop matches `a500-boing-frame.png`; final line matches `boing: checksum <8 hex>` golden crop | R10, R11 |
| ST-10 | The Boing demo's internal chip-buffer, plane-bit, and checksum self-checks | the final checksum line is printed; any self-check failure prints a distinct `boing: ... failed` line | R1, R2, R9 |

## Test homes

| File | Cases |
|---|---|
| `compiler/tests/amiga_video_spec_test.rs` | ST-1–ST-6, ST-8 |
| `compiler/tests/amiga_video_impl_test.rs` | ST-7 |
| `stdlib/std.math.clo` host run via the spec test | ST-6 |
| `examples/amiga/boing.clo` + `docker/scripts/a500-boing-smoke.sh` | ST-9, ST-10 |
| `make check`, `make verify`, `make sdk-registry` | ST-11 (full gate) |

## Red phase

ST-1–ST-5 and ST-8 are written before `stdlib/std.amiga.video.clo` and
`stdlib/std.math.clo` exist. The red run fails at the unresolved module import
(`E10000`/unresolved import) for both hosted profiles. ST-6 fails because the
host cannot find `std.math`. ST-7 fails because the wrappers have no generated
bodies. The red output is recorded in `05-findings.md`.

## Fixtures

- `examples/amiga/boing.clo` — the runtime acceptance program.
- `examples/hello/hello.clo` — the existing A500 baseline; must stay green.
- `docs/amiga-sdk/a500-boing.png` — live display evidence.
- `docs/amiga-sdk/a500-boing-line.png` — golden checksum-line crop.
- `build/amiga-sdk/boing-smoke/` — gitignored runtime scratch; the decrypted
  ROM is removed on exit.

## Verification

| Command | Covers |
|---|---|
| `make check` | spec check, registry validator, full Rust suite including ST-1–ST-8 |
| `make sdk-a500-smoke` | existing hello baseline on the real KS1.3 ROM |
| `make sdk-a500-boing` | ST-9, ST-10 on the real KS1.3 ROM |
| `make verify` | all of the above plus the existing differential/corpus/fault legs |
| `make sdk-registry` | covers Bundle 1 rows, coverage generation, issue #2 body |

## Pass criteria

- Every new spec test is green; no existing test changes its expectation.
- The A500 Boing screenshot shows the ball on the gradient background.
- The checksum crop is byte-identical to the committed golden.
- `make verify` exits 0 and `make sdk-registry` reports Bundle 1 11/11.
