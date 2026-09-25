# Clorraine v1 language completion Implementation Plan

> **Feature**: Complete the v1 language surfaces the corpus-subset MVP staged out: generics, lambdas (stages 1-4), and volatile intrinsics with the per-chip MMIO wrappers.
> **Status**: Planning Complete
> **Created**: 2026-09-24 19:15
> **Implements**: (none — standalone plan; the requirements source is the frozen specification `spec/`, v0.4)
> **CodeOps Artifact Schema**: 1

## Overview

The MVP (`plans/clorraine-compiler-mvp/`) implemented exactly the corpus subset
of the frozen specification. Everything outside that subset parses but reports
a staged-feature code. Three of those surfaces are **v1** by the ledger, not v2:
generics (G1-G8), lambdas (L1-L5, stages 1-4), and volatile access (T7). Today
the compiler monomorphizes generic *functions* but silently emits invalid C for
a generic *struct* that stores `T`, reports `E90000`/`E90002` for every lambda,
and lowers the six volatile intrinsics to the literal `0`. The last two are not
missing features alone; they are silent wrong-code holes.

This plan closes all three surfaces against the frozen specification, fixes the
two silent miscompiles as first-class work, and reconciles the specification and
its golden data with the now-implemented behavior. Generics land first because
lambda stage 4 (a capturing closure passed to a callable-bound generic
parameter) depends on the callable bound; the volatile/MMIO phase is independent
and lands last.

The plan is standalone: no upstream requirement document exists, so
`01-requirements.md` is the owning requirements document and every decision
traces to the Ambiguity Register (`00-ambiguity-register.md`).

## Minimum-Sufficient Baseline

**Original goal:** Make the v1 language complete for generics, lambdas, and
volatile access, and remove the silent wrong-code paths, using the existing
stage pipeline.

**Smallest viable design:** Extend the existing per-stage modules in place: the
generic path reuses `Monomorphization` and `TyKind::Struct(name, args)` and
adds struct instances; the lambda path lifts lambdas to `static` functions
(reusing the existing generic-instantiation machinery for the callable-bound
case); the volatile path adds the two missing codegen/typing cases and a
Clorraine stdlib module. No new crate, harness, dependency, or phase is added.

**Excluded machinery:** a heap closure environment, C-hook trampolines, an
`Array<move-only>` drop-glue/region model, a `comptime` evaluator, data
inclusion, and a dedicated instantiation-report tool (AR-33-AR-39).

**Approved complexity:** None beyond ordinary compiler and stdlib work; the
larger mechanisms considered were deferred (AR-33-AR-39).

## Document Index

| #   | Document                                       | Description                                 |
| --- | ---------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                           | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)             | Feature requirements and scope              |
| 02  | [Current State](02-current-state.md)           | Analysis of the current implementation      |
| 03-01 | [Generics](03-01-generics.md)                | Struct monomorphization, bounds, ephemeral args |
| 03-02 | [Lambdas](03-02-lambdas.md)                  | Lifting, captures, closure calling convention |
| 03-03 | [Volatile and MMIO](03-03-volatile-mmio.md)  | Six intrinsics, volatile globals, MMIO module |
| 03-04 | [Spec and Goldens](03-04-spec-goldens.md)    | Specification edits and golden test data    |
| 07  | [Testing Strategy](07-testing-strategy.md)     | ST cases and verification                   |
| 99  | [Execution Plan](99-execution-plan.md)         | Phases, sessions, and task checklist        |

## Quick Reference

### Usage Examples

```clorraine
// Generic struct that stores the type parameter (today: invalid C)
struct Pair<T> { first: T; second: T; }
let p = Pair<i32> { first: 1, second: 2 };

// Capture-less lambda: a plain function pointer
let inc: func(i32): i32 = (x: i32) => x + 1;

// Capturing closure passed inline to a callable-bound generic parameter.
// Leave F to inference: writing the explicit type argument func(i32): i32 for F
// would make the argument position a concrete func value and require E10400.
func apply<T, F: func(T): T>(x: T, f: F): T { return f(x); }
let base: i32 = 10;
let r = apply(1, [base] (x: i32) => x + base);

// Volatile access (inside a library wrapper; unsafe in application code)
export func setDma(enabled: bool): void {
    unsafe {
        if (enabled) { CUSTOM_BASE.dmacon = CUSTOM_BASE.dmacon | 0x0080; }
    }
}
```

### Key Decisions

| Decision            | Outcome                                                        |
| ------------------- | -------------------------------------------------------------- |
| Scope               | Generics + lambdas (stages 1-4) + volatile/MMIO (AR-2)         |
| Generic structs     | One concrete C struct per deduped instantiation (AR-17)        |
| Callable bound      | A capturing closure fills a generic `F: func(...)`; not a concrete `func(...)` type (AR-10) |
| Closure locals      | Inline-only; a named capturing closure is E10404 (AR-11)       |
| Volatile globals    | Module-level `volatile let` emits a C `volatile` static (AR-20) |
| Retired code        | E90002 is reserved; E90000 keeps only the `Array<move-only>` trigger (AR-21) |
| MMIO module         | `std.amiga.custom`, minimal normative register set (AR-7, AR-28) |
| Verify              | `make check` per task; `make verify` per phase (AR-26)         |

## Related Files

Created: `compiler/tests/v1_language_spec_test.rs`,
`compiler/tests/v1_language_impl_test.rs`, `stdlib/std.amiga.custom.clo`,
`spec/tests/corpus/volatile-mmio/**`,
`spec/tests/compile-fail/E11200_custom_module.clo`, and
`plans/clorraine-v1-language-completion/**`.

Modified: `compiler/src/ir/mono.rs`, `compiler/src/own/generics.rs`,
`compiler/src/types/{mod,table}.rs`, `compiler/src/sema/{mod,expr,call,items}.rs`,
`compiler/src/codegen/{types,program,calls,expr}.rs`, `compiler/src/ir/global.rs`,
`compiler/src/stdlib.rs`,
`compiler/tests/{compile_fail,stage,corpus,determinism}_spec_test.rs`,
`docker/scripts/corpus-amiga.sh`, `AGENTS.md`, `plans/00-roadmap.md`, and the authorized `spec/`
chapters and goldens listed in `03-04-spec-goldens.md`.

Read-only dependencies: `spec/{02,03,04,06,07,12,14,16,17}-*.md`,
`spec/appendix-amiga.md`, `spec/decisions.md`, `spec/grammar.ebnf.md`,
`stdlib/std.mem.clo`, `compiler/runtime/shim_hosted_amiga.c`.
