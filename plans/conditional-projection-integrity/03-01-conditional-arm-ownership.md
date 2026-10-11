# Phase 1: Conditional Arm Ownership

> **Document**: 03-01-conditional-arm-ownership.md
> **Parent**: [Index](00-index.md)

## Overview

Three defects where an owned value flowing through a conditional is not moved out per arm: a
projection of a mixed conditional in a consuming position (DEF-176), a guarded intrinsic argument
of an owned conditional projection (DEF-178), and a nested conditional arm's owned store (DEF-179).
Two are criticals (double frees); one is a major (a leak that became a use-after-free under the
reverted batch E attempt). The invariant every fix preserves: **exactly one release per created
owner**, with the place arm left readable and its remaining fields released once by their own owner.

## Current Arc

- `materialize_owned_temporaries` (`own/lower.rs:1894`) with `conditional_borrows_place`,
  `conditional_needs_arm_fill`, and `conditional_has_ownerless_projection_arm` decides whether a
  conditional is bound as one owner, hoisted per arm, or left inline.
- `bind_guarded_projection_root` (`own/lower.rs:1931`) binds a projection's call root through the
  deferred `ValueKind::Materialize` fill; `bind_intrinsic_argument` (`own/lower.rs:2004`) handles
  owned intrinsic arguments, with the conditional-argument gate routing through
  `hoist_ownerless_conditional_arms` / `own_ownerless_conditional_arms`.
- Present failures: the consuming store copies the select without moving the chosen arm's member
  (the fresh arm's root releases the source field the copy aliases — double free); the guarded
  intrinsic argument materializes into a released temporary aliasing the place arm (double free);
  the nested conditional's arm store copies instead of moving the member out of the chosen root
  (double free).

## Proposed Changes

### 1. Per-arm member move-outs for consuming projections — `compiler/src/own/lower.rs`

For a projection of a conditional whose value is consumed (a store, a return, a `take` argument, a
defer body), lower per arm instead of copying the select:

1. Evaluate the condition; per arm, bind the chosen root (the place stays a read, the fresh call
   result already has an owner through the arm-fill machinery).
2. Copy the selected member into the consuming destination and **zero it in the chosen arm's root**
   (the O7 empty state), so the arm's root release frees only the un-moved fields and the consumed
   value has exactly one owner.
3. Keep the evaluation order: the condition, then the chosen arm only.

The reverted batch E attempt wrong for exactly one reason: it bound the whole conditional as one
owner, which emptied the place arm (a later read saw freed storage). The design forbids that shape:
the place arm is only read; the move-out is written back to the chosen arm's root through its own
storage.

### 2. Guarded intrinsic argument — `compiler/src/own/lower.rs`

Extend `bind_intrinsic_argument`'s conditional gate to the **guarded** argument shape
(`c && len((c ? b : mk()).v) > 0`, and the `||`/`?:` operands): the projection's root binds through
`bind_guarded_projection_root` with the guard's deferred fill, so the argument's owner releases on
the evaluated path exactly once.

### 3. Nested conditional arm — `compiler/src/own/lower.rs`

Make the arm selection recursive: a nested conditional inside an arm is itself lowered by the same
per-arm move-out shape (change 1), so the member moves out of the innermost chosen root and each
level releases only its un-moved fields.

## Tests

| Case | Shape | Expectation |
|------|-------|-------------|
| `armstore` | DEF-176's store, both arms | ASan-clean; the place arm stays readable; one release per owner |
| `armreturn` | a projection-conditional return | builds; ASan-clean; one release |
| `armtake` | a projection-conditional as a `take` argument | builds; ASan-clean |
| `armdefer` | the same inside a `defer` body | builds; ASan-clean |
| `armguard` | DEF-178's guarded intrinsic argument, both arms | ASan-clean; one release of the fresh arm |
| `armnested` | DEF-179's nested store, both inner arms and both outer arms | ASan-clean; one release per root |
| `armplace` | the accepted controls: the place arm read before and after the store; the mixed-lend pins from batch E (`should_keep_a_mixed_conditional_lend_arguments_place_arm_readable`, `should_release_both_owning_conditional_view_arms_siblings`) | unchanged behavior; byte-identical emission where nothing is consumed |

## Acceptance

- Every double-free shape is ASan-clean at every spelling; every accepted control stays accepted
  and (where it does not consume a projection) byte-identical.
- The phase's spec suite is red at the baseline (the three double-frees) and green at the head.
- No new diagnostic; the emitted C builds strictly on `host`, and the ownership shapes emit for
  `hosted-amiga`/`freestanding` in the phase verification.
