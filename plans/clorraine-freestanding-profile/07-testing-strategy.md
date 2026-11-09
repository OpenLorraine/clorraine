# Testing Strategy: Freestanding profile

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Compiler profile and gating logic | 90% |
| Runtime shim (exercised end to end) | 80% |
| Driver toolchain and pipelines | 80% |
| Specification goldens | all pass |

- Test names state behavior: `should [expected behavior] when [condition]`.
- The freestanding run harness is the E2E test: it drives the emitted C through
  the real no-libc toolchain and the host syscall layer. The m68k leg cannot be
  executed without an OS, so its assertion is link self-containment, not output.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived from `01-requirements.md`, the component specs, and the Ambiguity
> Register. The immutable-oracle rule applies: if the implementation disagrees,
> the implementation is wrong.

### Profile selection and gating

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-1 | `clc --profile freestanding -o out.c main.clo`, `main` returns 0 | exit 0; `out.c` contains `void _start(void)`, `typedef unsigned int usize;`, `cl_trap`, and a `memcpy` definition | R1, R3, R4, R5 / AR-13, AR-19 |
| ST-2 | The same program under `--profile host` and `--profile hosted-amiga` | byte-identical to the pre-change output (no regression) | R1 / AR-19 |
| ST-3 | `import { bufferNew } from std.mem;` under freestanding | exit 1; first stderr line `error E11200: 'std.mem' is not available on profile 'freestanding'` | R8 / AR-11 |
| ST-4 | `import { String } from std.text;` under freestanding | exit 1; `E11200` for `std.text` | R8 / AR-11 |
| ST-5 | `import { toHex } from std.fmt;` under freestanding | exit 1; `E11200` for `std.fmt` | R8 / AR-11 |
| ST-6 | `import { delayTicks } from std.amiga.dos;` under freestanding | exit 1; `E11200` for `std.amiga.dos` | R8 / AR-11 |
| ST-7 | `func main(): i32 { let x: f64 = 1.5; return i32(x * 4.0); }` under freestanding | exit 0; no `E11200` | R10 / AR-10 |
| ST-8 | `Profile::parse("freestanding")`; `supports_module("std.io")`; `supports_module("std.mem")`; `stack_frame_limit()`; `shim()` | `Some`, true, false, 1024, and the shim names `_start`/`memcpy`/`cl_trap`/`Open`/`Close`/`Read`/`Write`/`cl_write`/`cl_write_line` | R2, R8, R13 / AR-4, AR-11, AR-16 |
| ST-9 | `clc --profile nope main.clo` | exit 1; `error E11201: unknown profile 'nope'` | R1 / AR-30 |
| ST-10 | `clorraine run --profile freestanding` and the help text | the profile is accepted; help lists `freestanding` | R12 / AR-4, AR-20 |

### m68k build and link

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-11 | Compile+link the freestanding C for m68k with `-ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv -Wall -Wextra -Werror … -lg` | the link command exits 0 (the linker fails on any unresolved symbol; `-lg` supplies the float helpers) | R6 / AR-7, AR-10, AR-27 |
| ST-12 | The same link at `-O2` | link exits 0 (no self-recursive `memcpy`/`memset`) | R16 / AR-9 |
| ST-13 | The `toolchain` command vector for freestanding | equals the normative `spec/17-codegen.md` §13 line with `-lg` placed after the input | R6, R12 / AR-20, AR-27 |

### Host no-libc run

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-14 | `println("hello")` under freestanding, host no-libc run | stdout `hello\n`; exit 0 | R7, R8 / AR-7 |
| ST-15 | `func main(): i32 { return 7; }` | run exits 7 | R4 / AR-13 |
| ST-16 | A program that copies a local array and a struct, then returns a derived `i32` | run exits with the expected value (proves `memcpy`/`memset` resolve) | R5 / AR-9 |
| ST-17 | The ST-7 float program on the host no-libc run | run exits 6 | R10 / AR-10 |
| ST-18 | A program calling `openFile("x", 1005)` | stderr contains `error E11214: file I/O is not available on the freestanding profile`; run exits non-zero | R9 / AR-12, AR-28 |
| ST-19 | A program dividing by zero | stderr first line `error E11210: division by zero`; run exits non-zero | R11 / AR-14 |

