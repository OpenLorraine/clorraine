# Store Emission: Conditional Bases and Guarded View Literals: Residual Major Closeout II

> **Document**: 03-05-store-emission-conditional-literals.md
> **Parent**: [Index](00-index.md)

## Overview

Two batch-J phase-3 residue rows share the store/literal emission surface: an element store
through a conditional or projected base is accepted and silently dropped (the RHS side effects
with it), and a guarded conditional of call-filled view-array literals either fails the strict
build (the `take` spelling) or runs a skipped arm's call (the plain spelling). `DEF-322`'s fix is
the frozen place rejection at the assignment-place judgment; `DEF-323`'s fix extends the existing
guard-literal machinery (`own/lower.rs`).

## Design

### 1. The element store through a conditional base (`DEF-322`)

`(c ? b1 : b2).ys[i] = 7;` emits no store and drops the RHS (`def322_probe.clo` runs 251; the
scalar member twin is rejected `E10221`). The assignment target must be a place (Ch 04 §4 /
EX-14: a local, a parameter, a struct field, an array element, or a dereferenced raw pointer),
and a conditional base is not one; the read spelling `(c ? b1 : b2).ys[i]` is accepted because a
read needs no place, so it does not sanction a store.

Report the frozen `E10221` (`left side of an assignment must be a place`) at the assignment-place
judgment — the same rule and site that reject the scalar member twin (`sema/ops.rs`; the executor
re-resolves) — so the store is never silently dropped and the RHS side effects never run. The
per-arm route is parked (AR-6/PF-001): it would add an acceptance the frozen place list does not
define and would accept what the scalar twin rejects. The read spelling and the scalar rejection
are the controls.

### 2. The guarded conditional of call-filled view-array literals (`DEF-323`)

`g && consume(ok ? Mixed { names: n1(), tag: … } : Mixed { names: n2(), tag: … })` — the `take`
parameter spelling is accepted and the strict build fails (`too few arguments to function
'cl_..._n1'`: the hidden fixed-array destination is dropped); the plain parameter spelling builds
and runs the skipped arm's call (run 10: `calls` = 3 where 2 is correct). The scalar and
owned-field twins are fixed; `literal_field_needs_stores` (`own/lower.rs:4512`) returns false for
a view-array field, so `materialize_guard_literal` (`:1680`) never binds the literal and the
brace-list spelling reaches the guard unreached.

Extend the guard-literal materialization to the view-array elements: a view-array field whose
elements need binding (`view_element_needs_binding`, the existing predicate) routes through the
destination-store shape the scalar/owned twins use, so the arm's call writes the hidden
destination on the evaluated path and the literal's fresh roots release per taken evaluation.
The plain spelling then runs only the taken arm's call; the `take` spelling passes the
destination. The direct-literal take control (builds and runs 0) and the scalar/owned twins stay
byte-identical controls.

## Integration Points

- `compiler/src/own/lower.rs:4488` (`literal_fields_need_stores`), `:4512`
  (`literal_field_needs_stores` — the view-array early return), `:4534`
  (`view_element_needs_binding`), `:1680` (`materialize_guard_literal`), the store lowering and
  the conditional handling (`mark_read_conditional_move`).
- The guard emitter (`codegen/guards.rs`) and the statement/expression fill shapes the fixed
  scalar/owned twins already use.
- The `def322` scalar twin (`E10221`) and the read spelling as controls.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A conditional or projected base at an assignment target | The frozen `E10221` rejection (the place rule; the scalar twin's shape) | AR-6 |
| A guarded literal whose view elements still need a store | The materialized destination path; the element binding predicate decides | — |
| A `take` argument on the skipped path | The deferred per-arm store runs only on the evaluated path | — |
| An evaluation-order change for the RHS | The RHS keeps its source position; the store tests pin the side-effect count | — |

## Testing Requirements

- Spec: the conditional-base store (both selections rejected once with the frozen `E10221`, no
  TU, the RHS side effects never run), the read spelling (accepted, runs), the guarded conditional
  of call-filled view-array literals (both parameter spellings; the skipped path runs no call;
  ASan-clean), the direct-literal take control, and the scalar `E10221` control.
- Impl: the rejection's code and span (no TU), the guard-literal destination fill,
  the call-count shapes, the controls byte-identical; a re-emission determinism pin.
