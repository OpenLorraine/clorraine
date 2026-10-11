# Phase 2: Generic Instance Conformance

> **Document**: 03-02-generic-instance-conformance.md
> **Parent**: [Index](00-index.md)

## Overview

Three defects where a generic body is judged at the declaration but never again at its
instances: an opaque `T` iterable (DEF-145), a value-form member read on a bare type parameter
(DEF-146), and a conditional-wrapped projection that defeats the substituted `len` fold
(DEF-150).

## Current Arc

### DEF-145 — the iterable duty has no instance audit

`check_for_in` (`own/loops.rs:470-500`) accepts an iterable whose type is a fixed array, a
slice, a `TyKind::Param`, or `TyKind::Unknown`; the comment records the intent — "an opaque
type parameter may be either once its instance is known, so it keeps its accepted path and is
judged at the instance" — but no instance judgment exists. The verification matrix:

| Instance | Behavior |
|----------|----------|
| a scalar-element array (`i32[4]`) | builds and runs (the working reference) |
| a struct-element array (`S[2]`) | binds the element as a raw pointer over the struct — strict build fails |
| a scalar (`i32`), an enum | not iterable — the emitted loop is invalid C |
| a null-shaped local (`let x = null; for (v in x)`) | the emitted bound compares unsigned `< 0` — strict build fails |

The fix mirrors the established per-instance audit pattern (`own/instance_check.rs:40-56`,
fed by `sema.capture_moves`, `sema/mod.rs:71`): the declaration records the duty, and the audit
substitutes each instance's type arguments.

### DEF-146 — the value form skips the member rule

`member_type` (`sema/expr.rs:538-560`) reports `E10212` for an unknown struct or enum member;
a bare type-parameter base falls to `_ => Ty::unknown()` silently. The call form reports
(`call.rs:445-462`), the value form does not. The frozen rule (Ch 04 §8) is about the
parameter having no member surface at all — reads included.

### DEF-150 — the conditional arm defeats the fold

The structural fold resolves substituted member chains (`codegen/expr.rs:280-310`,
`resolved_value_ty`) but not a `Conditional` value, so `len(flag ? b.s : b.s)` in a generic
body reaches the emitter's unknown path and yields `0` (verified: the `B { s: i32[7] }` instance
emits `return 0;`). The fold's contract (the batch F structural rule) evaluates nothing — the
condition and the arms' stores must not emit.

## Proposed Changes

> **Superseded (Phase 2 fix round):** the element rule below was replaced by the instance-typed
> element binding — the binding carries a placeholder resolved from the substituted argument's
> one-step element, so struct, row, and float elements are accepted and only a non-array
> substitution reports `E10043`, with `E10040` for a non-borrow non-duplicable element. See the
> register's AR-4 and the ledger's fix round.

### 1. An iterable duty and its instance audit (DEF-145)

- `sema/mod.rs`: add `iterables: Vec<IterableRequirement>` (the function's name, the
  iterable's declared type, the span), recorded by `check_for_in` when the iterable's type is a
  `Param`; the `Unknown`-and-null-shaped case is rejected at the declaration with `E10012`
  (Phase 4's null rule; the shared predicate lands here).
- `own/instance_check.rs`: add `check_iterable_requirements` to the audit — for each
  requirement, substitute each instance's arguments; the first substituted type that is not a
  fixed array or a slice reports `E10043` at the site (one report). A struct-element array
  instance is probed against its concrete body twin; if the concrete twin builds, the instance
  emission is fixed to match (the typed element view); if not, the element rule rejects it with
  the same `E10043` family at the instance. The probe decides; the phase pins the matrix.

### 2. The value-form member report (DEF-146)

> **Superseded (Phase 2 fix round):** the report lands per instance (the register's AR-5), with
> the substituted display and the kind dispatch mirrored from the value-form checker; the
> declaration keeps accepting the read.

The audit's fallback reports `E10212` with the existing message
(`'{name}' is not a member of '{type}'`) once at the first offending instance.

### 3. A conditional arm in the substituted projection (DEF-150)

`resolved_value_ty` (`codegen/expr.rs:280`) gains a `Conditional` arm: resolve the arms'
common projection through the substituted base (both arms naming the same member resolve to
that member's type; different types stay `Unknown` and keep today's behavior). The fold then
sees the fixed array and yields the extent, and because the fold remains structural, the
receiver's stores still never emit. The mixed-arm case (different extents) keeps the runtime
path unchanged.

## Tests

| Case | Shape | Expectation |
|------|-------|-------------|
| `iterstruct` | `each<S[2]>` (DEF-145) | the pinned outcome: builds, or one `E10043` |
| `iterscalar` | `each<i32>` | one `E10043` at the site |
| `iterenum` | `each<E>` | one `E10043` at the site |
| `iterok` | `each<i32[4]>` | builds and runs (the unchanged reference) |
| `iterborrow` | a `for (borrow e in xs)` instance | unchanged |
| `memval` | the value-form member repro (DEF-146) | one `E10212`; the call form's report unchanged |
| `condfold` | the DEF-150 repro | builds; runs returning 7; the emission pin shows the folded constant |
| `condfolddrop` | the fold with a side-effecting receiver | the condition and the arms' stores never emit; runs |

## Acceptance

- The rows' reproductions report exactly the pinned diagnostic (or build and run); each case
  failed at the phase baseline.
- The `iterok`/`iterborrow` references and the concrete-array body twin are unchanged.
- The corpus differential shows only intended deltas (new rejections emit no C).

## Open Item

DEF-145's struct-element arm is resolved by the in-phase probe matrix (the concrete body twin
decides between the emission fix and the rejection); the phase records the outcome in the
findings ledger.
