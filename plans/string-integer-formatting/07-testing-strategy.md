# Testing Strategy: String Integer Formatting Throughput

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

The change is a runtime formatter replacement with an unchanged public
contract. The oracle is the previous `%d`/`%u` output plus the template and
ownership behavior already frozen by the specification. The performance
contract (no `snprintf` in the integer path) is observable through a wrapped
C library call, so it is a specification test, not a source-text pin.

| Code type | Target |
|---|---|
| Decimal formatter (boundaries and loops) | 100% of the listed ST cases |
| Converter integration (templates, `text`, `toDecimal`) | Every ST case |
| Profile parity | Host + hosted-amiga emission checks |
| Performance | Scored benchmark comparison and the call-count test |

## 🚨 Specification Test Cases

> Derived from `01-requirements.md` (R1–R5), `03-runtime-formatting.md`, and
> the Ambiguity Register. The expectations come from the prior `%d`/`%u`
> contract and the frozen template rules, not from the new implementation.
> This table is the immutable oracle.

### Decimal boundaries

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-1 | `let n: i32 = 0; $"n={n}"` | `n=0` | R1 / AR-03 |
| ST-2 | `let n: i32 = 1; $"n={n}"` | `n=1` | R1 / AR-03 |
| ST-3 | `let n: i32 = -1; $"n={n}"` | `n=-1` | R1 / AR-03 |
| ST-4 | `let n: i32 = 2147483647; $"n={n}"` | `n=2147483647` | R1 / AR-03 |
| ST-5 | `let n: i32 = -2147483647 - 1; $"n={n}"` | `n=-2147483648` | R1 / AR-03 |
| ST-6 | `let n: i32 = -2147483647; $"n={n}"` | `n=-2147483647` | R1 / AR-03 |
| ST-7 | `let n: u32 = 0; $"n={n}"` | `n=0` | R1 / AR-03 |
| ST-8 | `let n: u32 = 0xFFFFFFFF; $"n={n}"` | `n=4294967295` | R1 / AR-03 |
| ST-9 | `let n: u32 = 2147483648; $"n={n}"` | `n=2147483648` | R1 / AR-03 |
| ST-10 | `let n: usize = 0xFFFFFFFF; $"n={n}"` | `n=4294967295` | R1 / AR-03 |
| ST-11 | `let a: i8 = -128; let b: u8 = 255; $"{a} {b}"` | `-128 255` | R1 / AR-03 |
| ST-12 | `toDecimal(-42)` | `-42` | R1 / R3 |
| ST-13 | `let a=1i32; let b=2u32; let c=3i32; $"{a}{b}{c}"` | `123` (adjacent, no separators) | R1 / AR-08 |
| ST-14 | `$"x={a},y={b}"` with `a=-7`, `b=9` | `x=-7,y=9` | R1 / AR-08 |
| ST-15 | `let s: str = ""; $"[{s}]"` | `[]` | R3 / AR-08 |
| ST-16 | `let s: String = stringOf("abc"); let t: String = $"[{s}]"; println($"{s}{t}")` | `abc[abc]` (source still usable) | R3 / AR-08 |
| ST-17 | `$"{true} {false}"` | `true false` | R3 / AR-02 |
| ST-18 | `$"{1.5} {0.000123456789} {1234567.0}"` | `1.5 0.000123457 1.23457e+06` | R3 / AR-02 |
| ST-19 | `toHex(0x3F9A6)` | `0x3F9A6` | R3 / AR-02 |

