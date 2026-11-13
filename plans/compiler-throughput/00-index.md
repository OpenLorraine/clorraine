# Compiler Throughput (PLAN-38)

> **Feature**: Deliver issue #13 — measure `clc` wall time, peak memory, and hot
> compiler passes on the specification corpus, the benchmarks, large real
> programs, and generated inputs; land a measured improvement only where the
> measurement shows a bottleneck; keep generated code unchanged.
> **Status**: Done — 25/25; full gate green
> **Created**: 2026-11-13
> **Last Updated**: 2026-11-13
> **Base**: `9c44537` (clean `main == origin/main`, `clc 1.1.2`)
> **Implements**: R1–R12
> **CodeOps Artifact Schema**: 1

**Original goal:** deliver issue #13 — establish a compiler-throughput baseline,
find the hot passes, and either land a measured improvement or record a
documented no-op without trading generated-code quality.

## Overview

Generated-code runtime and size are one axis; compiler throughput is another.
The recent performance work measured the first axis only. This feature adds a
repeatable throughput harness, records the baseline, and fixes the one
quadratic path the measurement exposed: the C emitter looked up each lowered
body by scanning every body, once per prototype and once per emitted function.
A program with thousands of generated functions therefore spent most of its
compile time in code generation.

The fix builds one index of lowered bodies and reuses it. Emitted C is
byte-identical. Hand-written programs stay in the same few-millisecond range;
generated programs with thousands of functions compile several times faster.
The remaining hot pass on mixed generated input is ownership checking, which
is linear and acceptable at the measured scale, so it is recorded rather than
changed.

## Minimum-Sufficient Baseline

**Smallest viable design:** add one measurement script under the existing
`tools/perf/` tree, one `make` target, and one lookup index in the existing
emitter. No new dependency, compiler pass, IR node, profile, build flag, or
runtime helper.

**Excluded machinery:** parallel compilation, incremental caching, a daemon,
LTO/PGO, allocator replacement, arena rewriting, and any change to the scored
`-O2` line or the frozen specification.

## Document Index

| # | Document | Description |
|---|---|---|
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Measurement, optimization, scope, and acceptance requirements |
| 02 | [Current State](02-current-state.md) | Current pipeline, the quadratic scans, and the base timings |
| 03 | [Design](03-design.md) | Harness shape, body-index design, safety argument, rejected alternatives |
| 05 | [Findings](05-findings.md) | Baseline, hot passes, improvement, memory, tests, reviews, acceptance |
| 07 | [Testing Strategy](07-testing-strategy.md) | Harness, index, emission-identity, and gate cases |
| 99 | [Execution Plan](99-execution-plan.md) | Phases and task checkboxes |

## Decisions

| Decision | Value |
|---|---|
| Base commit | `9c44537`; assert the full hash and a clean tree before the gate |
| Harness | `tools/perf/compile_time.py` with `run` and `compare`; JSON under `build/perf/` |
| Make target | `make bench-compile` measures the release `clc` and writes `build/perf/compile-head.json` |
| Workloads | 130 spec files, 7 benchmarks, 6 large Amiga examples, 8 generated shapes |
| Metric | Median wall time, with minimum recorded for shared-machine checks; peak RSS from `wait4` |
| RSS caveat | A forked child inherits a memory floor; the harness records it and findings compare large workloads |
| Optimization | Index lowered bodies once; replace both linear scans in the emitter |
| Output | Emitted C, diagnostics, exit codes, and the scored benchmark rows stay identical |
| Remaining hot pass | Ownership checking is linear at the measured sizes; documented, not optimized |
| Spec | No `spec/` change; no language, ABI, runtime, stdlib, or driver-flag change |
