# Nested Fill Compaction (PLAN-22)

> **Feature**: A measured follow-up to the generated-C optimization programme: parameterize the benchmark comparison gate, then compact a nested constant-zero fixed-array fill into one `memset`.
> **Status**: Requirements and plan preflights clean; execution ready
> **Created**: 2026-11-08
> **Last Updated**: 2026-11-08
> **Base commit**: `b68dae5`
> **Implements**: R1–R7
> **CodeOps Artifact Schema**: 1

**Original goal:** continue the measurement-first generated-C performance
closeout: use the existing harness to find the next hot shape, implement only
what the measurements justify, prove the win, and guard against regressions.

## Why This Shape

The audit in [Current State](02-current-state.md) finds one emitted shape with
clear in-scope headroom: a nested constant-zero fixed-array fill. The matrix
benchmark currently pays a 32-slot row temporary, 32 scalar stores, and 32
`memcpy` calls for `[; [; 0]]`. One `memset` over the covered rows is
byte-equivalent and measures:

| Metric | Base | Head | Delta |
|---|---:|---:|---:|
| matrix net instructions | 10,959 | 9,908 | -9.6% |
| matrix m68k `-O2` `.text` | 80,712 | 76,676 | -5.0% |
| matrix emitted C bytes | 315,167 | 312,510 | -0.8% |

The other benchmarks are recorded as saturated or out of scope: sieve already
runs at 4-5 instructions per loop iteration with the bounds guards removed by
the C compiler; strings is dominated by stdlib and libnix allocation; moves
needs a calling-convention change.

The audit also finds that `make bench-compare` hardcodes PLAN-14's sieve
targets and a generic improvement rule, so it rejects both a flat baseline and
this smaller win. The gate is parameterized first, with the no-regression rule
kept intact.

## Minimum-Sufficient Baseline

**Smallest viable design:** one comparator option and its tests, one local
codegen rule in the existing nested-fill branch, two focused test suites, the
benchmark evidence, the required reviews, and the documentation closeout.

No new crate, no build script, no benchmark source change, no language or ABI
change, no runtime-shim or stdlib change, and no new measurement tool.

**Excluded machinery:** a general constant-propagation or range-analysis pass,
float fill folding, struct-copy ABI elision, a new benchmark, host-time gating,
and any change to the scored programs or their expected output.

## Document Index

| # | Document | Description |
|---|---|---|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Every material decision; all resolved |
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Functional, non-functional, and acceptance requirements |
| 02 | [Current State](02-current-state.md) | Repository state, harness audit, measured attribution, gate defect |
| 03-01 | [Comparison Gate](03-01-comparison-gate.md) | Parameterized `--require` design and tests |
| 03-02 | [Nested Fill Compaction](03-02-nested-fill-compaction.md) | Emission rule, eligibility, safety, threshold |
| 05 | [Findings](05-findings.md) | Baseline/head evidence, red/green record, reviews, acceptance |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec cases, impl pins, red-first, sanitizers, gates |
| PF-R | [Requirements Preflight](00-preflight-report-requirements.md) | Adversarial requirements audit and rulings |
| PF-P | [Plan Preflight](00-preflight-report.md) | Adversarial plan audit and rulings |
| 99 | [Execution Plan](99-execution-plan.md) | Phases and tasks |

## Decisions

| Decision | Value |
|---|---|
| Base commit | `b68dae5`; plan docs committed before the baseline |
| Push cadence | Commit at the seven phase boundaries; push after the final full gate |
| Comparison requirements | Provisional `matrix:instructions:5 matrix:text:3`; re-derived in Phase 1 |
| Threshold | 64 covered leaf slots (conservative floor) |
| Float leaves | Excluded in this cut |
| Execution mode | `--auto-commit --auto-design`; pushes held to the final gate |
| Branch | `main`; work in a clean worktree |

## Specialist Agents

| Role | Agent | Phase |
|---|---|---|
| Correctness reviewer | `correctness-reviewer` | Tooling and codegen |
| Memory-safety auditor | `memory-safety-auditor` | Codegen (read-only ASan probes) |
| Emission-integrity auditor | `emission-integrity-auditor` | Codegen (read-only base/head emission diff) |
| Performance auditor | `performance-auditor` | Codegen (over-optimization and threshold judgment) |

Review packets include a blast-radius step and a test-integrity step. Critical
and major findings pause execution for a ruling.
