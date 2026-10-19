# Design

Each fix extends an existing route. No new diagnostic, registry entry, or `spec/` chapter.

## Phase 1 — Loan-carrying copies (`DEF-485`, `486`, `493`, `497`, `498`, `499`)

### `DEF-485` — the `const` conditional carrier copy

`check_let` records a carrier copy's union of arm positions in `record_copy_element_loans`; `check_const`
has the same view-like branch but no carrier branch. Mirror the carrier branch in `check_const`: when
the target may carry a view and the value is a conditional, struct literal, array literal, or call,
record the copy positions under the fresh binding. The `const` binding owns the same loans a `let`
would, so the later write through the backing reports `E10031` once.

### `DEF-486` — the global conditional carrier copy

The global store's carrier branch consults a place source or fresh literal storage, not a
conditional's arms. For each arm, resolve its frame-backed view owners: a local carrier contributes
its binding loans, a literal or call its own storage judgment. If any arm names this frame's storage,
report the existing `E10033` once at the store, exactly as the whole-place carrier store does.

### `DEF-493` — the plain carrier copy from a carrier-returning call

A carrier-returning call with exactly one lend candidate hands back views derived from that
argument. Add a `call_carrier_source` helper (the carrier analogue of `call_view_source`): resolve
the callee's return type and its single non-`take` lend candidate, then return the written argument
(or the dot receiver at index zero). The copy collector uses it as the call result's copy source, so
`let h = makeHolder(b.v)` records a shared loan of `b` on the destination; the later `b.v = ...`
reports `E10031` once. A call with zero or several lend candidates contributes no positions and
keeps the existing acceptance.

### `DEF-497` — the mixed aggregate carrier

`frame_owned_view_owner` returns as soon as a recorded position exists. A carrier whose recorded
position is static (`tag`) therefore hides an owned field (`name: String`). When the recorded
positions are all known-safe but the argument's type still owns viewable storage, fall through to
the local-aggregate judgment, so a local carrier reports `E10030` once at the call while a
parameter- or module-backed carrier stays accepted.

### `DEF-498` — the conditional shadow copy

`check_let` snapshots a plain place source before declaring the binding, but a conditional's arms
are collected after the declaration, so `let h: Holder = c ? h : h;` resolves both arms to the fresh
destination. Collect the copy loans **before** `declare_local` (the collector is read-only and names
the outer bindings), then apply the collected loans after the declaration. The non-shadowing
conditional copy keeps the same result.

### `DEF-499` — the alias element return

`carrier_return_owners` reads the loans of a lowered place root without resolving loop aliases. An
alias has no storage of its own; resolve the root through `alias_resolved_root`, exactly as the
view-return and call-boundary walks do, so the container's element loans reject the return with
`E10030` once.

## Phase 2 — Call-mediated view escapes (`DEF-487`, `488`, `489`, `495`, `496`)

### `DEF-487` — the function-returned call-mediated view

Extend the view return's loan walk to resolve a call's single lend source even when that source is a
container with element loans, then create the loan at the return. The existing escape rule reports
`E10030` once; a parameter-backed source keeps its acceptance.

### `DEF-488` / `DEF-489` — the inline carrier and the conditional call arm

Use `call_carrier_source` in the cross-frame argument judgment (`frame_owned_view_owner`) and in
the copy collector. An inline call result therefore names its argument's local backing, and a
conditional that mixes a static literal with a call carrier contributes the call arm's source. Both
report the existing `E10030` once at the store.

### `DEF-495` — closure, func-typed, generic, and wrapper calls

`call_view_source` and `call_loans` resolve a named declaration only. Resolve the other callable
forms in the same order as `check_call_expr`: a `func`-typed field or binding through its signature,
a call-returned callable through its result type, and a generic declaration through the call's
substitution (accept a return or parameter that mentions a type parameter, as the lowered walk
already does). `unsafe` and conditional wrappers already recurse through `view_provenance`/
`view_loans`, so they follow once the inner call resolves.

### `DEF-496` — the dot-call receiver

The cross-frame loop inspects written arguments only. Include the receiver as parameter zero with
its own mode and type, so a local or fresh receiver is judged exactly like a first argument. A
caller-backed receiver stays accepted.

## Phase 3 — Generic per-instance conditional sources (`DEF-501`..`DEF-504`)

All four record a `RequiresNotLend` duty (`mark_moved_if_owning`) that the instance audit resolves;
no new diagnostic.

- `DEF-501`: the `take` path's unresolved conditional fallback records borrow-element moves, not
  lend duties. Add the `type_param_conditional_sources` walk so the projected arm carries a duty.
- `DEF-502`: the instance audit applies a field projection to a `raw` root without stripping the
  pointer. Strip `TyKind::Raw` before the field step, as the element step already does, so the
  selected member is judged.
- `DEF-503`: a struct-literal field and a projected assignment handle a concrete conditional but
  not a type-parameter arm. Route both through `type_param_conditional_sources` and mark each arm
  maybe-owned.
- `DEF-504`: `conditional_projection_move_sources` skips a generic inner arm before applying the
  projection chain. Supplement the return's projection walk with
  `maybe_owning_projection_move_sources`, so each inner lend arm records its own duty and the
  instance reports one `E10034` per arm.

## Phase 4 — Records and close-out

Run the focused suites, `make check`, and `make verify`; dispatch the three-lens phase reviews
(correctness, memory-safety, emission-integrity; add semantics for the evaluation-order changes);
record every phase, correction, and residual in `05-findings.md`; sync the roadmap.
