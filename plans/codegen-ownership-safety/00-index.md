# Codegen Ownership Safety Implementation Plan

> **Feature**: Fix the borrow-element alias double free (DEF-13) and the shadowing miscompile (DEF-14) in the ownership checker and C emitter
> **Status**: Planning Complete
> **Created**: 2026-09-27
> **Implements**: DEF-13, DEF-14
> **CodeOps Artifact Schema**: 1

## Overview

Two critical defects were found by the runtime oracle (AddressSanitizer on the
emitted C) after the non-lexical-lifetimes plan closed. Both are correctness
bugs in code generation for accepted programs, and both violate the frozen
specification.

**DEF-13** — `for (borrow e in xs)` over an array of a droppable type (for
example `String[2]`) frees an array element twice. The emitter lowers the
element view as a copy-in/copy-back cache whose previous value is dropped like
an owner at the top of every iteration. That drop releases the element the
cache aliases, so the array is left holding a dangling pointer. A `return`
inside the loop is worse: the write-back never runs, so the element stays
dangling and the scope cleanup frees it again.

**DEF-14** — locals are keyed by source name through the IR and the C emitter,
so shadowing miscompiles. The specification's own MOD-16 example (a local
shadowing a parameter) emits C that fails `-Werror`, and a shadowed droppable
local can be freed twice or dereferenced through a null. The specification
guarantees that a local may shadow an outer local or a module name, with the
innermost binding winning.

The fix keeps both changes inside the existing ownership-stage extension
points: the element view becomes a true alias through a compiler temporary, and
each binding receives a unique IR spelling while diagnostics keep the source
name.

## Minimum-Sufficient Baseline

**Original goal:** make the two critical defects compile and run correctly with
no double free, no dangling element, and no shadowing miscompile.
**Smallest viable design:** alias the borrow element through one per-iteration
`raw` temporary and delete the copy-in/write-back machinery (AR-2); give each
binding a unique IR spelling assigned from the binding table, which is not
truncated on scope exit (AR-3).
**Excluded machinery:** patching the cache protocol instead of removing it;
direct element-place lowering with `Value`-typed places; a binding-id-keyed IR
redesign; an ASan test harness; the multi-extent element-loop defect (filed as
`DEF-15`); the owned-element read materialization double free (filed as
`DEF-16`); nested element loops (filed as `DEF-17`); moves through a borrow
element (filed as `DEF-18`).
**Approved complexity:** None. Both designs reuse existing extension points
(`Binding.base`, compiler temporaries, the name-keyed codegen maps), and the
Complexity Escalation Gate was not triggered.

## Document Index

| #   | Document | Description |
| --- | --- | --- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md) | This document — overview and navigation |
| 01  | [Requirements](01-requirements.md) | Requirements and scope |
| 02  | [Current State](02-current-state.md) | Analysis of the defects and affected code |
| 03-01 | [Borrow Element Alias](03-01-borrow-element-alias.md) | Element-view lowering specification (DEF-13) |
| 03-02 | [Local Identity](03-02-local-identity.md) | Unique IR binding spelling specification (DEF-14) |
| 07  | [Testing Strategy](07-testing-strategy.md) | Specification cases and verification |
| 99  | [Execution Plan](99-execution-plan.md) | Phases, sessions, and task checklist |

## Quick Reference

### Before and After (DEF-13)

```c
/* Before: the copy-in drops the element the cache aliases. */
for (...) {
    if (e) { FreeString(e); }        /* frees the previous array element */
    e = arr[i];
    if (e) { FreeString(e); }        /* O3 replacement of the current element */
    e = cl_std_text_stringOf(cl_str("c"));
    *(&(arr[i])) = e;                /* write-back */
}

/* After: the binding aliases the element through a per-iteration temp. */
for (...) {
    __cl_tmp1 = &(arr[i]);           /* fresh alias each iteration */
    if ((*__cl_tmp1)) { FreeString((*__cl_tmp1)); }   /* O3 replacement */
    (*__cl_tmp1) = cl_std_text_stringOf(cl_str("c"));
}
```

### Key Decisions

| Decision | Outcome | AR |
| -------- | ------- | -- |
| Plan scope | DEF-13 + DEF-14 only | AR #1 |
| Element alias mechanism | Per-iteration `raw` temp, no write-back | AR #2 |
| Local identity mechanism | Unique IR spelling per binding | AR #3 |
| Replacement semantics | O3 drop-then-store on the element | AR #6 |
| Exit paths | The alias is the element; every exit observes the write | AR #7 |
| New place variant | `PlaceBase::DerefTemp { temp, ty }` | AR #11 |
| Test suites | `codegen_ownership_spec_test.rs`, `codegen_ownership_impl_test.rs` | AR #12 |
| Verification | `make check` per task; `make verify` before completion | AR #5 |

### Related Files

- `compiler/src/own/loops.rs` — element-loop checking and binding
- `compiler/src/own/statements.rs` — `break`/`continue` write-back hooks (removed)
- `compiler/src/own/flow.rs` — `ExitFrame`, `Binding`
- `compiler/src/own.rs`, `compiler/src/own/lower.rs` — binding declaration and `Function.locals` builders
- `compiler/src/ir/place.rs`, `compiler/src/ir.rs` — place bases and locals
- `compiler/src/codegen/function.rs`, `compiler/src/codegen/expr.rs` — C emission of places and locals
- `compiler/tests/codegen_ownership_spec_test.rs`, `compiler/tests/codegen_ownership_impl_test.rs` — new test suites
