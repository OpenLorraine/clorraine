# Current State — Deferred Evaluation Completion (Batch V)

Baseline `55b6419`.

## `DEF-534` — deferred `len` effects

The structural `len` branch (`own/lower.rs`) folded the extent and called
`evaluate_len_operand_effects` only when `defer_move_outs == 0 &&
!in_loop_condition`. In an outer `?:` arm, a short-circuit right side, a loop
test, or a deferred body, a `len([mk()])` returned the constant without running
`mk`; a conditional or indexed operand behaved the same. The fix cannot emit a
statement at the fold position: the statement would run on a skipped arm or
once instead of every loop test.

## `DEF-535` — conditional opaque store

`record_opaque_store` recorded one source place and returned
`owned_source && has_source_place`. A conditional source has no single place, so
the function recorded its duty but returned false; the assignment then marked
each owned arm as a maybe-move, and the lend audit reported `E10034` for both
borrowed `String` arms. The view audit also added a synthetic statement
temporary because the conditional's owned arms had no named frame, producing
`E10030`.

## Review surface

The batch changes IR (`DiscardThen`, `OpaqueSource`, `fresh_arm`), codegen
(comma/statement discard, conditional fills, per-arm stores), ownership flow
state (definite re-initializations), and the opaque-store audit. Each change is
reviewed for memory safety and emission validity.
