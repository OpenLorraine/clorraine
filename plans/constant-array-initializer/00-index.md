# Constant Array Initialization (PLAN-25)

> **Feature**: A measured generated-C closeout for module-level constant scalar
> array initializers: emit one block-scope `static const` data template and one
> `memcpy` instead of one store per element.
> **Status**: Plan preflighted — ready for execution.
> **Created**: 2026-11-09
> **Last Updated**: 2026-11-09 21:42
> **Base commit**: `v1.1.0` for the cumulative benchmark comparison; current
> `main` for the isolated optimization comparison.
> **Implements**: R1–R6
> **CodeOps Artifact Schema**: 1

**Original goal:** continue the measurement-first generated-C performance
closeout: use the existing harness to find the next hot shape, implement only
what the measurements justify, prove the win, and guard against regressions.

## Why This Shape

The four existing benchmarks are saturated or out of scope after the
nested-fill and volatile-fill work:

| Benchmark | Current finding | Decision |
|---|---|---|
| sieve | 4–5 instructions per loop iteration; guards removed by the C compiler | Saturated |
| matrix | nested zero fill already compacted to one `memset` | Saturated |
| strings | dominated by standard-library and libnix allocation | Out of scope |
| moves | by-value struct copies need an internal ABI/calling-convention change | Out of scope (internal ABI) |

The audit finds one emitted shape with clear headroom: a module-level `let`
array with a complete explicit list of guard-free constant scalar elements.
The current emitter writes the empty state and then one store per element in
`cl_init_<module>`. For a 4,096-entry `i32` sine table that is 4,096 m68k
store instructions and a 24,464-byte `.text` reduction when they become one
read-only template plus a copy. The table data belongs in a read-only
template, and one `memcpy` at the same initialization point preserves every
observable behavior.

The exploratory probe (current tree, committed `benchmarks/lookup/program.clo`,
block-scope template, direct `vamos --instr-trace` and `m68k-amigaos-size`)
measures:

| Metric | Current | Template + `memcpy` | Delta |
|---|---:|---:|---:|
| `lookup` raw instructions | 17,025 | 12,842 | -24.6% |
| `lookup` m68k `.text` (`-O2`) | 117,440 | 92,976 | -20.8% |
| Output | `lookup: 134217728` | `lookup: 134217728` | equal |

The harness subtracts the 3,372-instruction calibration before scoring, so the
net percentage instruction delta is larger (-30.6%); the absolute delta is
unchanged. Phase 1 re-runs the measurement with the committed benchmark and
the harness.

## Minimum-Sufficient Baseline

**Smallest viable design:** one committed benchmark, one local rule in the
module-level global array initializer path, the existing constant evaluator and
`memcpy` spelling, two focused test suites, the shape fixture, the required
reviews, and the documentation closeout.

Eligibility is deliberately narrow: one extent, a 2- or 4-byte non-float
scalar leaf, a complete explicit list of guard-free constant elements, no
fill, at least 16 elements and 64 bytes under the profile’s type table, a
non-`volatile` module-level `let`, and the host, hosted-amiga, or
hosted-amiga-13 profile.
Everything else keeps its current spelling. The target
global stays writable and keeps its address and initialization point, so no
ABI, storage layout, module-order, or `unsafe` mutation behavior changes.

**Excluded machinery:** a static declaration initializer that skips the init
function (init-order and address-taken risk); linker section/garbage-collection
flags (a toolchain change, and the dead code is largely the out-of-scope
runtime shim); whole-program dead-function elimination; function-local constant
array literals; fill forms; short/empty lists; multi-extent arrays; 1-byte
scalar elements (`i8`/`u8`/`bool`, measured runtime regression); float, struct,
owned, and view elements; module-level `const` arrays (front-end `E10224`);
freestanding multi-byte templates (byte-wise `memcpy`); new crates; build
scripts; and any harness/schema change.

## Document Index

| # | Document | Description |
|---|---|---|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Every material decision, including the auto-design records |
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Functional, measurement, compatibility, and acceptance requirements |
| 02 | [Current State](02-current-state.md) | Repository state, harness audit, measured attribution, residual opportunities |
| 03 | [Design](03-design.md) | Emission rule, eligibility, template spelling, and tests |
| 05 | [Findings](05-findings.md) | Baseline/head evidence, red/green record, reviews, acceptance |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec cases, impl pins, red-first, sanitizers, gates |
| PF-R | [Requirements Preflight](00-preflight-report-requirements.md) | Adversarial requirements audit and rulings |
| PF-P | [Plan Preflight](00-preflight-report.md) | Adversarial plan audit and rulings |
| 99 | [Execution Plan](99-execution-plan.md) | Phases and tasks |

## Decisions

| Decision | Value |
|---|---|
| Base commit | `v1.1.0` for the cumulative comparison; clean pre-codegen head for the isolated comparison |
| Benchmark | `lookup`: 4,096-entry `i32` sine table (offset by 32768 to 1..65535), one scan, non-zero checksum |
| Eligible shape | Module-level `let`, one extent, 2- or 4-byte non-float scalar, complete explicit guard-free constant list, no fill, at least 16 elements and 64 bytes under the profile’s type table, not volatile, host/hosted-amiga/hosted-amiga-13 profiles |
| Emission | Extent-sized block-scope `static const` template + one `memcpy` at the existing init point |
| Excluded | Static declaration initializer, linker flags, whole-program DCE, local/fill/short-list/multi-extent/float/struct/owned/const-array shapes, freestanding multi-byte templates |
| Comparison requirements | Provisional `lookup:instructions:10 lookup:text:10`; re-derived in Phase 1 |
| Execution mode | `--auto-commit --auto-design`; pushes held to the final gate |
| Branch | `main` |

## Specialist Agents

| Role | Agent | Phase |
|---|---|---|
| Correctness reviewer | `correctness-reviewer` | Every codegen phase |
| Memory-safety auditor | `memory-safety-auditor` | Codegen (read-only ASan probes) |
| Emission-integrity auditor | `emission-integrity-auditor` | Codegen (read-only base/head emission diff, strict C) |
| Performance auditor | `performance-auditor` | Codegen (threshold and over-optimization judgment) |

Review packets include a blast-radius step and a test-integrity step. Critical
and major findings pause execution for a ruling.

## Scope Boundary

In scope: the measurement audit, the `lookup` benchmark, the constant-array
data-template emission, behavior and emission tests, the measurement evidence,
the phase reviews, and the documentation/roadmap closeout.

Out of scope: language, grammar, diagnostics, ABI, runtime shim, standard
library, DX/installer/LSP, Amiga wrappers, v2, `Box<T>`/trampolines, compiler
compile-time speed, host wall time, linker/toolchain flags, dead-code
elimination, safety trade-offs, unchecked bounds, trap removal, new crates,
and build scripts.
