# Testing Strategy: Amiga SDK Baseline (Bundle 0)

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Specification Test Cases

| # | Scenario | Expected | Requirement |
|---|---|---|---|
| ST-1 | `Profile::parse("hosted-amiga-13")` and `name()` round-trip | `Some`, name is `hosted-amiga-13` | R1 |
| ST-2 | `toolchain::tool(HostedAmiga13)` debug command | `m68k-amigaos-gcc -std=c99 -mcrt=nix13 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror ... -lamiga -lm` | R1 |
| ST-3 | 1.3 program importing `std.amiga.graphics` | exit 0; C emitted | R2 |
| ST-4 | 1.3 program importing `std.amiga.exec` | exit 1; `E11200`; no C | R2 |
| ST-5 | 3.1 program importing `std.amiga.exec` | exit 0; unchanged behavior | R3 |
| ST-6 | `clc --profile hosted-amiga-13` hello, then cross-build with `-mcrt=nix13` | C compiles and links under strict flags | R3, R4 |
| ST-7 | A500 hello under `vamos` | pre-check only: captured line is `Hello, Amiga, from Clorraine!`, exit 0 | R4 |
| ST-7b | A500 hello under FS-UAE A500/KS1.3 | captured visible line is `Hello, Amiga, from Clorraine!`, exit 0; currently blocked (AR-17) | R4 |
| ST-8 | Registry validator with a duplicate ID / bad status / missing dependency / done without evidence | each is rejected with a named error | R5 |
| ST-9 | `registry.sh coverage` on the seed | deterministic `coverage.md`; every bundle appears | R5 |
| ST-10 | `registry.sh issues` on the seed | one body per bundle, each with its capability checklist | R6 |
| ST-11 | `make check`, `make verify`, `./spec/check.sh`, `make sdk-registry` | green | R8 |

## Test homes

| File | Cases |
|---|---|
| `compiler/src/profile.rs` unit tests | ST-1, ST-3, ST-4 |
| `compiler/src/clorraine/toolchain.rs` unit tests | ST-2 |
| `compiler/tests/amiga_a500_spec_test.rs` | ST-3–ST-6 |
| `tools/amiga-sdk/registry_test.sh` | ST-8–ST-10 |
| `docker/scripts/a500-smoke.sh` | ST-7 |
| `make check` / `make verify` / `spec/check.sh` | ST-11 |

## Red phase

ST-1–ST-6 are written before the profile exists. They fail with `E11201`
(unknown profile) or the missing `hosted-amiga-13` name; the red run is recorded
in the findings ledger. ST-8–ST-10 are written before `registry.sh` exists; the
test script fails with a missing-command error.

## Fixtures

- `examples/hello/hello.clo` — the A500 runtime program.
- `docs/amiga-sdk/registry.tsv` + `bundles.tsv` — the seed.
- Temporary malformed registry copies under `$CODEOPS_TMPDIR` for ST-8.

## Verification

- `make sdk-registry` — validate + regenerate coverage.
- `make check` — spec/workspace gate.
- `make verify` — full Amiga/FS-UAE gate; the 3.1 legs must stay green.
- `docker/scripts/a500-smoke.sh` — A500/1.3 build + `vamos` pre-check (V47 API emulation).
- The acceptance is the FS-UAE A500/KS1.3 visible run; it is blocked pending AR-17.
- The FS-UAE A500/KS1.3 visual run is a manual check; the automated capture is
  deferred and recorded in `05-findings.md`.
