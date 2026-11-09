# Slice and Temporary Integrity Implementation Plan

> **Feature**: Close the first batch of the open defect backlog in the Clorraine compiler: the
> critical multi-extent slice view mis-typing (DEF-70), the aggregate-literal consume order/fill
> gaps (DEF-72, DEF-73, DEF-74), the unbound non-direct-call roots and receivers (DEF-71, DEF-89),
> and the invalid call-indexed element-store spelling (DEF-75).
> **Status**: Done — plan and roadmap rows complete (2026-11-09 closeout).
> **Created**: 2026-10-08 07:36
> **Implements**: DEF-70, DEF-71, DEF-72, DEF-73, DEF-74, DEF-75, DEF-89
> **CodeOps Artifact Schema**: 1

## Overview

The def-backlog close-out plans fixed every defect that shared the ownership/drop machinery they
touched, and filed the residual shapes they could not safely absorb. Twenty-four defects remain
open in the roadmap; this plan closes the first batch of seven, chosen because it contains the only
live critical defect and continues the established expression-temporaries workstream.

Two threads run through the batch. The first is the slice-view type model: a multi-extent view is
emitted flat today, so a row move copies one scalar while the caller still owns the row (host
AddressSanitizer double-free) and a nested access mis-types. The frozen specification already
requires row-typed slice elements (`spec/08-arrays-strings.md:28`), so the plan implements the
written rule rather than amending it. The second thread is the expression-temporary machinery: a
consume inside an aggregate literal can lose its fill, its evaluation order, or its root-call
binding, and two root forms outside the direct-call shapes leak the call result's other fields.

Six of the seven fixes reuse existing machinery — the deferred `Materialize` carrier and its guard
emitter, the projection-root binding, the owned-argument binding, and the element-type key. Only
DEF-70 changes the emitted type keys for multi-extent slice users; programs whose behavior depends
on the view element are wrong today, the rest are retyped without behavior change, and the change is
audited by the emission-integrity reviewer.

## Minimum-Sufficient Baseline

**Original goal:** Make every filed DEF-70, DEF-71, DEF-72, DEF-73, DEF-74, DEF-75, and DEF-89
reproduction behave correctly: no double free, no leak, no use-after-free, no wrong slot, no
evaluation-order reversal, no mis-typed access, and no invalid C — with no new language features.

**Smallest viable design:** type a slice view's element from the source's row (`iterated_element`)
and carry that type through the key, typedef, builder, and access paths instead of re-deriving it at
each site; keep a consume preparation deferred so the consuming statement's guard emission fills it
in source order; register the release on the existing eager move-out temporary and extend the two
root-binding helpers and the owned receiver binding to the missing shapes; parenthesize the
drop-expression suffix bases where the base needs grouping (field and fixed-array index). No new
layer, dependency, harness, or crate.

**Excluded machinery:** expression-wide materialization of every owned call result (AR-5); a new
emission statement shape for consume fills beyond the existing deferred `Materialize` and
`AssignThenMoveOut` carriers (AR-4); capture inference and any other defect batch (AR-1).

**Approved complexity:** None — every fix reuses existing machinery (AR-3, AR-4, AR-5).

## Document Index

| #   | Document | Description |
| --- | -------- | ----------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md) | This document — overview and navigation |
| 01  | [Requirements](01-requirements.md) | Feature requirements and scope |
| 02  | [Current State](02-current-state.md) | Analysis of the current implementation |
| 03-01 | [Slice Row Views](03-01-slice-row-views.md) | DEF-70 technical specification |
| 03-02 | [Deferred Consume Preparation](03-02-deferred-consume-preparation.md) | DEF-72, DEF-73, DEF-74 technical specification |
| 03-03 | [Root and Receiver Binding](03-03-root-and-receiver-binding.md) | DEF-71, DEF-89 technical specification |
| 03-04 | [Element-Store Drop Spelling](03-04-element-store-drop-spelling.md) | DEF-75 technical specification |
| 07  | [Testing Strategy](07-testing-strategy.md) | ST cases and verification |
| 99  | [Execution Plan](99-execution-plan.md) | Phases, sessions, and task checklist |

## Quick Reference

### Usage Examples

```clorraine
// A row slice over a fixed grid: elements are i32[3] rows (DEF-70).
let grid: i32[2][3] = [[1, 2, 3], [4, 5, 6]];
let row: i32[3] = slice<i32[3]>(grid, 0, 2)[1];   // row is [4, 5, 6]

// A nested access through a multi-extent view (DEF-70).
let xs: i32[2][2] = [[1, 2], [3, 4]];
return slice<i32[2]>(xs, 0, 2)[1][0];            // 3

// An aggregate literal whose field reads a call result at a run-time index (DEF-72).
Box<i32> { v: make()[pick()] }                   // pick() runs once; the chosen slot binds

// Source order across fields (DEF-73, DEF-74).
Box { w: first(), v: make()[i] }                 // evaluates first() before make()
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| DEF-70 disposition (AR-3) | Row-typed slice views per the frozen spec; no spec amendment |
| Consume preparation (AR-4) | Deferred; fills emitted at the consuming point in source order |
| Non-direct-call roots and receivers (AR-5) | Extend the existing binds; no general expression walk |
| DEF-75 spelling (AR-8, AR-10) | Parenthesize the drop-expression suffix bases (field and fixed-array index) |
| `slice<T>` disagreement (AR-9) | Existing `E10200`; no new diagnostic |

## Specialist Agents

**None** — the capability-gap check ran against this repository's evidence and found no gap a
standing specialist would close.

_Detection evidence: the phase-relevant roles already exist as project agents
(`.opencode/agents/memory-safety-auditor.md`, `.opencode/agents/emission-integrity-auditor.md`);
the remaining work is generic ownership/codegen review that the catalog correctness role and the
`spec/17-codegen` lens cover. A new specialist would restate those, and no repeated-finding area
lacks an existing auditor._

## Related Files

| Path | Change |
| ---- | ------ |
| `compiler/src/types/mod.rs`, `compiler/src/types/keys.rs` | Row-typed view element and key |
| `compiler/src/own/call.rs`, `compiler/src/own/lower.rs` | Slice typing, consume preparation, root/receiver binds |
| `compiler/src/codegen/types.rs`, `compiler/src/codegen/program.rs`, `compiler/src/codegen/calls.rs` | View typedef/builder emission |
| `compiler/src/codegen/function.rs`, `compiler/src/codegen/guards.rs`, `compiler/src/codegen/drop.rs` | Guard fills, element-store drop spelling |
| `compiler/tests/slice_temporary_integrity_{spec,impl}_test.rs` | New test suites |
| `compiler/tests/residual_defect_completion_impl_test.rs`, `compiler/tests/array_value_integrity_impl_test.rs`, `compiler/src/codegen/types.rs` (unit tests) | Migrated shape pins |
| `AGENTS.md` | Test-naming map for the new suites |
