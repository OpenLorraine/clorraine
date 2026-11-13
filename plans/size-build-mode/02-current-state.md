# Current State: Opt-in Size Build Mode (PLAN-37)

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## 1. Current Driver Model

`compiler/src/clorraine/cli.rs` parses `Options.release: bool`. The default is
false (debug). `parse_options` sets it from `--release`; `validate` rejects it
with `emit`.

`compiler/src/clorraine/toolchain.rs` owns one `Tool` value per profile:

| Field | Meaning |
|---|---|
| `program` | C compiler executable |
| `base` | flags shared by debug and release |
| `release` | release-only flags, including the optimization level |
| `libs` | link libraries |

`command()` chooses `tool.release` when `release` is true and `DEBUG_FLAGS`
otherwise; warnings are appended only in debug.

The current release vectors are:

| Profile | Release vector |
|---|---|
| host Linux | `cc -std=c99 -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o prog out.c` |
| host non-Linux | `cc -std=c99 -O2 -o prog out.c` |
| `hosted-amiga` | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o prog out.c -lamiga -lm` |
| `hosted-amiga-13` | the same line with `-mcrt=nix13` |
| freestanding m68k | `m68k-amigaos-gcc -std=c99 -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv -O2 -o prog out.c -lg` |
| freestanding host run | `cc -std=c99 -ffreestanding -fno-builtin [-nostdlib] -O2 -o prog out.c` |

## 2. Base Behaviour

At `e65de1c` the base driver prints the release help and rejects the new flag:

```
$ ./target/debug/clorraine --help
...
      --release                       Optimize (-O2); strip + section-GC on Linux/Amiga; freestanding/non-Linux and debug keep symbols
...

$ ./target/debug/clorraine build --size examples/hello.clo
usage error: unknown option '--size'
```

The base has no `-Os` path. `clc` emission is independent of the build mode, so
the emitted C for a given profile is identical before and after this feature.

## 3. Issue Evidence

PLAN-26 measured `-Os` on the same `clc` emission with the pinned toolchain:

| Program | `-O2` `.text` | `-Os` `.text` | Change |
|---|---:|---:|---:|
| calibration | 76,368 | 69,756 | −8.7% |
| sieve | 76,728 | 69,984 | −8.8% |

The allocation-heavy `strings` program executes 2,967,423 instructions at
`-O2` and 3,357,540 at `-Os`, a +13.1% runtime cost. That cost was above the
scored 1% guard when the issue was written, so `-O2` remains the default and
the size mode must be opt-in. These are historical issue numbers; the
delivered-line measurements are in [05-findings.md](05-findings.md) §4.

## 4. Why the Change Is Driver-Only

The optimization flag is a C compiler decision applied to the already emitted
translation unit. `clc` has no optimization-level input, and the generated C is
valid under both `-O2` and `-Os`. Keeping the change in the driver preserves
the compiler contract, the benchmark harness, and every scored result.

## 5. Existing Test Seams

- `compiler/src/clorraine/cli.rs` has unit tests for parsing, defaults, and
  rejection rules.
- `compiler/src/clorraine/toolchain.rs` has unit tests that pin every command
  vector.
- `compiler/tests/clorraine_driver_spec_test.rs` spawns the built driver and
  asserts help, build/run artifacts, exit codes, and the capture/handoff
  behaviour.

The feature extends those three seams; it needs no new test harness.
