# Partial-Move / Drop Coherence Implementation Plan

> **Feature**: Make a value-consuming move of an owned place zero its source, so a later drop or reassignment of the source cannot free a value the destination owns.
> **Status**: Planning Complete
> **Created**: 2026-09-25
> **Implements**: DEF-11
> **CodeOps Artifact Schema**: 1

## Overview

The compiler lowers a value-consuming read of an owned place as a plain copy
when the place is nested in an aggregate literal (or passed as a `take`
argument), so the source is never emptied. The destination and the source then
own the same handle, and both drops free it: a double free. It reproduces for
`b = Box<String> { value: b.value };` and for
`let t = b.value; b = Box<String> { value: t };`, for generic and non-generic
structs. The frozen spec already states the invariant the compiler misses: a
move lowers to a copy plus a zero of the source, and the zeroed source is the
drop flag (`spec/17-codegen.md:182-199`, P3).

The fix realizes that invariant in the ownership lowering and removes a stale
save-copy from the assignment lowering. The result is free-once semantics for
every value-consuming move-out form.

## Minimum-Sufficient Baseline

**Original goal:** an owned field moved out of a value must not be freed twice.
**Smallest viable design:** one lowering helper that consumes an owned place into
a temporary via the existing `Statement::Move` (copy + zero source), used at the
value-consuming sites; plus the assignment lowering evaluating the RHS before
releasing the old target.
**Excluded machinery:** None. The change reuses the existing `Move`/`MoveOut` IR
and `emit_move_zero`; no new module, dependency, harness, or abstraction
(AR-10).

## Document Index

| #   | Document                                                              | Description                                 |
| --- | --------------------------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)                        | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                                  | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                                    | Requirements and scope                      |
| 02  | [Current State](02-current-state.md)                                  | Analysis of the current implementation      |
| 03-01 | [Nested Move Lowering](03-01-nested-move-lowering.md)               | Consume an owned place as a move            |
| 03-02 | [Assignment Ordering](03-02-assignment-ordering.md)                 | Release the old target after the RHS        |
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
| Breadth | One general invariant, not a per-site patch (AR-1) |
| Semantics | Free once; the zeroed source is the drop flag; partial moves stay legal (AR-2, AR-7) |
| Fix site | Ownership lowering + assignment lowering (AR-3) |
| Tests | ASan runtime + emitted-C, in the spec suites plus an ASan helper (AR-4) |
| Verify | `make check` per task; final `make verify` (AR-5) |

## Related Files

- `compiler/src/own/lower.rs` — literal lowering (`lower_value`).
- `compiler/src/own/statements.rs` — `emit_store`; drop/return cleanup.
- `compiler/src/own/call.rs` — `take_move_out_places`.
- `compiler/src/codegen/function.rs` — `emit_assign`, `emit_move_zero`.
- `compiler/src/ir.rs` — `Statement::Move`, `Statement::MoveOut`.
- `compiler/tests/codegen_run_spec_test.rs`, `compiler/tests/stage_spec_test.rs` — regression oracles.
