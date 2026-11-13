# Constant-Array Codegen Residuals (PLAN-36)

> **Feature**: Close issue #18's measured residuals: emit the constant-array
> data template for function-local tables, re-measure the 1-byte and 2–5 KB
> decisions, and record the short-list tail policy. No language, grammar, ABI,
> or standard-library change.
> **Status**: Done — 31/31; full gate green
> **Created**: 2026-11-13
> **Last Updated**: 2026-11-13
> **Base**: `7071761` (clean `main == origin/main`, `clc 1.1.2`)
> **Implements**: R1–R14
> **CodeOps Artifact Schema**: 1

**Original goal:** deliver issue #18 — measure each constant-array residual
before changing it, land only wins, keep the scored guard green, and keep the
accepted limits current.

## Overview

PLAN-25 (constant-array initializer) emits one read-only `static const`
template plus a `memcpy` for an eligible **module-level** constant scalar
array. It left four measured residuals:

1. **Function-local tables** use the store form because the template rule only
   runs from the module initializer. The in-repo `cube` example uses a
   256-entry function-local sine table.
2. **1-byte tables** keep stores because the measured template cost is above
   the 1% instruction guard.
3. **The 2–5 KB 4-byte band** was code-size-only at `i32[1024]` in the
   PLAN-25 measurement (about +63 instructions, +1.18% in an init-dominated
   run, for −6,148 `.text`).
4. **Short/empty lists** keep stores because an extent-sized template would
   zero-fill a tail the stores leave at the existing state; the tail policy
   was not measured.

This plan measures each item on a common probe set, lands the function-local
template (the only strict win), keeps the stores for 1-byte and short lists,
and keeps the template in the 2–5 KB band after re-measurement. It adds a
committed `local-table` benchmark so the new shape has a scored guard.

## Minimum-Sufficient Baseline

**Smallest viable design:** factor the existing template predicate and data
emission into one shared helper, call it from the top-level fixed-array
assignment path before the target is rendered, and copy with
`memcpy(target.address, name, target.bytes)`. No new module, pass, IR node,
runtime helper, profile, or language surface.

**Excluded machinery:** module `const` arrays (`E10224`, a checker/language
change), a store-packing-aware 1-byte rule, a size-aware band cap, a
freestanding template, and any change to the scored default flags.

**Approved complexity:** none. The change reuses the existing predicate and
emission and adds one call site plus tests.

## Document Index

| # | Document | Description |
|---|---|---|
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Functional, measurement, scope, and acceptance requirements |
| 02 | [Current State](02-current-state.md) | Current rule, baseline probes, and measured residuals |
| 03 | [Design](03-design.md) | Shared helper, call site, safety argument, exclusions |
| 05 | [Findings](05-findings.md) | Measurements, decisions, tests, reviews, acceptance |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec and impl cases, red/green order, gate coverage |
| 99 | [Execution Plan](99-execution-plan.md) | Phases and task checkboxes |

## Decisions

| Decision | Value |
|---|---|
| Base commit | `7071761`; assert the full hash and `dirty == false` before the gate |
| Function-local template | **Land.** Measured −1,004 `.text` / −260 net instructions at `i32[256]`, −4,036 / −1,045 at `i32[1024]`, −5,256 / −1,672 at `i32[1280]` |
| 1-byte tables | **Keep stores.** Re-measured `u8[256]` template: −336 `.text` but +211 net instructions; the instruction cost is the guard |
| 2–5 KB 4-byte band | **Keep the template.** Re-measured `i32[1024]`: −6,044 `.text` for +41 net instructions (+0.4% in a scan workload); `i32[1280]` wins both |
| Short/empty lists | **Keep stores.** The declaration zero-fills the tail at function entry; a loop re-entry can leave prior values. Recorded as a measured limit, not changed in a performance bundle |
| Module `const` arrays | **Out of scope.** `E10224` is a checker/language change and needs its own scope ruling |
| Freestanding | **Keep stores.** The runtime `memcpy` is a byte loop; no measured win |
| Benchmark | Add `local-table` (256-entry function-local `i32` sine table) to the scored suite; commit its baseline in `05-findings.md` |
| Spec | No `spec/` change; the emitted C stays valid C99 and the behavior is unchanged |