### Driver and specification

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-20 | `clorraine build --profile freestanding main.clo` | exit 0; the m68k artifact exists under `build/clorraine/<stem>/freestanding/` | R12 / AR-20 |
| ST-21 | `clorraine run --profile freestanding main.clo` | prints the program output; exit is the program's code | R12 / AR-20 |
| ST-22 | `./spec/check.sh` after the golden edits | passes; the `E11200_unimplemented_profile.clo` golden is gone, the replacement `E11200_unavailable_module.clo` golden is present, and `E11201_unknown_profile.clo` remains | R14 / AR-21, AR-30 |

## Test Categories

### Specification Tests (from ST-cases above)

> Written BEFORE implementation.

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/freestanding_spec_test.rs` | ST-1, ST-3..ST-7, ST-11..ST-21 | Profile, runtime, driver |
| `compiler/tests/stage_spec_test.rs` (updated) | ST-9, ST-22 | Profile selection |
| `compiler/src/profile.rs` `mod tests` | ST-8 | Profile facts |
| `compiler/src/stdlib.rs` `mod tests` | ST-8 | Per-profile shim/symbol coverage |
| `compiler/src/clorraine/toolchain.rs` `mod tests` | ST-13 | Toolchain table |
| `compiler/src/clorraine/cli.rs` `mod tests` | ST-10 | CLI text |
| `compiler/tests/compile_fail_spec_test.rs` | ST-22 | E11200 replacement golden |
| `compiler/tests/codegen_run_spec_test.rs` (existing) | ST-2 | Regression |

### Implementation Tests (edge cases, internals)

> Written AFTER implementation.

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/freestanding_spec_test.rs` | The link-exit assertion; the `-O2` link; the fallback path selection; a program with no `main` | High |
| `compiler/src/profile.rs` | `implemented()`; the freestanding shim markers and that it references no libg libc-ish symbol (`exit`/`abort`/`malloc`) | Medium |
| `compiler/src/driver.rs` | The unknown-name and unimplemented branches | Medium |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Freestanding run harness | compiler + runtime + host `cc` | Emit, compile, link, run, compare output and exit code |
| Freestanding m68k link | compiler + runtime + `m68k-amigaos-gcc` | Compile and link with `-nostdlib -lg`; assert the link exits 0 |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Console program | Emit under freestanding; run the host no-libc build | `hello\n`, exit 0 |
| Mixed-arch fallback | Force the fallback path (`native_syscalls = false`); run | The libc-linked build still runs and prints |
| Make passthrough | `make run FILE=… PROFILE=freestanding` in the container | Builds and runs through the driver |

## Test Data

### Fixtures Needed

- `spec/tests/corpus/freestanding/program.clo` and `expected-output.txt`.
- `spec/tests/compile-fail/E11200_unavailable_module.clo` (the replacement
  `E11200` golden).
- Small inline `.clo` snippets for ST-3..ST-7, ST-15..ST-19 (written to a
  scratch directory by the harness).
- The m68k link test needs `m68k-amigaos-gcc`, so it runs only inside the SDK
  container via `make check` / `make verify` (the same gate the other Amiga legs
  use).

### Mock Requirements

None. The runtime and toolchain are real; the only test-only seam is the
`native_syscalls` boolean, which selects the arch path deterministically instead
of depending on the host machine.

## Verification Checklist

- [x] All specification test cases (ST-*) defined with concrete input/output
- [x] Every ST case traces to a requirement or AR entry
- [x] Specification tests written before implementation
- [x] Specification tests verified to fail before implementation (red phase)
- [x] All specification tests pass after implementation (green phase)
- [x] Implementation tests written for edge cases and internals
- [x] All unit / integration / E2E tests pass
- [x] No regressions in existing tests (`host` and `hosted-amiga` unchanged)
- [x] Test coverage meets goals
