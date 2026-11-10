# Testing Strategy: Build-Flag and Binary-Size Closeout (PLAN-26)

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

The feature changes build commands, a measurement script, and gate wiring; it
does not change emitted C or runtime behavior. The tests therefore pin exact
command vectors and metric shapes, then prove the real artifact behavior with
the pinned toolchain and the full gate.

| Code type | Target |
|---|---|
| Command-vector and harness logic | 100% of the changed branches |
| Audit floor logic | every floor, pass and fail |
| End-to-end artifact behavior | every profile and gate listed in R4.7 |

## Specification Test Cases

### Release command vectors

| # | Input / Scenario | Expected output / behavior | Source |
|---|---|---|---|
| ST-1 | `host` Linux release, `--release` build | `cc -std=c99 -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o prog out.c` | R2.1, AR-04 |
| ST-2 | `hosted-amiga` release | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o prog out.c -lamiga -lm` | R2.2, AR-04 |
| ST-3 | `hosted-amiga-13` release | the ST-2 vector with `-mcrt=nix13` | R2.2, AR-04 |
| ST-4 | `freestanding` m68k release | the current `-O2` line, no GC/strip | R2.3, AR-07 |
| ST-5 | `freestanding` host-run release, `native_syscalls` true and false | `cc -std=c99 -ffreestanding -fno-builtin [-nostdlib] -O2 -o prog out.c`, no GC/strip | R2.3, AR-07 |
| ST-6 | Debug builds for every profile | the current `-O0 -g -Wall -Wextra -Werror` vector, no GC/strip | R2.4, AR-12 |
| ST-7 | `host_release_flags(true)` and `host_release_flags(false)` | Linux vector and `["-O2"]` respectively, both exercised on Linux | R2.5, R5.3, AR-06 |
| ST-8 | `spec/17-codegen.md` §13 read by a driver spec test | exactly the two GCC release rows differ; debug, vbcc, and freestanding rows are byte-identical | R2.9, AR-19 |

### Harness and audit

| # | Input / Scenario | Expected output / behavior | Source |
|---|---|---|---|
| ST-9 | `bench.HOST_RELEASE` and `bench.M68K_RELEASE` | the ST-1/ST-2 vectors' release suffixes | R4.1, AR-08 |
| ST-10 | `measure()` on a scored program, and the document's `calibration` summary | the scored record has `m68k_release_bytes` equal to the linked file size; the calibration summary keeps `instructions`/`host_release_ns` | R4.2, AR-08 |
| ST-11 | Driver constants composed with their bases, parsed by the harness test | `["-std=c99"] + HOST_RELEASE_GNU` equals `bench.HOST_RELEASE`; `bench.M68K_BASE + M68K_RELEASE_FLAGS` equals `bench.M68K_RELEASE`; the `-mcrt=nix13` composition matches the 13 audit vector; the fallback/freestanding arrays match | R2.6, R5.4, AR-22/AR-25 |
| ST-12 | Synthetic audit matrix where every floor passes | `--check` exits 0 and prints all checks passed | R1.6, R3, AR-23 |
| ST-13 | Synthetic audit matrix with one missed floor per case (m68k text, m68k binary, host binary, 1.3 text, calibration, net instructions, output, freestanding growth) | `--check` checks every group, prints every miss, writes the JSON, and exits 1 | R3.10, AR-21/AR-23 |
| ST-14 | Two audit runs of the real repeat set | every non-timing record is identical; a differing field fails | R1.5, R5.6, AR-09 |
| ST-15 | Real audit on the pinned image | the constructor audit lists the removed `__amigapath.o`/`environ.o` entries, the remaining list entries, and trace evidence that the `_start` loop and a remaining constructor run; all four 1.3 corpus outputs match from a private working directory | R1.7, R3.4, AR-13/AR-20 |
| ST-16 | Real audit on the pinned image | each scored m68k `.text` ≤ 80% old, stripped file ≤ 80%, host file ≤ 85%, each 1.3 corpus `.text` ≤ 60%, calibration raw ≤ 90%, net instructions ≤ 101%, outputs equal | R3.1–R3.7, AR-10 |
| ST-17 | Freestanding audit pair | the shipped `.text` and file bytes exactly equal the old pair; the separate GC probe records the 208 → 232-byte growth; the command is unchanged | R3.8, AR-07 |

### Gates and integration

| # | Input / Scenario | Expected output / behavior | Source |
|---|---|---|---|
| ST-18 | `make bench-shape` with the shipped flags | nix20 and nix13 compile warning-free; vbcc/freestanding unchanged | R4.6, AR-04 |
| ST-19 | `make bench-base COMMIT=f9d5be8` then `make bench` and `make bench-compare` | `base.json.compiler.git == f9d5be8…`, `dirty == false`, flags equal, no output/net/text regression | R4.5, R4.7, AR-03/AR-08 |
| ST-20 | `make sdk-a600-services` and `make sdk-a1200-workbench` with the release binaries | both report lines and the Workbench capture/AppIcon goldens match | R4.6, AR-20/AR-24 |
| ST-21 | Host release binary after the change | `readelf -S` shows no `.symtab` section (a missing tool fails, not skips); the program still runs and prints the same output | R2.1, R3.7, AR-05 |
| ST-22 | `clorraine --help` | the `--release` line describes GC/strip for Linux/Amiga, says freestanding/non-Linux keep symbols, and says debug keeps symbols | R6.1a, AR-14 |

## Test Categories

### Specification and Implementation Tests

| Test file / module | ST cases | Component |
|---|---|---|
| `compiler/src/clorraine/toolchain.rs` unit module | ST-1–ST-7 | Command vectors (unit/implementation tests; the repo reserves `*_spec_test.rs` for end-to-end behavior) |
| `compiler/tests/clorraine_driver_spec_test.rs` | ST-8, ST-22 | Spec-row pin and CLI help |
| `tools/perf/tests/test_bench.py` | ST-9–ST-11 | Harness flags and metric |
| `tools/perf/tests/test_flag_audit.py` | ST-12–ST-14 (synthetic floor/repeat logic) | Audit logic |
| `make bench-flags` (real gate) | ST-14–ST-17, ST-21 | Real audit, repeats, host symbol table |
| `make bench-shape`, release Workbench gates | ST-18, ST-20 | Shape and release Workbench |
| `make bench-base` / `bench` / `bench-compare` | ST-19 | Base identity and no regression |

### Implementation Tests

| Test | Description | Priority |
|---|---|---|
| `tools/perf/tests/test_flag_audit.py` (extra cases) | malformed matrix, missing tool, duplicate program, non-numeric field | High |
| `toolchain.rs` unit module | no global `RELEASE_FLAGS`; every `Tool` has a release slice; freestanding host-run both branches | High |
| `test_bench.py` | `binary_bytes` on a temp file; missing file error | Medium |

### Integration / End-to-End

| Test | Components | Description |
|---|---|---|
| `make bench-flags` | `clc`, GCC, `vamos`, audit | Real matrix, floors, two runs, 1.3 outputs |
| `make bench-shape` | GCC, vbcc, freestanding | Warning-free compile with the shipped flags |
| `make sdk-a600-services` / `make sdk-a1200-workbench` | Release Workbench/services | Report lines and goldens |
| `make bench-base` / `make bench` / `make bench-compare` | Harness | Base identity and no regression |
| `make check`, `make asan`, `make ubsan`, `make corpus-amiga`, `make differential`, `make verify` | Whole repository | No behavior or safety regression |

## Test Data

- Existing `benchmarks/*/program.clo` and `expected-output.txt`; unchanged.
- Existing `spec/tests/corpus/{buffer-container,file-copier,string-parser,volatile-mmio}`;
  unchanged.
- A synthetic audit matrix in `tools/perf/tests/test_flag_audit.py`; no
  external fixture.
- The post-PLAN-25 `clc 1.1.1` compiler at `f9d5be8` via `make bench-base`;
  no copied source.

## Verification Checklist

- [ ] Every ST case has a concrete expectation and a source.
- [ ] Specification tests are written before implementation and shown red.
- [ ] Every floor has a passing and a failing synthetic case.
- [ ] The real audit repeats identically.
- [ ] Release Workbench and 1.3 corpus gates pass.
- [ ] The full R4.7 gate passes on the final head.
- [ ] No test weakens or edits a benchmark/spec golden except the two
      authorized spec rows.
