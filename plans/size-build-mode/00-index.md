# Opt-in Size Build Mode (PLAN-37)

> **Feature**: Deliver issue #21 — add an opt-in `--size` build mode that
> compiles the emitted C with `-Os` while leaving the default `--release`
> (`-O2`) line unchanged. No language, grammar, ABI, standard-library, or
> codegen change.
> **Status**: Done — 24/24; full gate green
> **Created**: 2026-11-13
> **Last Updated**: 2026-11-13
> **Base**: `e65de1c` (clean `main == origin/main`, `clc 1.1.2`)
> **Implements**: R1–R14
> **CodeOps Artifact Schema**: 1

**Original goal:** deliver issue #21 — make the deliberate size-for-runtime
trade available only when the developer asks for it, keep `-O2` as the default
release line, and record a per-profile comparison.

## Overview

PLAN-26 measured `-Os` on the same emitted C, before the shipped release line
had section GC, and found a large code-size win (calibration `.text` −8.7%,
sieve −8.8%) but a +13.1% dynamic instruction count on the then-current
`strings` benchmark. Re-measurement on the delivered line shows the retained
m68k `.text` is near-neutral after GC, host `.text` falls by up to about 12%,
and runtime is workload-dependent (`moves` +31.3% net is the worst case).
Because the no-regression rule applies to every scored benchmark, `-O2` stays
the default. Issue #21 asks for a public, opt-in mode instead: a developer who
values artifact size over speed can choose it deliberately.

The change is a driver surface only. `clc` still emits the same C; the
`clorraine` driver selects `-Os` in place of `-O2` and keeps the existing
release processing (function/data sections, `--gc-sections`, strip) where the
profile has it. Debug builds, the default release line, and the scored
benchmark harness are untouched.

## Minimum-Sufficient Baseline

**Smallest viable design:** add one build-mode value to the existing driver
option model, add one `-Os` flag slice per profile to the existing `Tool`
table, and let the existing `command()` function select the slice. No new
module, pass, IR node, runtime helper, profile, environment variable, or
dependency.

**Excluded machinery:** a second release pipeline, per-function flags, link
scripts, LTO, `-Oz`, automatic size selection, and any change to the frozen
compile matrix or the scored harness.

**Approved complexity:** one enum, one flag per profile, one CLI option, and
one help/documentation entry.

## Document Index

| # | Document | Description |
|---|---|---|
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Functional, measurement, scope, and acceptance requirements |
| 02 | [Current State](02-current-state.md) | Current driver flags, the issue evidence, and the base behaviour |
| 03 | [Design](03-design.md) | Build-mode model, per-profile flags, CLI rules, exclusions |
| 05 | [Findings](05-findings.md) | Measurements, decisions, tests, reviews, acceptance |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec, unit, and end-to-end cases with red/green order |
| 99 | [Execution Plan](99-execution-plan.md) | Phases and task checkboxes |

## Decisions

| Decision | Value |
|---|---|
| Base commit | `e65de1c`; assert the full hash and `dirty == false` before the gate |
| Flag | `--size` (long form only); it is a build mode, not a C compiler flag passthrough |
| Semantics | `--size` implies release processing and selects `-Os`; it works without `--release` |
| Conflict | `--release --size` is a usage error; repeated `--size` is accepted |
| `emit` | `--size` is rejected, like `--release`: the emitted C does not depend on the mode |
| Default | Debug (`-O0 -g`, warnings as errors) and `--release` (`-O2`) are unchanged |
| Profiles | All four profiles get a size line, including the freestanding m68k and host-run lines |
| Scored harness | Unchanged; `tools/perf/bench.py` continues to measure the shipped `-O2` line |
| Spec | No `spec/` change; the frozen compile matrix still defines the required release line |
