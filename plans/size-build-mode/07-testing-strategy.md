# Testing Strategy: Opt-in Size Build Mode (PLAN-37)

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## 1. Test Layers

| Layer | File | Oracle |
|---|---|---|
| CLI parsing | `compiler/src/clorraine/cli.rs` unit module | A command line becomes the intended `BuildMode`, or a usage error |
| Command vectors | `compiler/src/clorraine/toolchain.rs` unit module | Every profile has the exact `-Os` vector; debug and release vectors are unchanged |
| End-to-end driver | `compiler/tests/clorraine_driver_spec_test.rs` | The built binary accepts `--size`, builds/runs the program, rejects the conflict, and lists the flag in help |
| Existing contract | the rest of `cargo test --workspace` | No default, emitted-C, runtime, or benchmark change |
| Full gate | `make check`, sanitizers, `make verify` | The change is safe in the shipped pipeline |

The feature does not touch `clc`, so there is no `*_spec_test.rs` /
`*_impl_test.rs` pair for compiler behaviour. The driver suite is the
specification oracle; the two unit modules pin the internals.

## 2. CLI Cases

| # | Input | Expected | Traces |
|---|---|---|---|
| ST-1 | `build --size a.clo` | `mode == BuildMode::Size` | R1 |
| ST-2 | `build --release a.clo` | `mode == BuildMode::Release` | R5 |
| ST-3 | `build a.clo` | `mode == BuildMode::Debug` | R5 |
| ST-4 | `build --release --size a.clo` | usage error naming the conflict | R3 |
| ST-5 | `build --size --size a.clo` | accepted; `mode == Size` | R3 |
| ST-6 | `emit --size a.clo` | usage error | R4 |
| ST-7 | `emit --release a.clo` | unchanged usage error | R4 |
| ST-8 | `run --size --profile freestanding a.clo` | accepted with the freestanding profile | R7 |

## 3. Command-Vector Cases

| # | Tool / mode | Expected | Traces |
|---|---|---|---|
| IT-1 | host Linux, `Size` | `cc -std=c99 -Os -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o prog out.c` | R6 |
| IT-2 | host non-Linux, `Size` | `cc -std=c99 -Os -o prog out.c` | R6 |
| IT-3 | `hosted-amiga`, `Size` | the nix20 line with `-Os` and the release section flags | R6 |
| IT-4 | `hosted-amiga-13`, `Size` | the nix13 line with `-Os` and the release section flags | R6 |
| IT-5 | freestanding m68k, `Size` | the freestanding line with `-Os`, no GC/strip | R6 |
| IT-6 | freestanding host run, native syscalls, `Size` | the no-libc line with `-Os` | R6 |
| IT-7 | freestanding host run, fallback, `Size` | the fallback line with `-Os` | R6 |
| IT-8 | every profile, `Debug` | the existing `-O0 -g -Wall -Wextra -Werror` vector | R5 |
| IT-9 | every profile, `Release` | the existing `-O2` vector | R5 |

## 4. End-to-End Cases

| # | Scenario | Expected | Traces |
|---|---|---|---|
| ST-9 | `build --size hello.clo` | exit 0, executable exists, program does not run | R1, R2 |
| ST-10 | `run --size hello.clo` | exit 0, program output is the only stdout | R1 |
| ST-11 | `build --release --size hello.clo` | exit 2, stderr names the conflict | R3 |
| ST-12 | `emit --size hello.clo` | exit 2, no output C | R4 |
| ST-13 | `--help` | lists `--size`, `-Os`, and the release conflict | R8 |
| ST-14 | `run --size --profile hosted-amiga hello.clo` | exit 0 and the program output under `vamos` (skips without the SDK) | R1, R6 |

The host end-to-end cases run on a bare host with `cc`. The Amiga case skips
itself with a note when `m68k-amigaos-gcc` or `vamos` is absent, matching the
existing driver suite.

## 5. Red/Green Order

1. Write the CLI cases, the command-vector cases, and the end-to-end cases
   before implementation.
2. Run them on the base driver. Expected red:
   - `--size` is an unknown option in every `--size` case;
   - the `BuildMode` type and `Tool.size` do not exist, so the unit modules do
     not compile until the test-side type names are present.
3. Implement the enum, the `Tool.size` slices, the CLI mode, and the help text.
4. Re-run the focused suites until green.
5. Run the full workspace suite, then the gate.

## 6. Measurement Verification

| Check | Method |
|---|---|
| Same emission | The `.c` file bytes for a size and release build of the same program/profile are identical |
| Profile vectors | The toolchain unit cases pin the exact command lines |
| Size win | Build `calibration` and `strings` in both modes for each profile; record artifact bytes and m68k `.text` |
| Runtime trade | Count dynamic instructions under `vamos --instr-trace` for the two programs on `hosted-amiga`; record host wall time as advisory |
| Default unchanged | Compare the release vectors and `make bench-compare` against the base |

## 7. Gate Coverage

| Gate | What it proves |
|---|---|
| `make check` | The new cases and the whole existing suite |
| `make asan` / `make ubsan` | The driver change introduces no memory or undefined-behaviour defect |
| `make bench-shape` | The emitted C still builds on every toolchain leg |
| `make bench-compare` | The scored `-O2` line and every benchmark row are unchanged |
| `make corpus-amiga` / `make differential` | The m68k outputs are unchanged |
| `make verify` | The full acceptance gate |
