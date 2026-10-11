# Phase 4: Statement-Path Discards

> **Document**: 03-04-statement-path-discards.md
> **Parent**: [Index](00-index.md)

## Overview

Three defects in the statement path — an expression statement whose value is discarded: a
discarded closure call with a maybe-owning result leaks at an owning instance (DEF-185), a
discarded call inside a conditional expression statement leaks (DEF-186), and a discarded `.len()`
statement emits an undefined `len(...)` call (DEF-256). The invariant: a discarded result that may
own storage binds to a statement temporary whose release runs exactly once, and an intrinsic
dot-call lowers like its value-position spelling.

## Current Arc

- `emit_expr_statement` (`own/statements.rs:179`) lowers expression statements:
  - the **closure-callee** branch binds only when `self.owns_drop(&ty)` — its named-call sibling
    below uses `self.owns_drop(&result_ty) || self.maybe_owns(&result_ty)` — so an owning instance
    of a generic closure call leaks (DEF-185);
  - the **named-call** branch already binds with the `owns_drop || maybe_owns` predicate;
  - the **`Call`** arm lowers through `lower_call`, which resolves the callee but does not run the
    intrinsic fold: `s.len();` and `mk().v.len();` emit `len(s)` / `len(mk().v)`, an implicit
    declaration and a link failure (DEF-256);
  - every other shape falls to `lower_discarded` (`own/lower.rs:940`): a conditional expression
    statement (`c ? make1() : make2();`) materializes the value but does not bind the selected
    arm's result, so the fresh arm's storage leaks (DEF-186).

## Proposed Changes

### 1. The closure-call predicate — `compiler/src/own/statements.rs`

Use `owns_drop(&ty) || maybe_owns(&ty)` in the closure-callee branch (parity with the named-call
sibling); a duplicable instance stays a plain call and a maybe-owning one registers the statement
release.

### 2. Bind the selected arm's result in the conditional statement path — `compiler/src/own/`

A discarded conditional whose arms call fresh owning results binds the selected arm's result to a
statement temporary (the same per-arm release the consuming store uses), or hoists per arm so each
arm's call is released on its own path; the duplicable/empty instances stay plain calls.

### 3. Route the discarded intrinsic dot-call through the value lowering — `compiler/src/own/statements.rs`

In the `Call` arm (and `lower_discarded`), lower an intrinsic dot-call (`len`, `sizeof`, and the
other folded surface) through the same intrinsic-aware path the value position uses, so the
emission calls the defined helper (or folds) instead of a bare `len(...)`. The row cross-references
`DEF-224` (the value-form `.len`), which fixes a different side of the same surface.

## Tests

| Case | Shape | Expectation |
|------|-------|-------------|
| `closurecalldiscard` | DEF-185's `run<String>` discarding a generic closure call | ASan-clean; one release at an owning instance; a duplicable instance unchanged |
| `conddiscard` | DEF-186's `c ? make1() : make2();` in a loop | ASan-clean; one release per iteration on the taken arm |
| `dotlendiscard` | DEF-256's `s.len();` and `mk().v.len();` statements | strict build OK; the value-position spelling byte-identical |
| `discardcontrol` | a discarded named call, a discarded intrinsic argument, an `assert` statement | unchanged behavior (byte-identical where the phase does not touch the shape) |

## Acceptance

- Every leak shape is ASan-clean; every emitted unit builds strictly.
- Accepted controls keep their emitted C byte-identical; the emission auditor classifies every
  corpus delta as the new statement temporary or the new intrinsic call spelling.
- The phase's spec suite is red at the baseline (the leaks and the invalid `len` call) and green at
  the head.
