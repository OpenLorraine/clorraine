# Borrow-View Moves and Shadowed Narrowing Implementation Plan

> **Feature**: Reject ownership transfers through a `for (borrow e in xs)` element view (`DEF-18`), and resolve a shadowing initializer against the outer narrowed binding (`DEF-20`)
> **Status**: Planning Complete
> **Created**: 2026-09-29
> **Implements**: DEF-18, DEF-20
> **CodeOps Artifact Schema**: 1

## Overview

Two checker-semantics defects remain on the ownership surface. First (`DEF-18`),
a `for (borrow e in xs)` element view binds as an ordinary local, so the
borrowed-parameter move rule never sees it: `eat(e)`, `let s: String = e;`, a
field move, an aggregate-literal field, and a `return e;` through the view are
accepted and the element is transferred out of the array through a lend the
specification says never owns its element. Second (`DEF-20`), a declaration that
shadows a narrowed nullable binding (`if (b != null) { let b: Buffer = b; ... }`)
reports a spurious `E10010`, because the declaration is processed before the
initializer's null-narrowing check.

Both violate the frozen specification: `spec/03` sections 3 and 4 state that
moving an element out of an array is an element move and that element moves are
`E10026` in v1, and `spec/08` section 5 states that the loop variable never owns
the element and cannot leave the loop body. `spec/02` and `spec/16` define the
null narrowing a `!= null` check creates.

The fix is small and direct: the shared move decision rejects a transfer whose
binding is a borrow view, exactly as it already rejects a transfer out of a
borrowed parameter; and the declaration path runs the initializer's null check
before the shadowing binding is declared, matching the pre-declaration
resolution the store preparation already uses.

## Minimum-Sufficient Baseline

**Original goal:** moves through a borrow element view are rejected with the
specification's element-move diagnostic, and a shadowing initializer under a
narrowed nullable resolves to the outer binding.

**Smallest viable design:** one guard in `FunctionChecker::move_place`
(`own/place.rs:182`) for a `BindingKind::Local` binding with `Mode::Borrow`,
reusing the existing `E10026` reporting; and one reordering in `check_let` and
`check_const` (`own/statements.rs`) so `check_value_into` runs before the
shadowing binding is declared. No new type, function, layer, dependency,
harness, or refactor.

**Excluded machinery:** None — no new support surface was considered.

**Approved complexity:** None.

## Document Index

| #   | Document                                                       | Description                                 |
| --- | -------------------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)                 | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                           | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                             | Feature requirements and scope              |
| 02  | [Current State](02-current-state.md)                           | Analysis of current implementation          |
| 03-01 | [Borrow-View Moves](03-01-borrow-view-moves.md)              | The element-view move rejection             |
| 03-02 | [Shadowed Narrowing](03-02-shadowed-narrowing.md)            | The initializer resolution order            |
| 07  | [Testing Strategy](07-testing-strategy.md)                     | Test cases and verification                 |
| 99  | [Execution Plan](99-execution-plan.md)                         | Phases, sessions, and task checklist        |

## Quick Reference

### Usage Examples

```clorraine
func consume(take s: String): void { }

// Rejected: the element view never owns its element (E10026).
func steal(borrow xs: String[2]): void {
    for (borrow e in xs) {
        consume(e);
    }
}

// Accepted: the narrowed outer binding flows into the shadowing initializer.
import { Buffer, bufferNew } from std.mem;

func keep(flag: bool): i32 {
    let b: Buffer? = bufferNew(8);
    if (b != null) {
        let b: Buffer = b;      // no E10010
        return 0;
    }
    return 1;
}
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| `DEF-18` diagnostic | `E10026`, the existing element-move row (AR-3, AR-7) |
| `DEF-18` fix site | The shared `move_place` decision (AR-9) |
| `DEF-20` fix | The initializer's null check runs before the shadowing declaration (AR-5) |
| `const` variant | Included in `DEF-20` (AR-6) |
| Verification | `make check` per task, `make verify` at close (AR-8) |

## Related Files

- `compiler/src/own/place.rs` — `move_place`: the shared move decision
- `compiler/src/own/loops.rs` — `bind_for_in_element`: the borrow element binding
- `compiler/src/own/statements.rs` — `check_let`, `check_const`: the declaration path
- `compiler/src/own/expr.rs` — `check_value_into`, `check_nonnull_expr`, `plain_binding`
- `compiler/tests/borrow_view_and_shadowing_spec_test.rs`, `compiler/tests/borrow_view_and_shadowing_impl_test.rs` — new suites
- `compiler/src/own/tests.rs` — unit coverage for the rejection
