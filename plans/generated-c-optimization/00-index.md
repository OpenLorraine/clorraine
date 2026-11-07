# Generated-C Optimization (PLAN-14)

> **Feature**: A measurement-first optimization programme for the C the compiler emits: a deterministic benchmark harness, a representative benchmark suite, and only the generated-C changes that the measurements justify.
> **Status**: Executing — 2026-11-07
> **Created**: 2026-11-07
> **Last Updated**: 2026-11-07
> **Implements**: R1–R8
> **CodeOps Artifact Schema**: 1

**Original goal:** make the generated C measurably better on the m68k target
without changing what a Clorraine program means, using PLAN-14's
measure-first rule.

## Overview

PLAN-14 is the generated-C optimization row. Its trigger is now satisfied: the
open defect backlog is closed and the language and emitted ABI are frozen. The
row is deliberately a *programme*, not a fixed list of optimizations. It says:

- build the benchmark harness first;
- optimize only against a measured need, never on feel;
- guard every optimization with behavior, memory, and emission reviews;
- accept the cost that the shape-pinning implementation suites move with every
  emission change.

This plan therefore delivers two things in order. First, a committed,
deterministic harness and a small benchmark suite that measures host time, m68k
dynamic instruction count, emitted-C size, and linked code size. Second, the
generated-C changes the baseline actually justifies.

The baseline study found two clear, measured costs:

| Finding | Baseline evidence |
|---|---|
| A large fixed-array fill is emitted as one store per element | `u8[8192]` fill emits 8,192 statements; the sieve's C is 489,491 bytes; `.text` is 101,396 bytes at `-O2` and 152,116 bytes at `-O0` |
| A module `const` is emitted as a run-time `static` and read from memory | the sieve executes 3,044,035 m68k instructions; folding the constant alone drops it to 2,165,514 (−28.9%) and host `-O2` median time from about 31.0 ms to about 22.0 ms |

The study also rejected a plausible optimization. Removing three upper bounds
checks by hand made the debug host build slightly faster, but the release host
build and, after constant folding, the m68k code were *slower* (2,165,514 →
2,436,214 instructions). The measurement discipline exists to catch exactly
that kind of reversal. The plan keeps only the dead lower-bound comparison
removal for non-negative constant slice/string indices and lets the C compiler
eliminate the upper check after constant folding.

## Minimum-Sufficient Baseline

**Smallest viable design:** one Python-stdlib harness in `tools/perf/`, one
committed suite of four scored programs plus a calibration under
`benchmarks/`, and four codegen changes:

1. fold scalar module `const` reads in function bodies to typed literals,
   except when the const's address is taken;
2. compact large one-extent scalar fills into a loop (or `memset` for a
   `u8`/`i8` fill);
3. omit the provably dead `< 0` half of a slice/string constant index guard;
4. fold the division guard when the divisor is a constant.

No new crate, no build script, no external Python package, no emulator change,
no language change, no ABI change, and no storage-layout change.

**Excluded machinery:** a C-level optimizer pass, an SSA or dataflow framework,
a new benchmark CI service, FS-UAE cycle instrumentation, profile-guided
optimization, changes to the runtime shims, and read-only/`static const`
storage for module constants. Those are separate capabilities; none is needed
to prove this row.

**Approved complexity:** one tooling directory, one benchmark directory, one
small whole-program address-taken pre-pass, four local codegen rules, one
container base-build script, the `make bench*` targets, two new test suites,
and documentation. The work stays inside the existing compiler and
verification model.

## Document Index

| # | Document | Description |
|---|---|---|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Every material decision, including the auto-design delegated records |
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Functional, measurement, compatibility, and acceptance requirements |
| 02 | [Current State](02-current-state.md) | Baseline measurements and the evidence for each selected change |
| 03-01 | [Benchmark Harness](03-01-benchmark-harness.md) | Harness contract, metrics, JSON schema, comparison thresholds |
| 03-02 | [Generated-C Changes](03-02-generated-c-changes.md) | Each codegen rule, its invariants, and its rejected alternatives |
| 05 | [Findings](05-findings.md) | Base/head evidence, accepted limits, and rejected candidates |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec, implementation, harness, and benchmark evidence split |
| 00-PF-REQ | [Requirements Preflight](00-preflight-report-requirements.md) | Independent requirements audit and resolution |
| 00-PF-PLAN | [Plan Preflight](00-preflight-report.md) | Independent plan audit and resolution |
| 99 | [Execution Plan](99-execution-plan.md) | Task-by-task delivery and evidence |

## Specialist Agents

**None** — the plan uses the repository's existing correctness, memory-safety,
emission-integrity, performance, and semantics reviewers. No new standing
specialist closes a capability gap that those roles do not already cover.

_Detection evidence:_ the work is compiler codegen and measurement; the
existing `.opencode/agents/` roles cover correctness, ownership/memory,
emission integrity, performance, and semantics. There is no new external
system, provider, or regulated domain. The negative check is recorded in
`05-findings.md`.

## Related Files

| Path | Role |
|---|---|
| `tools/perf/bench.py` | Harness |
| `tools/perf/tests/test_bench.py` | Harness unit tests |
| `benchmarks/` | Scored programs and calibration |
| `docker/scripts/bench-base.sh` | Base compiler build/run |
| `compiler/src/own/program.rs`, `own/lower.rs`, `own/place.rs` | Const fold and address-taken pass |
| `compiler/src/consts/mod.rs` | Constant folder |
| `compiler/src/codegen/function.rs` | Fill compaction |
| `compiler/src/codegen/guards.rs`, `expr.rs`, `render.rs` | Guard and literal rendering |
| `compiler/tests/generated_c_optimization_spec_test.rs` | Behavior oracle |
| `compiler/tests/generated_c_optimization_impl_test.rs` | Emission pins |
| `Makefile` | `bench*` targets and `check` |
| `docs/known-limitations.md` | Measurement boundary |

## Scope Boundary

This plan changes what the emitter writes and how that output is measured. It
does not change:

- the grammar, the frozen specification, any golden input, or any diagnostic;
- the language's ownership, borrow, drop, evaluation-order, or trap rules;
- the C ABI of any exported function, handle, struct, view, or generated
  symbol;
- the runtime shims;
- the storage or initialization order of module bindings.

Emitted C must stay valid C99 for both `m68k-amigaos-gcc` and vbcc, and must
stay byte-identical for the same source, profile, and flags.
