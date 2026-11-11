# String Integer Formatting Throughput

> **Feature**: Replace `snprintf`-based integer formatting in the two `String`
> runtime shims with a direct decimal writer that fills the string block in
> place.
> **Status**: Done — 30/30; full gate green
> **Created**: 2026-11-11
> **Implements**: R1–R8
> **CodeOps Artifact Schema**: 1

**Original goal:** reduce the measured runtime cost of `String` integer
formatting in the standard library and runtime shims, with a proven benchmark
win, no scored regression above 1%, and no language, ABI, or emitted-C shape
change (issue #19).

## Overview

The generated-C performance work has one large runtime cost left: the
`strings` benchmark at 2,964,051 net m68k instructions. A per-function
instruction trace attributes 65.4% of that count to the `snprintf` family
(`vfprintf`, `putc`, libgcc division, `vsnprintf`) and only 11.8% to
`malloc`/`free`. Two integer placeholders per iteration each call
`cl_text_u32`, which formats through `snprintf("%u")` into a stack buffer and
then allocates and copies a `String`.

This feature keeps the issue's stated boundary — standard-library and
runtime-shim work only — and replaces the `snprintf` conversion with a
bounded decimal writer that fills the freshly allocated string block
directly. The formatter handles `INT_MIN` through an unsigned magnitude and
writes no more than 11 (`i32`) or 10 (`u32`) bytes plus the NUL. Float
formatting keeps `%g`, and every other converter is unchanged.

The pre-plan probe on the committed `strings` benchmark measures a 50.6%
net-instruction reduction (2,964,051 → 1,464,668), no allocation-count change
(6,003), and a `.text` reduction (24,196 → 24,080). Every other scored
benchmark improves or stays flat, all outputs are equal, and the probe's
strict host and m68k builds are clean. The residual allocation cost is
tracked as a follow-up issue rather than bundled (AR-01, AR-05, AR-16).

## Minimum-Sufficient Baseline

**Original goal:** remove the dominant measured cost of integer `String`
formatting inside the runtime shims.
**Smallest viable design:** rewrite `cl_text_i32` and `cl_text_u32` in
`compiler/runtime/shim_host.c` and
`compiler/runtime/shim_hosted_amiga.c` to allocate the maximum decimal width
once and write the digits and NUL into the block; no codegen, template, or
standard-library signature changes.
**Excluded machinery:** a codegen lowering that formats placeholders directly
into a destination buffer (measured further win, but outside issue #19's
scope; follow-up issue), a shared shim header, a custom allocator/pool, a
generalized formatting framework, and a new benchmark harness.
**Approved complexity:** None — the change reuses the existing shim
functions, block layout, and benchmark harness.

## Document Index

| #   | Document | Description |
| --- | --- | --- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md) | This document — overview and navigation |
| 01  | [Requirements](01-requirements.md) | Requirements, scope, and acceptance criteria |
| 02  | [Current State](02-current-state.md) | Measurement audit and current formatter analysis |
| 03  | [Runtime Formatting](03-runtime-formatting.md) | Technical specification for the shim change |
| 05  | [Findings](05-findings.md) | Probe evidence and final base/head measurements |
| 07  | [Testing Strategy](07-testing-strategy.md) | Specification test cases and verification |
| 99  | [Execution Plan](99-execution-plan.md) | Phases, sessions, and task checklist |

## Quick Reference

### Usage Examples

The feature changes no source syntax. Existing code keeps working and gets
faster:

```clorraine
let n: i32 = -2147483647 - 1;
println($"n={n}");                 // n=-2147483648
println(text(2147483647));          // 2147483647
println($"u={0xFFFFFFFFu}");       // u=4294967295
```

### Key Decisions

| Decision | Outcome |
| --- | --- |
| Scope boundary | Shim-only; codegen lowering is a follow-up (AR-01) |
| Integer conversion | Manual decimal writer, direct into the block (AR-02, AR-06) |
| Float conversion | Unchanged `%g` through `snprintf` (AR-02) |
| Allocation count | Unchanged in this plan; residual tracked (AR-05) |
| Verification | `make check`, sanitizers, `bench-shape`, `make verify` (AR-14, AR-17) |

## Specialist Agents

| Role | Kind | Use | AR Ref |
| --- | --- | --- | --- |
| Correctness reviewer | reviewer | Implementation phase review | AR-19 |
| Memory-safety auditor | read-only auditor | ASan probes and capacity review | AR-19 |

_Detection evidence: the change writes into a heap block from a fixed-width
decimal writer and does not touch ownership, drop, IR, codegen, symbol names,
or language rules (`AGENTS.md` phase-review dispatch). The emission-integrity
auditor and semantics reviewer are not dispatched for that reason._

## Related Files

| File | Role |
| --- | --- |
| `compiler/runtime/shim_host.c` | Host `String` block, converters, builder |
| `compiler/runtime/shim_hosted_amiga.c` | Amiga `String` block and converters |
| `compiler/tests/string_integer_formatting_spec_test.rs` | New behavior suite |
| `compiler/tests/string_integer_formatting_impl_test.rs` | New emission/implementation suite |
| `benchmarks/strings/program.clo` | Scored workload (unchanged) |
| `plans/string-integer-formatting/05-findings.md` | Evidence record |
