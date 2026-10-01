# Call Value Integrity Implementation Plan

> **Feature**: Fix the sixteen filed defects DEF-30 – DEF-45 in the Clorraine compiler (owned conditionals including aggregates, fixed-array value positions, func-pointer emission and resolution, guard/cleanup emission)
> **Status**: Complete — executed and verified 2026-10-01 (39/39 tasks, spec 62/62, impl 61/61, ASan probes clean, `make verify` all legs passed)
> **Created**: 2026-09-30 08:51
> **Implements**: DEF-30, DEF-31, DEF-32, DEF-33, DEF-34, DEF-35, DEF-36, DEF-37, DEF-38, DEF-39, DEF-40, DEF-41, DEF-42, DEF-43, DEF-44, DEF-45
> **CodeOps Artifact Schema**: 1

## Overview

The runtime-value-integrity reviews and preflight left sixteen filed defects open. They share one
theme: a value or a callable crosses a C boundary without the ownership, place, or declarator
treatment the value semantics require. The non-array forms of owned conditionals double-free or leak;
fixed-array values have no C place in stores, arguments, projections, and struct-literal fields;
function-pointer spellings bypass the shared declarator and calls through captures, globals, and
values resolve by source name; and a few guard/cleanup paths emit stale or missing code.

None of the defects needs a new language surface or a new subsystem. Every fix reuses machinery the
runtime-value-integrity plan built — the branch store, the materialization helpers, the hidden
destination call ABI, the pending-release set, and the shared func-pointer declarator — and extends
it to the positions the defects name. The plan is one feature with four implementation phases
(owned conditional stores, fixed-array value positions, func pointers, guards/cleanup) plus a close-out
phase.

The frozen specification under `spec/` is the oracle. This plan changes no specification text and
adds no diagnostic. One filed shape (a conditional over move-only element places) stays the
spec-mandated `E10026` rejection; the plan pins that behavior with a regression case.

## Minimum-Sufficient Baseline

**Original goal:** Fix the sixteen filed defects DEF-30 – DEF-45 so the accepted programs compile to
valid C with correct ownership and runtime behavior.
**Smallest viable design:** Reuse the existing branch-store rule, materialization helpers,
hidden-destination call ABI, pending-release set, and func-pointer declarator; extend each to the
defect's code path. No new architecture layer, dependency, harness, or abstraction.
**Excluded machinery:** A generalized ownership-analysis layer or a new IR pass was not considered
necessary; the defects are position-specific gaps in existing lowering. `None` approved.
**Approved complexity:** None (see `00-ambiguity-register.md`).

## Document Index

| #   | Document | Description |
| --- | -------- | ----------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md) | This document — overview and navigation |
| 01  | [Requirements](01-requirements.md) | Feature requirements and scope |
| 02  | [Current State](02-current-state.md) | Analysis of the current implementation |
| 03-01 | [Owned Conditional Stores](03-01-scalar-conditional-ownership.md) | DEF-34 and DEF-45 design |
| 03-02 | [Fixed-Array Value Positions](03-02-fixed-array-value-positions.md) | DEF-31, 32, 33, 35, 38 design |
| 03-03 | [Func Pointers and Callable Resolution](03-03-func-pointers-and-callable-resolution.md) | DEF-30, 40, 41, 42, 43, 44 design |
| 03-04 | [Guards and Cleanup](03-04-guards-and-cleanup.md) | DEF-36, 37, 39 design |
| 07  | [Testing Strategy](07-testing-strategy.md) | Specification test cases and verification |
| 99  | [Execution Plan](99-execution-plan.md) | Phases, sessions, and task checklist |

## Quick Reference

### Key Decisions

| Decision | Outcome | AR Ref |
| -------- | ------- | ------ |
| Plan shape | One plan for all sixteen defects, phased by cluster | AR #1 |
| Owned conditionals | Branch store per path (handles and owned aggregates); transfer temps are release-aware; adoption at binding, assignment, `take` argument, `return`, and struct-literal field | AR #2 |
| Move-only element arms | Stay `E10026` (spec limitation); regression-pinned | AR #3 |
| `borrow` in func types | Implemented: `FuncTy` params carry the mode | AR #4 |
| Regression tests | New `call_value_integrity_spec/impl` suites | AR #5 |
| Struct-literal array field | Zero-init then per-field store | AR #7 |
| Do-while guarded condition | Goto-free flag-first loop (no spec change) | AR #8 |
| Growable-array drop in guards | Limited `drop_expr_text` form (no-drop elements); drop-needing elements keep the skip | AR #9 |

### Usage Examples

These snippets are the defect shapes this plan makes correct:

```clorraine
// Owned conditional (DEF-34): the chosen arm moves into `c`.
let c: String = ok ? a : mk();

// Indexed fixed-array element store (DEF-31): the element is byte-copied.
let rows: i32[2][2] = [[1, 2], [3, 4]];
rows[i] = a;

// Conditional fixed-array argument (DEF-32/33): materialized per path.
eat(ok ? mkA() : mkB());

// Function pointer with a fixed-array return (DEF-30/43).
let g: func(i32): i32[2] = (x: i32) => { return [x, x + 1]; };

// Borrowed func-type parameter (DEF-44).
let bump: func(borrow Entity): void = nudge;
```

## Related Files

| Path | Role |
| ---- | ---- |
| `compiler/src/own/{statements,expr,lower,call,loops,place}.rs` | Lowering and ownership decisions |
| `compiler/src/codegen/{function,expr,types,cleanup,guards,drop,calls}.rs` | C emission sites |
| `compiler/src/{ir.rs,own.rs,codegen.rs}` | IR values, release bookkeeping, symbol resolution |
| `compiler/src/types/{mod,table}.rs` | Type table (`FuncTy`) |
| `compiler/tests/call_value_integrity_{spec,impl}_test.rs` | New regression suites |
| `compiler/tests/runtime_value_integrity_impl_test.rs`, `compiler/src/own/tests.rs` | Existing pins updated in place (AR #15) |
| `plans/00-roadmap.md` | DEF-30 – DEF-45 rows (Executing → Done, AR #18) |
