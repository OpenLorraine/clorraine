# Findings: String Integer Formatting Throughput

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

This document is the evidence record. It is filled in phases: the pre-plan
probe in Phase 1, the red/green record in Phase 2–3, and the final base/head
measurement in Phase 4. Values are never estimated; a missing value is
`pending`.

## 1. Base Identity

| Item | Value |
|---|---|
| Base commit | `8f07e408975d9ad090392712030d2e2761d19437` |
| Compiler version | `clc 1.1.2` |
| Base compiler SHA-256 | `232698a3dc89e8b96b9d51a7258542830d0d1c18a6f262366494c449c3b8f295` |
| Pre-change head | `828453a854341634f348f8a12def879fd96e27f8`, SHA-256 `7b7ab88b7b4ae65dddd26fe3b1e6d411fcf73c9c4d1c368a4ac711476362f3bc` |
| Worktree | clean for both measured runs; harness `dirty=false` |
| Toolchain | pinned SDK container; m68k GCC 16.2.0b, libnix `nix20`, `vamos` 0.8.1 |
| Base document | `build/perf/base.json` (commit `8f07e40`, 30 runs) |
| Pre-change document | `build/perf/pre-change-head.json` (commit `828453a`) |
| Base vs pre-change compare | passed, every metric `+0.0%` (the source is unchanged) |
| `make check` on the plan commit | passed (2026-11-11 09:50; spec, registry, setup, workspace tests, bench-test, fuzz-test, fuzz-smoke 28/0) |

## 2. Pre-Plan Probe

The baseline, attribution, and probe tables are owned by
`02-current-state.md` §Measurement Audit and §Probes. In one line: the base is
2,964,051 net instructions with 6,003 allocations; the selected direct-write
shim probe is 1,464,668 (−50.6%), 24,080 `.text` (−0.5%), and 6,003
allocations; the out-of-scope codegen probe is 1,132,051 (−61.8%) with 2,002
allocations.

Commands and artifacts (the `build/perf/audit` scratch is removed at
closeout):

```
target/debug/clc --profile hosted-amiga -o build/perf/audit/strings.c benchmarks/strings/program.clo
m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 \
    -ffunction-sections -fdata-sections -Wl,--gc-sections \
    -o build/perf/audit/strings-shimonly2 build/perf/audit/strings-shimonly2.c -lamiga -lm
vamos --instr-trace build/perf/audit/strings-shimonly2 2> build/perf/audit/shimonly2.trace
m68k-amigaos-size build/perf/audit/strings-shimonly2
```

The allocation counter was a host build of the same emission with
`cl_alloc_calls`/`cl_free_calls` counters and an `atexit` report; it printed
`AUDIT alloc=6003 free=6003` for the base and `alloc=2002` for the codegen
probe.

## 3. Red/Green Record

| Case | Base result | Head result | Note |
|---|---|---|---|
| ST-21 (`snprintf` count) | pending | pending | the red case |
| ST-1 – ST-20, ST-22 – ST-24 | pending | pending | intentional no-change oracle |
| Impl pins (written after the implementation) | pending | pending | attributability check on the base |

## 4. Final Base/Head Measurement

Commands:

```
make bench-base COMMIT=8f07e40
make bench
make bench-compare
```

| Benchmark | Base instructions | Head | Delta | Base `.text` | Head | Delta | Output |
|---|---:|---:|---:|---:|---:|---:|---|
| sieve | pending | pending | pending | pending | pending | pending | pending |
| matrix | pending | pending | pending | pending | pending | pending | pending |
| strings | pending | pending | pending | pending | pending | pending | pending |
| moves | pending | pending | pending | pending | pending | pending | pending |
| lookup | pending | pending | pending | pending | pending | pending | pending |

Allocation count on `strings`: pending.

## 5. Sanitizers and Cross-Profile

| Leg | Result |
|---|---|
| New suite, plain `cargo test` | pending |
| New suite under `make asan` | pending |
| New suite under `make ubsan` | pending |
| `make bench-shape` (GCC, vbcc, freestanding) | pending |
| `make check` | pending |
| `make verify` | pending |

## 6. Review Outcomes

| Review | Verdict | Findings |
|---|---|---|
| Correctness reviewer | pending | pending |
| Memory-safety auditor | pending | pending |

## 7. Residual and Follow-Up

- Allocations per `strings` run stay at 6,003 in this plan (one builder plus
  two converter temporaries per iteration, three outside the loop).
- The codegen direct-to-destination lowering would remove two of the three
  per-iteration allocations and measure 1,132,051 net instructions on the
  probe. It is outside issue #19's scope (AR-01, AR-05) and is recorded as a
  follow-up issue at closeout (AR-16).
