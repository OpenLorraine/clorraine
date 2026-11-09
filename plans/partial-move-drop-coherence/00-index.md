# Partial-Move / Drop Coherence Implementation Plan

> **Feature**: Make a value-consuming move of an owned place nested in an aggregate literal zero its source, so a later drop or reassignment of the source cannot free a value the destination owns.
> **Status**: Done — 10/10 tasks; DEF-11 fixed (2026-09-26); plan and roadmap rows complete.
> **Created**: 2026-09-25
> **Implements**: DEF-11
> **CodeOps Artifact Schema**: 1

## Overview

The compiler lowers a value-consuming read of an owned place as a plain copy
when the place is nested in an aggregate literal, so the source is never emptied.
The destination and the source then own the same handle, and both drops free it:
a double free. It reproduces for `b = Box<String> { value: b.value };` and for
`let t = b.value; b = Box<String> { value: t };`, for generic and non-generic
structs. The frozen spec already states the invariant the compiler misses: a
move lowers to a copy plus a zero of the source, and the zeroed source is the
drop flag (`spec/17-codegen.md:182-199`, P3).

The fix records the consumed field as moved in the ownership stage and consumes
it as a move in the lowering. Once the literal field is consumed, the assignment
fast path already releases the empty old target correctly, so the assignment
emitter is unchanged.

## Minimum-Sufficient Baseline

**Original goal:** an owned field moved out of a value must not be freed twice.
**Smallest viable design:** mark a consumed non-duplicable literal field moved,
and consume it into a temporary through the existing `Statement::Move`
(copy + zero source) in `lower_value`.
**Excluded machinery:** None. The change reuses the existing `Move` IR,
`move_target`, and `materialize_value`; no new module, dependency, harness, or
abstraction (AR-10).

## Document Index

| #   | Document                                                              | Description                                 |
| --- | --------------------------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)                        | Zero-Ambiguity Gate decisions (audit trail) |
| PF  | [Preflight Report](00-preflight-report.md)                            | Post-creation audit (12 findings, resolved) |
| 00  | [Index](00-index.md)                                                  | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                                    | Requirements and scope                      |
| 02  | [Current State](02-current-state.md)                                  | Analysis of the current implementation      |
| 03-01 | [Owned Literal-Field Consumption](03-01-nested-move-lowering.md)     | Ownership mark and lowering consume         |
| 07  | [Testing Strategy](07-testing-strategy.md)                            | Regression cases and verification           |
| 99  | [Execution Plan](99-execution-plan.md)                                | Phases, sessions, and task checklist        |

## Quick Reference

### Programs that change from a double free to free-once

```clorraine
struct Box<T> { value: T; }

func demoA(): void {
    let b = Box<String> { value: stringOf("x") };
    b = Box<String> { value: b.value };   // frees once
}

func demoB(): void {
    let b = Box<String> { value: stringOf("x") };
    let t = b.value;                      // moves out of b.value
    b = Box<String> { value: t };         // frees once
}
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Breadth | One general invariant for a consumed literal field (AR-1) |
| Fix site | Ownership mark + lowering; the assignment emitter is unchanged (AR-3) |
| Semantics | Free once; the zeroed source is the drop flag; partial moves stay legal (AR-2, AR-7) |
| Out of scope | The `take`-field `Member` shape, self-assignment, guarded fields (AR-13, AR-15) |
| Tests | build-and-run oracle in an impl suite + emitted-C in the spec suites (AR-4, AR-18) |
| Verify | `make check` per task; final `make verify` (AR-5) |

## Related Files

- `compiler/src/own/expr.rs` — `ExprKind::StructLiteral` check.
- `compiler/src/own/lower.rs` — `lower_value` literal lowering.
- `compiler/src/own/place.rs` — `move_target`/`move_place` (reused).
- `compiler/src/codegen/function.rs` — `emit_assign` (unchanged), `emit_move_zero`.
- `compiler/src/ir.rs` — `Statement::Move`.
- `compiler/tests/stage_spec_test.rs`, `compiler/tests/codegen_run_spec_test.rs` — regression oracles.
