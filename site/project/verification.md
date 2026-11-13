---
title: Verification
description: "How Clorraine is verified: specification tests, strict C builds, sanitizers, fuzzing, host-versus-m68k differentials, and Amiga acceptance gates."
---

# Verification

Clorraine is an engineering project, so the evidence matters more than the
claim. This page lists what is checked and what the checks prove.

The project does **not** claim formal verification, cycle-accurate performance,
or complete AmigaOS coverage.

## The gates

| Gate | What it does |
|---|---|
| `./spec/check.sh` | Validates the frozen specification, grammar, diagnostic registry, and golden data |
| `make check` | The fast gate: spec check, SDK registry, installer tests, Rust workspace tests, benchmark harness tests, fuzzer tests, and fuzz smoke |
| `make asan` | Runs every suite that references `RDC_ASAN` under AddressSanitizer, one suite per process |
| `make ubsan` | Runs the undefined-behaviour suites under UBSan and proves the leg is active with a known-overflow probe |
| `make fuzz-test` | Fuzzer unit tests |
| `make fuzz-smoke` | A bounded deterministic differential fuzzer matrix: committed seeds plus a generated range, on host, sanitizers, and m68k |
| `make differential` | Compares host and `hosted-amiga` stdout for the portable corpus programs |
| `make corpus-amiga` | Builds and runs the portable corpus on `hosted-amiga` with m68k GCC and `vamos`; builds the FS-UAE-only programs; vbcc build-only leg |
| `make corpus-fs-uae` | Boots the FS-UAE-only programs and diffs their captured output |
| `make run-fault-amiga` | Runs the six runtime-fault programs under `vamos` and asserts the fault line and a nonzero exit |
| `make bench-shape` | Builds the generated-C shape fixture with m68k GCC, vbcc, and the freestanding profile |
| `make bench-compare` | Enforces the benchmark invariants and the 1% no-regression guard |
| `make verify` | The full acceptance run: the fast gate plus UBSan, benchmark shape, the Amiga SDK gates, differential, FS-UAE corpus, and runtime faults |
| `make vscode-check` | Type-checks, tests, packages, audits, and smoke-tests the VS Code extension |

## Specification tests are the oracle

The compiler is checked against the frozen specification, not the other way
around. A failing specification test means the implementation is wrong. The
tests are not adjusted to match the implementation.

| Suite | Purpose |
|---|---|
| `stage` | Parsing, resolution, and semantic stages |
| `compile_fail` | Every diagnosed error has a golden case |
| `codegen_run` | Generated C compiles and the program produces the expected output |
| `run_fault` | Runtime traps and fault lines |
| `determinism` | The same input produces the same C and the same result |
| `corpus` | Real programs, compared on host and target |
| `graphics` | Amiga display programs |

Feature suites follow the same split: specification tests derive from approved
requirements, while implementation tests cover internals and pinned emission
shapes.

## Sanitizers and fuzzing

- **AddressSanitizer** catches leaks, use-after-free, and double-free in the
  host compiler and generated programs. The SDK image may need non-PIE links;
  `make asan` handles that automatically.
- **UndefinedBehaviorSanitizer** catches signed overflow, invalid shifts, and
  related undefined behaviour. The leg proves it is active before trusting a
  green run.
- **The fuzzer** is a SHA-256-seeded typed template generator. It compares host
  and m68k results and checks committed seed goldens. A shared wrong value is
  caught by the goldens; a divergence is caught by the differential.
- **Host-versus-m68k differential runs** compare stdout for the same program on
  both targets. Endianness, pointer width, and runtime differences show up here.

## Amiga acceptance

The Amiga gates are not screenshots of a local build. Each gate:

1. Builds a program with the pinned m68k toolchain and the selected profile.
2. Boots FS-UAE with the configured machine and media.
3. Captures the app's native screen or output line.
4. Compares it to a committed golden.

The A500 gates use Kickstart 1.3 and OCS. The A600, A1200, and A4000 gates use
the appropriate Kickstart and optional hardware. The advanced Workbench gates
need licensed media supplied by the person running them.

## Continuous integration

The public GitHub Actions workflow is **host-only**. It runs `./spec/check.sh`
and the Rust workspace tests. Tests that need the m68k toolchain, `vamos`,
FS-UAE, an X display, or licensed media skip themselves when those are absent.

`make verify` remains the authoritative local acceptance gate. A green CI run is
not a substitute for it.

## Performance

`make bench` records generated-C size, m68k section sizes, host time, and a
streamed `vamos` instruction count. `make bench-compare` checks the invariants
and a 1% no-regression guard.

Instruction counts are guest CPU instructions, not bus cycles. They are useful
for deterministic comparisons, not for claiming hardware performance.

## Reproduce the fast gate

From a checkout:

```sh
make check
```

From a machine with the SDK, emulators, X display, and licensed media:

```sh
make verify
```

`make verify` prints `verify: all legs passed` only when every leg succeeds.

## Read the evidence

- [Specification](https://github.com/OpenLorraine/clorraine/tree/main/spec)
- [Test drivers](https://github.com/OpenLorraine/clorraine/tree/main/compiler/tests)
- [Fuzzer](https://github.com/OpenLorraine/clorraine/tree/main/tools/fuzz)
- [Benchmark harness](https://github.com/OpenLorraine/clorraine/tree/main/tools/perf)
- [CI workflow](https://github.com/OpenLorraine/clorraine/blob/main/.github/workflows/check.yml)
- [Known limitations](/guide/known-limitations)
