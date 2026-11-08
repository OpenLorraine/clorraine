# Compiler Robustness Closeout (PLAN-18)

> **Feature**: Close the six latent compiler defects left outside the generated-C optimization work, add a host UBSan leg, and add a committed deterministic grammar-based fuzzer.
> **Status**: Done — 38/38; all gates green; pushed
> **Created**: 2026-11-07
> **Last Updated**: 2026-11-07
> **Implements**: R1–R9
> **CodeOps Artifact Schema**: 1

**Original goal:** close the six known latent defects with red-first tests and
base/head evidence, make undefined behavior visible in the default verification
path, and keep the compiler honest with a deterministic differential fuzzer.

## Overview

The generated-C optimization closed its own scope. Five defects it recorded
stayed in the backlog because they were outside that change, and a sixth (the
inline constant shift divisor) was fixed by its final commits without a
dedicated regression test. This feature is the closeout:

- fix the pure-fill guard omission (a memory-safety and fault-contract bug);
- make narrow integer arithmetic wrap at its result type in generated C, as the
  constant folder already does, and keep the strict C build clean;
- pin the inline constant shift divisor with a regression test and recorded
  base/head evidence;
- round `f32` casts and arithmetic in the constant folder;
- make the enum folder and the emitter compute the same member values;
- add a `make ubsan` host leg;
- add a deterministic Python-stdlib fuzzer with a bounded default-gate smoke
  run and a longer local run.

The six defects are latent: the frozen corpus does not exercise them, so they
survived the v1.0 gates. Each fix is proven by a test that fails without it.

## Minimum-Sufficient Baseline

**Smallest viable design:** five local codegen changes (pure-fill guards,
narrow-result wrapping, `f32` rounding plus the constant float-to-integer
E10202 trigger, enum member folding plus the user-ruled auto-increment fit
check, and type-name conversion emission), one already-fixed defect pinned by a
test, one sanitizer runner that mirrors the existing ASan runner, one
Python-stdlib fuzzer under `tools/fuzz/`, the new spec/impl suites, and
documentation. No grammar, ABI, runtime-shim, stdlib, or golden change. The
only diagnostic movement is the existing E10202 code gaining two triggers: the
user-ruled enum member fit check and the frozen EX-16 constant float-to-integer
overflow rule. No new diagnostic code, message, or help text. No new crate,
build script, or external package.

**Excluded machinery:** a general optimizer pass, a property-testing dependency
or a Rust fuzz target, coverage-guided mutation, FS-UAE cycle instrumentation,
float constant materialization, and any performance tuning beyond the
correctness fixes. The fuzzer is a bounded grammar-based generator, not a
coverage-guided engine.

**Approved complexity:** one tooling directory, one sanitizer runner script,
five Makefile targets (`ubsan`, `fuzz`, `fuzz-test`, `fuzz-smoke`,
`fuzz-smoke-host`), three new Rust test suites, two modified test suites (the
historical PLAN-14 pins), and documentation edits.

## Document Index

| # | Document | Description |
|---|---|---|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Every material decision and its resolution |
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Functional, verification, and acceptance requirements |
| 02 | [Current State](02-current-state.md) | Reproductions and root causes at the starting commit |
| 03-01 | [Defect Fixes](03-01-defect-fixes.md) | Each fix, its invariants, and rejected alternatives |
| 03-02 | [UBSan and Fuzzing](03-02-verification-infrastructure.md) | Runner, generator, classification, and gate design |
| 05 | [Findings](05-findings.md) | Base/head evidence, review findings, and residual limits |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec, impl, sanitizer, fuzzer, and gate evidence |
| 00-PF-REQ | [Requirements Preflight](00-preflight-report-requirements.md) | Independent requirements audit and resolution |
| 00-PF-PLAN | [Plan Preflight](00-preflight-report.md) | Independent plan audit and resolution |
| 99 | [Execution Plan](99-execution-plan.md) | Task-by-task delivery and evidence |

## Specialist Agents

**None new.** The repository's existing correctness, memory-safety,
emission-integrity, and semantics reviewers cover the change surface. The
sanitizer and fuzzer work is verification engineering inside the existing
toolchain, not a new external system or regulated domain.

_Detection evidence:_ the work is compiler codegen plus verification tooling;
`.opencode/agents/` already provides correctness, ownership/memory, emission
integrity, semantics, and performance roles. The negative check is recorded in
`05-findings.md`.

## Related Files

| Path | Role |
|---|---|
| `compiler/src/codegen/function.rs` | Pure-fill guard emission |
| `compiler/src/codegen/expr.rs` | Narrow arithmetic rendering |
| `compiler/src/consts/mod.rs`, `consts/arithmetic.rs` | `f32` rounding and enum member folding |
| `compiler/src/codegen/program.rs` | Enum constant emission |
| `compiler/tests/compiler_robustness_spec_test.rs` | Behavior oracle |
| `compiler/tests/compiler_robustness_impl_test.rs` | Emission pins |
| `compiler/tests/ubsan_corpus_spec_test.rs` | Corpus-wide UBSan sweep |
| `docker/scripts/ubsan-suite.sh` | UBSan suite runner |
| `tools/fuzz/__init__.py`, `tools/fuzz/fuzz.py` | Deterministic grammar-based fuzzer |
| `tools/fuzz/seeds/` | Committed seed programs and goldens |
| `tools/fuzz/tests/test_fuzz.py` | Fuzzer unit tests |
| `Makefile` | `ubsan`, `fuzz`, `fuzz-test`, `fuzz-smoke`, `fuzz-smoke-host`, gate wiring |
| `docs/known-limitations.md` | Accepted-limit boundary |
| `plans/00-roadmap.md` | PLAN-18 row |

## Scope Boundary

This feature changes what the emitter writes and how the compiler is verified.
It does not change:

- the grammar, the frozen specification, or any golden input;
- the language's ownership, borrow, drop, evaluation-order, or trap rules;
- the C ABI of any exported function, handle, struct, view, or generated
  symbol;
- the runtime shims, the standard library, or module initialization order;
- the storage policy for module constants (floats and enum-derived constants
  stay places).

The emitted C changes for fill guards (R1), narrow-result wrapping (R2),
float-derived constant values (R4), enum values and conversions (R5), and the
existing E10202 code gains two triggers: the user-ruled enum member fit check
and the frozen EX-16 constant float-to-integer overflow rule. No diagnostic
code, message, or help text is new.

Emitted C must stay valid C99 for host GCC, `m68k-amigaos-gcc`, vbcc, and the
freestanding profile, and must stay byte-identical for the same source,
profile, and flags.
