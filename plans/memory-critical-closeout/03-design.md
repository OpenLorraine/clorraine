# Design

Each fix extends an existing route. No new diagnostic, registry entry, or `spec/` chapter.

## Phase 1 — View escapes through carriers and calls (`DEF-449`, `450`, `451`, `453`)

### `DEF-449` — the conditional carrier copy loses element loans

The let/assignment copy records **element loans** only for a view-like destination (`is_view_like`)
and only a constant place source is transferred. A carrier (`Holder` with a `str[2]` field) is not
view-like, so the copy records carried **owners** but no element loans, and a later write through
the loaned owner is not recognized.

Fix: when a copied value is a conditional, array literal, or another value whose view positions the
composite collector understands (`collects_copy_positions`), record its copy loans into the
destination's paths even when the destination is a carrier, not only a single view. Use the existing
`record_copy_element_loans` with the lowered value so both arms contribute; do not transfer the
sources (a copy leaves both source carriers owning their views). Apply the same rule in the let path
(`own/statements.rs`) and the assignment path (`own/expr.rs`). The destination is a fresh binding or
a whole-value store, so clearing the positions under the destination path is correct.

Acceptance: a write through the loaned owner after the conditional copy reports the concrete twin's
diagnostic once; the mixed place/literal and unconditional-overwrite variants match; the emitted C
for the accepted controls is unchanged.

### `DEF-450` — the cross-frame out-slot rule ignores aggregate arguments

The caller-side rule inspects only view-like arguments, so a `Box` argument carrying a view field
is treated as caller-backed. The callee-side out-slot store then accepts a view derived from the
caller's local.

Fix: extend the cross-frame judgment to an argument whose resolved parameter type can carry a view
(recursively through struct fields and fixed arrays). The judgment uses the same storage-root walk
the view-like case uses; a root in the caller's frame whose view reaches a callee out-slot reports
the existing escape diagnostic at the call. A parameter-derived view that fits the destination's
frame stays accepted.

Acceptance: the filed two-frame repro reports once at the call; the same-frame twin stays accepted;
ASan/LSan clean for the accepted twin.

### `DEF-451` / `DEF-453` — call-mediated views are invisible to the loan walk

`view_loans`, `collects_copy_positions`, `lowered_root_bindings`, and the global-destination
fallback stop at `ValueKind::Call`; the lowering has already bound the result's owner to a
generated local, but the checker never names it. A view from `pick([mk().v; "EE"], 0)` or from
`pick(xs, 0)` into a global therefore records no loan.

Fix: add the `Call`/`ClosureCall` arm to the lowered root walk. The value's call result that owns
storage is already bound by `materialize_owned_argument`/`bind_ownerless_projection_root`; the walk
returns that binding (the destination temp or generated owner) as the loan owner. Extend
`value_loans`/`collects_copy_positions` so a call whose result is a view or whose arguments carry
views contributes those owners. The global-destination fallback then consults the same value loans,
so `g = pick(xs, 0)` reports `E10033` once and the inline temporary form `pick([mk().v; "EE"], 0)`
reports `E10030` at the call.

Acceptance: the three filed escapes report once; the caller-backed, module-level, and
within-statement reads stay accepted and clean; the accepted controls are byte-identical.

## Phase 2 — Take alias capture (`DEF-459`)

`materialize_owned_argument` pins the pointer of an owning deref take, but `pin_deref_projection`
uses `materialize_operand`/`defer_operand_fill`, which skip a *pure* value. `ps[idx]` is pure in the
no-side-effect sense, yet it reads mutable storage (`idx` is a global the callee may change), so
the post-call move-out and the alias address re-read a different pointee.

Fix: for an owning deref take, capture the pointer operand of every raw-deref chain, including a
pure place, index, or deref, before the call. Only an existing compiler temp/materialize (which the
callee cannot name) is left alone. The eager path binds through the ordinary materializer; the
guarded path keeps a deferred fill so a skipped `&&`/`||`/`?:` operand still never runs. The
move-out and the alias test read the same bound operand.

Acceptance: the filed repro is ASan-clean (no double free, no SEGV, no leak); the non-mutating twin
is unchanged; the emitted move-out and alias test use the captured pointer; re-emission is
byte-identical.

## Phase 3 — Generic conditional duties (`DEF-468`, `DEF-469`)

`judge_return_moves` decides the source list from the **return type**. When the return type is a
concrete owning type but an arm's type is a type parameter, `conditional_move_sources` skips the
arm ("left to its instantiation"), so no per-instance duty is recorded. The concrete twin reports
`E10034` because the arm is a lend parameter.

Fix: for a conditional return, additionally collect the arms that mention a type parameter,
including a projected member of a type-parameter base, through
`maybe_owning_move_sources`/`maybe_owning_projection_move_sources`, and record the per-instance
move with `mark_moved_if_owning`. That records the existing `RequiresNotLend` duty (and the
element-move duty when the arm is an element), which the instance audit resolves against the
substituted type. A duplicable instance stays accepted; an owning instance reports the concrete
twin's `E10034` once. Apply the same addition to the `take` path if the audit finds it missing.

Acceptance: both filed generic repros report once at the instance; the duplicable instance is
accepted and clean; no double free; the concrete twins are unchanged.

## Phase 4 — Ruling, controls, and close-out (`DEF-452`)

Ruling: a generated owner (`__cl_owned<N>`) is a compiler binding that holds a value captured by a
local or a literal, not a `__cl_tmp<N>` source temporary. Ch 03 D5 governs source temporaries; a
generated owner lives in the binding's frame and is released at that scope exit. The body-scope
binding is therefore conformant; a same-statement read stays accepted, and no next-statement read
is accepted for a value the binding does not own (the loan rules still judge escapes). Add a green
control for the same-statement read and the existing later-fill rejection, and record the ruling.
No code change.
