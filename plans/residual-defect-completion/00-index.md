# Residual Defect Completion Implementation Plan

> **Feature**: Complete the six filed residual defects DEF-50 – DEF-55 in the Clorraine compiler (array-extent C symbol keys, a generic-struct borrow argument's substituted type, lifted-lambda instance specialization, ownerless call-result projection binding, ownerless element take under an owning substitution, and the guarded drop-needing element hoist)
> **Status**: Planning Complete
> **Created**: 2026-10-02 08:47
> **Implements**: DEF-50, DEF-51, DEF-52, DEF-53, DEF-54, DEF-55
> **CodeOps Artifact Schema**: 1

## Overview

The residual-defect-hardening plan closed four defects and filed six more. Each filed defect is a
place where a value crosses a C boundary without the treatment its type or ownership requires, and
each was reproduced against the committed compiler before this plan was written.

DEF-50: `type_key` folds a fixed-array type to its element, so `Grid<i32[2]>` and `Grid<i32[3]>`
both emit `ClGrid__i32` and the C compiler rejects the redefinition. DEF-54: the named-call
pointer decision reads the unsubstituted argument type, so `sink<T>(borrow p.v)` with
`p: Outer<T>` emits a value where the C parameter is a pointer. DEF-52: a lifted lambda in a
generic body keeps the enclosing type parameter, so its nested generic call is never instantiated
and the emitted C names `ClT`. DEF-55: an owning read of a projection from a fresh call result
(`return mkv().a`, `let s = mkv().a`, `sink(mkv().a)`) never binds or releases the result, so its
other owning fields leak. DEF-53: an ownerless element take under a maybe-owning substitution
(`sink<T>(mk()[i])` with `T = String`) suppresses the caller's source marking while the callee
still releases, so the same storage frees twice. DEF-51: a conditionally evaluated materialized
operand of a drop-needing growable type has no expression-form drop, so the guarded re-store skips
the previous value and leaks it.

None of the six needs a new language surface, a new diagnostic, or a new subsystem. Every fix
reuses existing machinery — the symbol key, the argument decision, the recorded instance walk,
the materializer, and the guard emission's statement slot. The frozen specification under `spec/`
is the oracle; this plan changes no specification text and adds no diagnostic code.

## Minimum-Sufficient Baseline

**Original goal:** Make the six filed defects' reproductions compile to valid C and release what
they own: no leak, no double free, no invalid C.
**Smallest viable design:** Encode fixed-array extents in `type_key`; substitute the argument type
before the pointer decision; carry a lifted lambda through its enclosing instance's substitution
and emit one specialization per instance; bind an ownerless call result to an owned temporary
before reading a projection or an owning element from it; and hoist the statement-form drop for
guard operands whose type has no expression-form drop. No new layer, dependency, harness, or
crate.
**Excluded machinery:** A per-type drop-helper facility for the guarded element loop was
considered and rejected (the challenger's `Simplify` verdict; the existing statement-form path
serves); per-instance obligation duties for the call-result cases were considered and not
selected (the bind-temporary design is smaller).
**Approved complexity:** None — the DEF-51 escalation resolved to the smaller statement hoist
(see `00-ambiguity-register.md`, AR-8).

## Document Index

| #   | Document | Description |
| --- | -------- | ----------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md) | This document — overview and navigation |
| 01  | [Requirements](01-requirements.md) | Feature requirements and scope |
| 02  | [Current State](02-current-state.md) | Analysis of the current implementation and the confirmed reproductions |
| 03-01 | [Type Keys and Call Arguments](03-01-type-keys-and-call-arguments.md) | DEF-50 and DEF-54 technical specification |
| 03-02 | [Lifted-Lambda Instances](03-02-lifted-lambda-instances.md) | DEF-52 technical specification |
| 03-03 | [Call-Result Ownership](03-03-call-result-ownership.md) | DEF-53 and DEF-55 technical specification |
| 03-04 | [Guarded Drop Hoist](03-04-guarded-drop-hoist.md) | DEF-51 technical specification |
| 07  | [Testing Strategy](07-testing-strategy.md) | Specification cases and verification |
| 99  | [Execution Plan](99-execution-plan.md) | Phases, sessions, and task checklist |

## Quick Reference

### Reproductions

Each defect has a minimal reproduction that fails on the committed compiler; the exact emitted
failure is in [02-current-state.md](02-current-state.md) §Reproductions.

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Array-extent C symbol key | `arr2_i32` / `arr2x3_i32` (full extents in `type_key`) |
| Generic-struct borrow argument | Substitute the argument type before the pointer decision |
| Lifted lambda in a generic body | Specialize the lifted function per enclosing instance |
| Ownerless call-result projection | Bind the result to an owned temporary and move the projection out |
| Ownerless element take under a generic substitution | Same bind-temporary design; the callee owns the element |
| Guarded drop-needing element | Hoist the statement-form drop in `emit_guards`; no helper facility |

## Related Files

| File | Role in this plan |
| ---- | ----------------- |
| `compiler/src/codegen/types.rs` | `type_key` array-extent encoding; per-instance callable-type spelling |
| `compiler/src/codegen/calls.rs` | Named-call argument pointer decision |
| `compiler/src/own/generics.rs` | Instance walk and lifted-lambda specialization recording |
| `compiler/src/own.rs`, `compiler/src/ir.rs` | Lifted-function provenance |
| `compiler/src/codegen/program.rs`, `compiler/src/codegen/function.rs`, `compiler/src/codegen/expr.rs` | Per-instance lifted emission, environment structs, and closure value/call rendering |
| `compiler/src/own/lower.rs` | Ownerless call-result binding for projections and the take element move-out |
| `compiler/src/own/call_check.rs`, `compiler/src/own/expr.rs` | Maybe-owning element-take decision (unchanged; lowering owns the binding) |
| `compiler/src/codegen/guards.rs`, `compiler/src/codegen/drop.rs` | Guarded operand drop hoist |
| `compiler/tests/residual_defect_completion_{spec,impl}_test.rs` | The plan's specification and implementation suites |
