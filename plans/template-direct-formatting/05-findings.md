# Findings: Direct Template Formatting

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

## 1. Base Identity

| Item | Value |
|---|---|
| Base commit | `00eb6bc` (shim-only formatter) |
| Compiler | `clc 1.1.2` |
| Base metrics | pending (Phase 1) |
| Worktree | clean |

## 2. Probe

The pre-plan probe is owned by `02-current-state.md` §Pre-Plan Probe. In one
line: `strings` 1,132,051 net instructions and 2,002 allocations, versus
1,489,187 and 6,003 on the shim-only base.

## 3. Red/Green

| Case | Base result | Head result | Note |
|---|---|---|---|
| ST-1 allocation delta | pending | pending | expected 3 on base, 1 on head |
| ST-2/ST-3/ST-4 deltas | pending | pending | 2 on base, 1 on head |
| ST-5 float delta | pending | pending | 2 on both |
| Impl pins | pending | pending | written after the implementation |
| No-change cases | pending | pending | ST-6 – ST-8, ST-10, ST-11, ST-13, ST-15 |

## 4. Final Measurement

| Benchmark | Base instructions | Head | Delta | Base `.text` | Head | Delta | Output |
|---|---:|---:|---:|---:|---:|---:|---|
| sieve | pending | pending | pending | pending | pending | pending | pending |
| matrix | pending | pending | pending | pending | pending | pending | pending |
| strings | pending | pending | pending | pending | pending | pending | pending |
| moves | pending | pending | pending | pending | pending | pending | pending |
| lookup | pending | pending | pending | pending | pending | pending | pending |

Allocations on `strings`: pending.

## 5. Sanitizers, Profiles, and Emission

| Leg | Result |
|---|---|
| New suites, plain `cargo test` | pending |
| `make asan` | pending |
| `make ubsan` | pending |
| `make bench-shape` | pending |
| Target boundary under `vamos` | pending |
| Base/head emitted-C diff | pending |
| `make check` | pending |
| `make verify` | pending |

## 6. Reviews

| Review | Verdict | Findings |
|---|---|---|
| Correctness | pending | pending |
| Memory safety | pending | pending |
| Emission integrity | pending | pending |

## 7. Closeout

- Issues #25 and #19: pending.
- Roadmap: pending.
- Residual: the float placeholder keeps one `%g` temporary; every other
  placeholder is allocation-free.
