# Findings

## Phase 1 — Deferred `len` effects (`DEF-534`)

**Status**: Done (2026-10-21).
**Commits**: `7c4b62d` (implementation), `1434e64`, `66b29b2`, `c7f6078`, `7a40d6d`
(review corrections and tests).

A fixed-array `len` in an outer `?:` arm, a short-circuit right side, a loop
test, a for step, or a deferred body now folds the extent but runs the
operand's effects on the evaluated path. A place operand stays a read; a fresh
owned operand releases with the statement form; a loop test re-evaluates and
releases every test; a conditional loop test and a for step are materialized per
evaluation; an index/divisor/slice-count operand materializes once.

**Verification**: `should_evaluate_a_deferred_len_operand_on_the_evaluated_path`
and `should_evaluate_a_deferred_len_operand_once_without_leaking` (outer arm,
short-circuit, while/do/for, conditional test, for step, defer, index guard and
skip, mixed place/fresh, owned literal, struct member, growable container,
nested condition, nested return/call, nested view store, single evaluation); both pass under
`RDC_ASAN=1`. `make check` and `make verify` exit 0.

## Phase 2 — Conditional opaque store (`DEF-535`)

**Status**: Done (2026-10-21).
**Commits**: `7c4b62d`, `1434e64`, `66b29b2`, `c7f6078`, `7a40d6d`.

A conditional of owned places into `Wrap<T>` is now instance-judged: `T=str`
widens and is accepted; `T=String` reports `E10034` per non-take lend arm. The
audit names the arm, distinguishes `E10020`/`E10025`/`E10026`, reports distinct
arm offenses together, and tracks definite re-initializations by flow state so
a branch write does not hide the other path's use. A fresh arm at a view
destination reports `E10030` at the store.

**Verification**:
`should_store_a_conditional_of_borrowed_places_at_a_view_instance` and
`should_judge_a_conditional_opaque_store_by_instance_and_path` (generic,
concrete, non-duplicable, owning, local, fresh, mixed reports, partial whole,
conditional/unconditional/both-path re-init); pass under `RDC_ASAN=1`.

## Review record

| Area | Finding | Resolution |
|------|---------|------------|
| Deferred len | A whole-value fill moved a place arm (batch-T UAF) | Effects-only walk; place arm stays a read |
| Deferred len | Growable `Array<T>` re-fill leaked | Statement-form fill and release |
| Deferred len | Conditional loop test / for step leaked | Per-test materialization; per-arm statement fill |
| Deferred len | Member/index effect leaked a fresh root | Materialize the root; statement guard walk |
| Deferred len | Nested conditional condition dropped effects | `guarded_value(condition)` and per-arm statement fill |
| Deferred len | Materializing an owned conditional double-freed its String | Keep the materialization to duplicable, non-owning, non-view conditionals |
| Deferred len | Index/divisor/slice count re-evaluated | `DiscardThen` is impure; materialized once |
| Opaque store | Branch re-init hid the other path's use (UAF) | Flow-sensitive definite re-initialization set |
| Opaque store | Fresh arm beside a local arm accepted and leaked | Fresh-arm frame; store-time `E10030` at a view destination |
| Opaque store | Arm reports suppressed each other | Report all distinct arm offenses |
| Opaque store | Arm name/help text differed | Field path and registry wording |
| Caller-retained field | A nonduplicable field/element store of another lend's view | Closed in `plans/caller-view-escape`; one `E10033`, duplicable struct copies and same-carrier/static stores accepted |
| Generic root/value | A type-parameter value or a concrete view target under a generic root was missed | `maybe_view_only` duty and per-instance substitution; owning instance keeps the ordinary move path |
| Array/slice root | A duplicable array/slice was treated as a copy | `copies_whole_value` distinguishes a struct copy from a decayed array/slice pointer |
| Nested capture | A relayed whole-out-slot capture lost its kind | `capture_whole_out_slot` relays through the capture map |
| Static-return call | A static-returning named call was treated as a fresh view | The call-site source summary uses the callee's return-expression names; static/module-backed results are not statement temporaries |
| Residual | Caller-side stored-view invalidation | Container field/element stores closed in `plans/caller-view-escape`; a whole `borrow T?` out-slot assignment from another lend remains `DEF-541` (critical, needs a ruling against the existing AR-20 acceptance) |

**Final verification**: `make check` and `make verify` exit 0 at the code head;
the new cases pass under `RDC_ASAN=1`; the emission matrix shows no
accepted-source-invalid-C case and byte-identical re-emission on the changed
units.
