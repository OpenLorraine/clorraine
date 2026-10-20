# Findings

## Phase 1 — Run-time-index view store (`DEF-529`)

**Rows**: `DEF-529`.
**Status**: Done (2026-10-20).
**Commits**: `2aa065a` (red), `5ca8c08` (fix, implementation pin).

The lowering keeps a non-fixed-array place source at an opaque or type-parameter
element store. A `str` instance widens the place in place and leaves the
caller's owner readable; an owned instance still moves and empties the source.
A call-result source keeps the ownership path, and a fixed-array source keeps
the row-store move. The per-instance audit still reports `E10034` for an owned
instance that would move out of a borrow.

**Verification**: `should_keep_a_place_source_at_a_runtime_view_store` (run and
ASan), `should_store_a_place_source_in_place_at_a_runtime_view_store` (emitted
shape), the focused suites, and `make check` at the batch head.

**Review**: correctness, memory-safety, semantics, and emission all called the
fix sound and memory-clean. The adjacent pre-existing conditional-of-places
rejection at a view instance is filed as `DEF-535`.

## Phase 2 — Non-call effectful `len` operands (`DEF-531`)

**Rows**: `DEF-531`.
**Status**: Done (2026-10-20).
**Commits**: `ca0ba15` (red), `0e3b74e` (free spelling), `8786f23` (review
corrections: every spelling, effects-only walk, pins), `7fee974` (confirmation
corrections: discarded leaf/root binding).

A free, bare, or dot `len` of a fixed-array conditional, array literal, or
run-time index evaluates the operand's effects and still folds the structural
extent. A conditional runs only the selected arm, a literal runs each element
once, and an index runs its index expression and bounds guard once. A pure place
still folds; a direct call keeps the guard materialization.

The review correction replaced the unconditional whole-operand discard with an
effects-only walk. A conditional that mixes a caller place arm with a fresh arm
now runs only its condition: discarding the whole conditional moved and freed
the caller's fixed-array owner while the checker still considered it live, an
ASan heap-use-after-free. The bare `len(...)` and dot `xs.len()` spellings now
share the same evaluation, so the three spellings agree.

**Verification**: `should_evaluate_a_non_call_folded_len_operand_in_every_spelling`
(bare, dot, and qualified; conditional, literal, index, call, owned, and mixed
place-arm cases; run under `RDC_ASAN=1`),
`should_emit_a_len_operand_without_moving_a_caller_place` (emitted shape), the
focused suites, and `make check`.

**Residual**: a non-call operand in a conditionally evaluated position (an outer
conditional arm, a short-circuit right side, or a loop condition) keeps the
structural fold and drops its effects. The guard is
`defer_move_outs == 0 && !in_loop_condition`; this is the narrower shape allowed
by T-R7 and is filed as `DEF-534`.

**Confirmation review**: the first effects-only walk lowered each array-literal
element separately, so an owning call leaf (`stringOf(...)`) emitted a bare
`(void)(call)` and leaked; a member/element chain rooted at a call leaked its
root the same way. The correction binds a discarded bare call result to a
statement temporary, binds an ownerless projection root before the discard, and
binds a fixed-array projection arm's root on that arm's path. The last change
also fixes the invalid `memcpy(&(mk().rows), ...)` emission for a conditional of
fresh member arms; the skipped arm no longer runs. The owned and member-root
cases are pinned in the every-spelling test under ASan.

## Phase 3 — Module initializer call rule (`DEF-532`)

**Rows**: `DEF-532`.
**Status**: Done (2026-10-20).
**Commits**: `1afd3ab` (red), `88b73b4` (fix), `8786f23` (review
corrections), `7fee974` (widened receiver).

Module initializers now resolve their calls the way the function-body checker
does: the current module first, then an import binding under its original name,
then the program-wide map. A module-qualified callee resolves in that module
only, and the declaring module supplies the parameter types. A written argument
or a dot receiver that names a non-`const` global reports `E10038` for a
`borrow`, `const`, or unmarked parameter, once at the written place and with no
translation unit.

A func-typed field call wins over a free function of the same name. The walk
uses the receiver's projected type (including an indexed receiver) and the
semantic struct-field table, so a projected receiver, a same-named struct in
another module, a func field, and a generic dot callee are judged by the call
they actually name. An explicit `borrow` on a generic dot callee reports; an
unmarked parameter whose declared type is a type parameter or unresolved keeps
the existing initializer oracle.

**Verification**: `should_apply_the_global_borrow_rule_in_a_module_initializer`
(direct, bare dot, `const`, module-qualified, generic direct, func field),
`should_keep_a_field_call_and_a_same_named_struct_apart_in_an_initializer`
(projected field, same-named struct, current-module order),
`should_apply_the_initializer_borrow_rule_through_the_resolved_callee` (import
alias, program fallback with a type alias, generic dot, index-projected
receiver), `should_apply_the_borrow_rule_to_a_widened_initializer_receiver`
(static-str, projected static-str, and scalar widening), the focused suites,
`make check`, and the host ASan controls.

**Review corrections**:

| Finding | Outcome |
|---|---|
| T1/N5a projected func field falsely reported | Fixed: the receiver's projected type owns the field, so the call is skipped |
| T2 same-named struct hid the field | Fixed: the semantic struct-field table supplies the field check; the shared simple type name is the pre-existing `DEF-416` |
| T3/N3 simple-name callee picked the wrong module | Fixed: current module, import binding, then program map; declarers are recovered by pointer identity |
| T4 imported alias was not resolved | Fixed: the import binding supplies the original callee |
| T5 parameter types resolved in the caller module | Fixed: the declaring module supplies them |
| T6 generic dot callee was skipped | Fixed: the declaration-level receiver mode applies; only an unresolved unmarked parameter is exempt |
| N5b indexed receiver was dropped | Fixed: the receiver's sema type is the element type and the root global is still judged |
| Widened receiver (`static str`/`u8`) skipped the report | Fixed: the call has already been type-checked, so the walk no longer compares the receiver type; the func-field guard is the only exclusion |
| T7/N4 func-typed global/field call arguments are not judged | Filed as `DEF-537` |
| T9 two implementation pins were retargeted unnecessarily | Restored to the original global-slice shapes |

**Oracle conflict**: `DEF-533` is extended. The frozen oracles accept
`slice(xs)`, an unmarked view global, an unmarked scalar receiver or argument,
and a module-qualified global receiver, while BR-9 and the function-body twin
reject them. The walk keeps those shapes accepted until a user ruling retargets
the oracles or amends BR-9.

**Pre-existing residuals**:

| Row | Finding |
|---|---|
| `DEF-533` | Module-initializer global-borrow oracle conflict (extended) |
| `DEF-534` | Conditionally evaluated non-call `len` operands keep the fold |
| `DEF-535` | A conditional of borrowed places is rejected at a view-instance opaque store |
| `DEF-536` | A bare discarded fixed-array conditional moves a place arm (ASan UAF) |
| `DEF-537` | Func-typed global/field initializer calls skip the borrow rule |
| `DEF-416` | Same-named types in two modules share one type-table entry; the T2 front-end pin stops before the invalid C |
