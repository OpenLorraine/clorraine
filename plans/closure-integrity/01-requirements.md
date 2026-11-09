# Requirements: Closure Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The closure and callable surface of the v1 language has four major and two minor holes: a nested
lambda that relays an unlisted capture, a capture list that names a module-level binding, a dotted
capture write that reports the wrong diagnostic, a lambda that returns its captured owner and
double-frees, a captured func-typed binding whose owned result leaks, and a call of an unbounded
type parameter that emits invalid C. The plan conforms the implementation to the frozen capture
and ownership rules where those rules already decide the behavior, and lands two new diagnostic
rows (`E10406`, `E10307`) for the two gaps the spec leaves open.

## Functional Requirements

### Must Have

- [x] R1 (DEF-92): a capture entry inside a nested lambda that names a binding the enclosing
      lambda's environment cannot reach is rejected with `E10405` at the entry; the documented fix
      is to list the name in the enclosing lambda, after which the relay builds and runs.
- [x] R2 (DEF-93): a capture entry that names a module-level binding (`let`, `const`, or an
      imported name) is rejected with the new `E10406`; reading the same module name without a
      capture list stays accepted.
- [x] R3 (DEF-94): a write through a listed copy capture reports `E10402` (dotted targets
      included) and never `E10024`; a `[borrow ...]` capture write and a copy capture read stay
      accepted.
- [x] R4 (DEF-104): a lambda that returns its by-value non-duplicable capture is rejected with
      `E10401` — for the concrete spelling directly, and for a generic spelling per instance; a
      duplicable instance keeps copying and stays accepted.
- [x] R5 (DEF-109): a captured func-typed binding called inside the lambda resolves to the capture
      (not a same-named module lender), so an owned result is bound and released exactly once
      (ASan-clean).
- [x] R6 (DEF-110): a call of a value whose type is a type parameter without a callable bound is
      rejected with the new `E10307`; a callable-bound parameter stays callable.

### Should Have

- [x] R7: the two new codes land with their registry row, compile-fail golden, and compile-fail
      driver registration in the same phase (AR-12).

### Won't Have (Out of Scope)

- Everything in the backlog outside the six defects (AR-1): the evaluation/emission batch (DEF-95,
  DEF-96, DEF-99, DEF-107, DEF-111, DEF-117, DEF-118, DEF-119, DEF-120, DEF-121, DEF-122, DEF-124,
  and the remaining minors), the DX slice (PLAN-05), the fuzzing harness, and PLAN-13/14.
- Any change to the capture model itself: implicit relay, environment ownership of captures, and
  callable bounds beyond the existing `func(...)` form were offered and declined (AR-2, AR-3,
  AR-5); the plan rejects instead.

## Technical Requirements

### Compatibility

- The frozen chapters stay unchanged except the two authorized registry rows (AR-2, AR-3). Every
  other fix conforms the implementation to rules the specification already states (L-2, L-3, L-4,
  the E10405 listing rule, G-1/G-6).
- Programs that were previously accepted but produced invalid C or memory errors may now be
  rejected; their fixes are one-line (add the enclosing capture, drop the module capture, write
  `return x;` directly, declare the callable bound).

### Security

- Memory safety is the surface: the DEF-104 double free and the DEF-109 leak are pinned by host
  AddressSanitizer probes (AR-13); every rejected shape writes no translation unit.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Batch membership | six defects / five / eight | six | agreed batch A | AR-1 |
| Module-level capture rule | reject with E10406 / support | reject | module names need no capture; the support path adds a second capture mode | AR-2 |
| Unbounded callable rule | reject with E10307 / defer | reject | the only sound outcome; the grammar already offers `T: func(...)` | AR-3 |
| Relay report | reuse E10405 / new code | reuse | the frozen listing rule already states the behavior | AR-4 |
| Captured-owner return | reject per E10401 / env owns | reject | conforms to L-2/L-3; the alternative changes the capture model | AR-5 |
| E10406 scope | let/const/imports / const only | let/const/imports | one rule, one reason | AR-8 |
| Phase structure | per layer (3) / per defect (6) | per layer | shared files and one review surface per layer | AR-15 |

> **Traceability:** every scope decision back-references its register entry. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Acceptance Criteria

1. [x] All six requirements verified by the ST cases in [07](07-testing-strategy.md) via the spec
       suite, and R4/R5 additionally by the host ASan probes.
2. [x] Both new codes registered, asserted by their goldens, and covered by `./spec/check.sh`.
3. [x] No regression: `make check` green, and the existing closure/capture suites
       (`v1_language_*`, `def_backlog_*`, `call_value_integrity_*`, `borrow_generic_integrity_*`,
       `residual_defect_completion_*`) stay green.
4. [x] `make verify` legs passed at close-out.
