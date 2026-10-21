# Requirements — Deferred Evaluation Completion (Batch V)

## `DEF-534` — conditionally evaluated `len` effects

| ID | Requirement |
|----|-------------|
| V-R1 | A fixed-array `len` in an outer `?:` arm, a `&&`/`||` right side, a loop test, a for step, or a deferred body runs the operand's conditional, literal, index, cast, and call effects only on the evaluated path. |
| V-R2 | The structural extent stays a compile-time constant; a place operand stays a read and is not moved or emptied. |
| V-R3 | A fresh owned operand is released on every evaluation, including a growable container whose element drop has no C expression form. |
| V-R4 | A `len` effect used in an index, divisor, or slice count materializes once; a skipped path runs nothing. |
| V-R5 | A nested conditional condition runs its effects on the path that evaluates the outer arm. |

## `DEF-535` — conditional opaque store

| ID | Requirement |
|----|-------------|
| V-R6 | A conditional of owned places stored into a `Wrap<T>` member is judged by the instance: a duplicable `T=str` root is copied at the call and accepted; a nonduplicable caller-retained root reports `E10033` for a view of a different lend parameter; `T=String` reports `E10034` for each non-take lend arm. |
| V-R7 | A local-backed arm at a caller slot reports `E10030`; a later read of a moved arm reports `E10020`, a whole-value read after a top-level field move `E10025`, and a deeper projected arm `E10026`. |
| V-R8 | A branch re-initialization that covers only one path keeps the moved state; an unconditional or all-path re-initialization accepts later reads. |
| V-R9 | A fresh owned arm at a view destination reports `E10030` even when the carrier is never read again; a frame-local nonduplicable wrapper keeps the real write-through store. |

## General

| ID | Requirement |
|----|-------------|
| V-G1 | No `spec/` file, grammar rule, diagnostic registry entry, or generated C is edited. |
| V-G2 | Each phase runs the focused suites, `make check`, and the final `make verify`. |
| V-G3 | New behavior is pinned by a spec test; emitted shape or counts by implementation tests where the shape matters. |
| V-G4 | A pre-existing rule outside the batch's surface is filed rather than silently weakened; `DEF-540` records the caller-side view invalidation gap. |