### Integration and performance contract

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-20 | The `strings` loop shape: 2,000 iterations of `$"token-{i}-{i * u32(3)}"`, checksum `len + first byte` | `strings: 0x3F9A6` | R1 / R5 |
| ST-21 | A program formats `INT_MIN`, `UINT_MAX`, `toDecimal(INT_MIN)`, and `toDecimal(7)`, then prints the number of `snprintf` calls through a wrapped libc symbol | `hex=0x0` (zero formatter calls) | R1 / AR-01 |
| ST-22 | The ST-21 program built with AddressSanitizer and leak detection | exit 0, no `AddressSanitizer` report | R6 / AR-14 |
| ST-23 | The ST-5 and ST-8 program built with UndefinedBehaviorSanitizer and `-fno-sanitize-recover=all` | exit 0, no UBSan report | R6 / AR-14 |
| ST-24 | A loop with an owned `String` placeholder `$"[{s}]"` and the source reused; ASan build | exit 0, no leak or double free | R3 / AR-14 |

> **Red-phase note:** ST-1–ST-20 and ST-22–ST-24 describe behavior that is
> already correct on the base compiler, so they pass before the change; that
> is recorded as an intentional no-change oracle. ST-21 is the red case: the
> base formats through `snprintf` and prints a nonzero count. The
> implementation suite is written after the implementation (specification-first
> ordering) and pins the new helper emission.

## Test Categories

### Specification Tests

| Test file | ST cases | Component |
|---|---|---|
| `compiler/tests/string_integer_formatting_spec_test.rs` | ST-1 – ST-24 | Decimal formatter and converters |

The suite emits host C with `--profile host`, builds it with
`-std=c99 -O0 -g -Wall -Wextra -Werror`, and runs it. It resolves `RDC_CLC`
against the workspace root when the variable is set (for the pre-change
comparison) and otherwise uses the built `clc`. ST-21 links a stub with
`-Wl,--wrap=snprintf`; the wrapper forwards to `__real_snprintf` (through
`vsnprintf`) and increments a counter exposed by
`cl_test_snprintf_calls()`. ST-22/ST-24 add
`-fsanitize=address -fno-omit-frame-pointer -no-pie` and
`ASAN_OPTIONS=detect_leaks=1:symbolize=0`; ST-23 adds
`-fsanitize=undefined -fno-sanitize-recover=all -no-pie`.

### Implementation Tests

| Test file | Description | Priority |
|---|---|---|
| `compiler/tests/string_integer_formatting_impl_test.rs` | Host emission contains `cl_format_i32`/`cl_format_u32` in the converter bodies and no `snprintf(buffer` integer format; the float converter still uses `snprintf`; the Amiga emission contains the same helpers; literal-only templates still emit `cl_str(...)`; determinism | High |

### Integration Tests

| Test | Components | Description |
|---|---|---|
| `make bench` / `make bench-compare` | Compiler + harness | Scored outputs and the 1% guards |
| `make differential` | Host + m68k | Output equality for the portable corpus |
| `make bench-shape` | GCC, vbcc, freestanding | Strict-C build legs |

### End-to-End Tests

| Scenario | Steps | Expected result |
|---|---|---|
| Full acceptance | `make verify` | All legs green |

## Test Data

- No new fixtures. The `strings` benchmark is the workload; its source and
  expected output are unchanged.
- The ST-21 stub is a short C file written into the test's scratch directory
  (not committed).

## Red/Green Record

| Phase | Evidence |
|---|---|
| Red | ST-21 prints a nonzero count on `8f07e40` |
| Green | ST-21 prints `hex=0x0`; the impl suite (written after the implementation) passes |
| No-change oracle | ST-1–ST-20 and ST-22–ST-24 pass on both compilers |

## Verification Checklist

- [ ] Every ST case has a concrete input and expected output.
- [ ] Every ST case traces to a requirement, spec, or AR entry.
- [ ] Specification tests were written before the implementation.
- [ ] ST-21 is red on the base and green after the change.
- [ ] The impl suite is red on the base and green after the change.
- [ ] All host tests pass.
- [ ] ASan and UBSan legs pass.
- [ ] The benchmark outputs match and no scored metric regresses above 1%.
- [ ] `make check`, `make bench-shape`, and `make verify` pass.
