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
| Compiler SHA-256 | pending (Phase 1) |
| Worktree | clean (`git status --porcelain` empty) |
| Toolchain | pinned SDK container; m68k GCC 16.2.0b, libnix `nix20`, `vamos` 0.8.1 |
| Base document | `build/perf/base.json` (Phase 1) |

## 2. Pre-Plan Probe

The baseline, attribution, and probe tables are owned by
`02-current-state.md` §Measurement Audit and §Probes. In one line: the base is
2,964,051 net instructions with 6,003 allocations; the selected direct-write
shim probe is 1,464,668 (−50.6%), 24,080 `.text` (−0.5%), and 6,003
allocations; the out-of-scope codegen probe is 1,132,051 (−61.8%) with 2,002
allocations. The probe artifact path and the exact commands are recorded in
that document.

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
