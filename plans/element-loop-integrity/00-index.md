# Element Loop Integrity Implementation Plan

> **Feature**: Fix the owned-element read double free (DEF-16) and the nested-loop step mis-selection (DEF-17) in the ownership checker and C emitter
> **Status**: Complete — executed and verified 2026-09-28 (all 16 tasks done, ASan-clean, `make verify` all legs passed)
> **Created**: 2026-09-28
> **Implements**: DEF-16, DEF-17
> **CodeOps Artifact Schema**: 1

## Overview

Two critical defects were filed while executing the codegen-ownership-safety plan.
Both are correctness bugs in code generation for accepted programs, and both
violate the frozen specification.

**DEF-16** — reading an owned container's element and passing it to a parameter
binds a second owner. `materialize_owned_argument` (`compiler/src/own/lower.rs:659`)
exists to give an ownerless temporary (a call result) a binding the scope cleanup
can release; it treats an element read (`ValueKind::Index`) as such a temporary,
even though the array already owns the element. The scope then drops both owners:
`vlen(arr[0])`, `vlen(arr[i])`, `vlen(ps[0].name)`, and `sink(arr[0])` all abort
with a double free under AddressSanitizer. Member reads over locals are already
lowered to places, so only element (and element-derived) reads are affected.

**DEF-17** — a `for` loop nested inside another loop hangs. The emitter's
`for_step_block` search (`compiler/src/codegen/control.rs:407`) looks for the first
block in the body's closure that jumps to the loop head. For a nested loop, the
loop's own pre-header block (its init) jumps to the head and is re-entered through
the enclosing loop's back edge, so the search returns it: its init assignments are
printed as the C `for` step, the real step block becomes dead code, and the loop
never advances. Element-in-element, element-in-while, and C-style-for-in-while all
hang; two sequential loops are correct.

## Minimum-Sufficient Baseline

**Original goal:** make owned-element reads lend instead of double-own, and make
nested `for` loops run their own step.

**Smallest viable design:** (1) a read-only ownership predicate
`projects_from_place` that recognizes a place or a field/element projection of one
at any index, used to skip materialization for such values; (2) carry the already-
known step block on the loop's existing `BranchShape::Loop` shape and let the
emitter use it directly, deleting the search.

**Excluded machinery:** None — no new layer, dependency, harness, or refactor; both
fixes are direct changes in existing functions.

**Approved complexity:** None.

## Document Index

| #   | Document                                                            | Description                                 |
| --- | ------------------------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)                      | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                                | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                                  | Feature requirements and scope              |
| 02  | [Current State](02-current-state.md)                                | Analysis of the current implementation      |
| 03-01 | [Element Read Ownership](03-01-element-read-ownership.md)         | DEF-16 technical specification              |
| 03-02 | [Nested Loop Step](03-02-nested-loop-step.md)                     | DEF-17 technical specification              |
| 07  | [Testing Strategy](07-testing-strategy.md)                          | Test cases and verification                 |
| 99  | [Execution Plan](99-execution-plan.md)                              | Phases, sessions, and task checklist        |

## Quick Reference

### Reproductions (grounded during planning)

```clorraine
// DEF-16: double free (AddressSanitizer) — the array and the scope temp both free "a"
let arr: String[2] = [stringOf("a"), stringOf("bb")];
return vlen(arr[0]);

// DEF-17: hangs — the inner C `for` step prints `tmp2 = 0u` instead of `tmp2 = tmp2 + 1u`
for (borrow x in xs) {          // xs: i32[2] = [1, 2]
    for (borrow y in ys) { ... } // ys: i32[3] = [1, 2, 3]
}
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| DEF-16 approach | Narrow materialization guard; no change to the element read path (`projects_from_place`, AR #6, AR #12) |
| DEF-17 approach | Store the step block on `BranchShape::Loop`; delete `for_step_block` (AR #7, AR #8) |
| Scope | DEF-16 + DEF-17 only; DEF-18 – DEF-20 stay filed; `mk()[0]` invalid C becomes DEF-21 (AR #2, AR #3, AR #4) |
| Test surface | New `element_loop_integrity_spec_test.rs` and `_impl_test.rs` with a bounded run (AR #5, AR #10) |

## Related Files

- `compiler/src/own/lower.rs` — materialization guard and the new predicate
- `compiler/src/own/call.rs` — call-argument materialization call site
- `compiler/src/ir/control.rs` — `BranchShape` gains the `step` field
- `compiler/src/own/loops.rs` — passes the step at both `for` construction sites
- `compiler/src/codegen/control.rs` — the emitter consumes the stored step
- `compiler/tests/element_loop_integrity_spec_test.rs`, `compiler/tests/element_loop_integrity_impl_test.rs` — the new suites
