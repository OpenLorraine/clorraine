# Execution Plan — Deferred Evaluation Completion (Batch V)

> **Progress**: 17/17 tasks
> **Rows**: `DEF-534`, `DEF-535`; residual `DEF-540`
> **Baseline**: `55b6419`
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`)

## Phase 1: Deferred `len` effects (`DEF-534`)

- [x] 1.1.1 Add `ValueKind::DiscardThen` and its codegen traversal
- [x] 1.1.2 Lower a deferred `len` operand's effects to a value
- [x] 1.1.3 Emit statement and inline forms; keep a place arm a read
- [x] 1.2.1 Cover outer arm, short-circuit, while/do/for, defer, index
- [x] 1.2.2 Cover growable containers and a nested conditional condition
- [x] 1.3.1 Materialize an index/divisor/slice-count effect once
- [x] 1.3.2 Materialize a conditional loop test and a for step per test
- [x] 1.4.1 Run the focused suite, `make check`, and `RDC_ASAN=1`

## Phase 2: Conditional opaque store (`DEF-535`)

- [x] 2.1.1 Return the instance-decided flag for any owned source
- [x] 2.1.2 Record arm sources, frames, and later uses
- [x] 2.2.1 Audit arm moves, partial moves, and later reads per instance
- [x] 2.2.2 Track definite re-initializations in the flow state
- [x] 2.3.1 Reject an unread fresh arm at a view destination
- [x] 2.4.1 Pin the generic, concrete, non-duplicable, owning, local, fresh,
      mixed-report, partial-whole, and re-init matrix

## Review corrections

- [x] 3.1.1 Close the conditional loop, member/index, and for-step leaks
- [x] 3.1.2 Close the nested return/call condition effect drop
- [x] 3.1.3 Fix the arm diagnostic name and help text
- [x] 3.1.4 File `DEF-540` for the caller-side stored-view invalidation rule
- [x] 3.1.5 Keep an owned conditional boundary when materializing a `len`
      effect; add the nested view-store case

## Close-out

- [x] 4.1 Run `make check` and `make verify` at the batch head
- [x] 4.2 Record the findings, update the roadmap, and file `DEF-540`
