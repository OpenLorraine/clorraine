# Residual Defect Hardening Implementation Plan

> **Feature**: Fix the four remaining filed defects DEF-46 – DEF-49 in the Clorraine compiler (flat no-context array literal types and the deferred-row store, the multi-extent fixed-array return copy, the unguarded loop-condition owned-call release, and substitution-aware ownership for generic instances)
> **Status**: Planning Complete
> **Created**: 2026-10-01 13:16
> **Implements**: DEF-46, DEF-47, DEF-48, DEF-49
> **CodeOps Artifact Schema**: 1

## Overview

The call-value-integrity plan closed sixteen defects and filed four residual ones. Each is a gap
where a value crosses a C boundary without the treatment its ownership or type requires, and each
was reproduced against the committed compiler before this plan was written.

DEF-46: an array literal with no context type is inferred with a nested element type instead of
the flat extents every other consumer assumes, and the deferred guard path has no expression-form
store for a nested row that is not itself a literal. The deferred spelling emits invalid C;
the plain unannotated declaration is broken by the same type shape. DEF-47: a returned
multi-extent fixed-array place is copied one extent too shallow, emitting an array assignment C
rejects. DEF-48: a loop condition that holds an owned call result but needs no runtime guard
leaves the call inline in the header, so the caller never releases the value and each iteration
leaks. DEF-49: ownership decisions are made once, before monomorphization substitution, so a
generic body whose parameter type owns under substitution emits no release and leaks; the same
blindness applies to move-vs-copy and to the actions the lowerer suppressed for type parameters.

None of the four needs a new language surface, a new diagnostic, or a new subsystem. Every fix
reuses existing machinery — the flat extents convention, the expression-form conditional store,
the fixed-array store core, the guarded loop path with its materialize-and-release bookkeeping,
and the per-instance audit (`own/instance_check.rs`) that already re-applies predicates to each
recorded instantiation (`CopyObligation` records substitution-dependent duties but has no
discharge consumer yet).
For the generic case the checker deposits symbolic ownership duties at its param-typed decision
points; codegen and the per-instance audit discharge them with the substitution, so the generic
body stays checked once and no per-instance body is built.

The frozen specification under `spec/` is the oracle. This plan changes no specification text and
adds no diagnostic code.

## Minimum-Sufficient Baseline

**Original goal:** Fix the four filed defects DEF-46 – DEF-49 so the accepted programs compile to
valid C and release what they own under every instantiation.
**Smallest viable design:** Canonicalize array types to the existing flat extents in the
constructor and in generic substitution, and add the missing expression-form row store; delegate
the return copy to the existing fixed-array store core; route owned-call loop conditions into the
existing guarded loop path; and fix generic ownership by depositing symbolic ownership duties at
the checker's param-typed decision points and discharging them per instance. No new layer,
dependency, harness, or crate.
**Excluded machinery:** A generalized ownership refactor or a re-check of generic bodies from the
source tree was considered and not selected; the chosen design reuses the recorded instance set
and the existing codegen, and the plan keeps the smallest design that covers the filed shapes.
**Approved complexity:** None (see `00-ambiguity-register.md`).

## Document Index

| #   | Document | Description |
| --- | -------- | ----------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md) | This document — overview and navigation |
| 01  | [Requirements](01-requirements.md) | Feature requirements and scope |
| 02  | [Current State](02-current-state.md) | Analysis of the current implementation |
| 03-01 | [Flat Array Literal Types and the Deferred Row Store](03-01-flat-array-literal-types.md) | DEF-46 design |
| 03-02 | [Multi-Extent Return Copy](03-02-multi-extent-return-copy.md) | DEF-47 design |
| 03-03 | [Loop-Condition Owned-Call Release](03-03-loop-condition-owned-call-release.md) | DEF-48 design |
| 03-04 | [Substitution-Aware Ownership](03-04-substitution-aware-ownership.md) | DEF-49 design |
| 07  | [Testing Strategy](07-testing-strategy.md) | Specification test cases and verification |
| 99  | [Execution Plan](99-execution-plan.md) | Phases, sessions, and task checklist |

## Quick Reference

### Usage Examples

These are the defect shapes this plan makes correct:

```clorraine
// DEF-46: a no-context literal indexed through a deferred guard operand.
let hit: bool = ok && [c ? rows[0] : rows[1], [5, 6]][0][0] == 1;
// ... and the plain unannotated declaration of the same shape.
let lit = [c ? rows[0] : rows[1], [5, 6]];

// DEF-47: returning a multi-extent fixed-array place.
func copy(take a: String[2][2]): String[2][2] { return a; }

// DEF-48: an owned call result in an unguarded loop condition.
while (grow().count > 0 && i < 3) { i = i + 1; }

// DEF-49: a generic body that owns its parameter.
func keep<T>(take a: T[2]): i32 { return 0; }
```

### Key Decisions

| Decision | Outcome | AR Ref |
| -------- | ------- | ------ |
| Plan shape | One plan, all four defects, four phases plus close-out | AR #1 |
| Array literal types | Canonicalize flat extents in the constructor and `substitute`; add the expression-form row store | AR #3 |
| Return copy | Delegate the place branch to the existing fixed-array store core | AR #12 |
| Loop release | Route owned-call conditions into the guarded flag-first loop path | AR #5 |
| Generic ownership | Obligation records deposited by the checker and discharged per instance | AR #17 |
| Diagnostics | No new codes; existing moved-value diagnostic for a copy conflict under an owning instance | AR #11 |
| Spec oracle | No `spec/` change; the defects are implementation gaps | AR #8 |
| Regression suites | New `residual_defect_hardening_{spec,impl}_test.rs` | AR #2 |

## Related Files

| Path | Role |
| ---- | ---- |
| `compiler/src/types/mod.rs`, `compiler/src/sema/expr.rs` | Array type construction and literal inference |
| `compiler/src/codegen/{types,guards,function,cleanup,control}.rs` | C spelling, expression stores, fixed-array core, returns, loop emission |
| `compiler/src/own/{loops,lower,expr,statements}.rs`, `compiler/src/ir/guards.rs` | Condition lowering, guard prediction, suppression sites |
| `compiler/src/own/{generics,instance_check}.rs`, `compiler/src/ir/mono.rs`, `compiler/src/codegen/{function,program}.rs` | Instance recording, per-instance audit precedent, emission |
| `compiler/src/own/{call_check,call}.rs` | Take-argument move recording and the argument move-out (Phase 4) |
| `compiler/src/own/{place,flow}.rs` | Read hook and the moved-if-owning flow marker (Phase 4) |
| `compiler/src/ir.rs`, `compiler/src/own.rs`, `compiler/src/own/{lower,expr,statements,loops}.rs` | Obligation records and the param-typed decision sites (proposed) |
| `compiler/tests/residual_defect_hardening_{spec,impl}_test.rs` | New regression suites |
| `plans/00-roadmap.md` | DEF-46 – DEF-49 rows (Plan Created → Executing → Done, AR #7) |
| `AGENTS.md` | Test-map entry for the new suites (close-out) |
