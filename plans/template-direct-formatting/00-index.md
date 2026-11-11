# Direct Template Formatting

> **Feature**: Lower template placeholders into typed builder parts so integer
> and bool placeholders are formatted directly into the destination, removing
> the temporary `String` allocations; `String`/`str` copy for safety.
> **Status**: Done — 26/26; full gate green
> **Created**: 2026-11-11
> **Implements**: R1–R7
> **CodeOps Artifact Schema**: 1

**Original goal:** complete issue #19 by removing the measured allocation churn
that the shim-only formatter left in place, as tracked by issue #25, with no
language or ABI change and the full emission, sanitizer, and benchmark gate.

## Overview

The shim-only delivery removed the `snprintf` cost but kept three allocations
per `strings` iteration: two converter `String`s and the builder result. The
measured next step (issue #25) is to carry a scalar placeholder as a value in
a 12-byte template part and write it directly into the destination, and to
copy `String`/`str` placeholders when the part is built, before a sibling
placeholder can release the source.

The pre-plan probe measured `strings` at 1,132,051 net instructions (from
1,489,187) and 2,003 allocations (from 6,003), with one result allocation
for integer/bool placeholders. The change touches
`compiler/src/codegen/expr.rs::template` and the `ClStrPart` builder in the two
`String` shims. It changes the emitted C for every template, so it needs the
emission-integrity auditor in addition to the correctness and memory-safety
reviews.

## Minimum-Sufficient Baseline

**Original goal:** remove the two per-iteration converter allocations without
changing template behavior or the language ABI.
**Smallest viable design:** extend the existing `ClStrPart` from
`{ptr, len, owned}` to a 12-byte union part with two scalar kind codes, and
emit scalar placeholders as values, `str`/`String` placeholders as owned
copies, and floats as the existing `%g` temporary. One builder call, one
result allocation.
**Excluded machinery:** a new builder API, a general formatting framework, a
scratch allocator/pool, a new benchmark harness, and any language change.
**Approved complexity:** None — the plan reuses the existing builder, the
existing converters, and the existing benchmark.

## Document Index

| # | Document | Description |
|---|---|---|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions |
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Requirements and acceptance |
| 02 | [Current State](02-current-state.md) | Current lowering and probe audit |
| 03 | [Template Lowering](03-template-lowering.md) | Technical specification |
| 05 | [Findings](05-findings.md) | Evidence record |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec cases and verification |
| 99 | [Execution Plan](99-execution-plan.md) | Tasks and phases |

## Key Decisions

| Decision | Outcome |
|---|---|
| Scope | Implement #25; close #25 and #19 after the gate (AR-01, AR-11) |
| Part shape | 12-byte union part, kind codes 0/1 (AR-02) |
| String/str placeholders | Copy at part construction (AR-03, AR-13) |
| Float placeholder | Existing `%g` temporary, two allocations (AR-04) |
| Capacity | Max-width reserve, then `len = cap = written` (AR-05) |
| Reviews | Correctness + memory-safety + emission-integrity (AR-09) |

## Specialist Agents

| Role | Kind | Use | AR Ref |
|---|---|---|---|
| Correctness reviewer | reviewer | Implementation phase | AR-09 |
| Memory-safety auditor | read-only auditor | ASan/UBSan and lifetime probes | AR-09 |
| Emission-integrity auditor | read-only auditor | Base/head C diff and strict builds | AR-09 |

_Detection evidence: `AGENTS.md` phase-review dispatch for codegen/emission
changes. No semantics reviewer: no language-rule change (AR-06, AR-09)._

## Related Files

| File | Role |
|---|---|
| `compiler/src/codegen/expr.rs` | `template()` lowering |
| `compiler/runtime/shim_host.c` | Host `ClStrPart` and builder |
| `compiler/runtime/shim_hosted_amiga.c` | Amiga `ClStrPart` and builder |
| `compiler/tests/template_direct_formatting_spec_test.rs` | Behavior and allocation suite |
| `compiler/tests/template_direct_formatting_impl_test.rs` | Emission suite |
| `benchmarks/strings/program.clo` | Workload (unchanged) |
