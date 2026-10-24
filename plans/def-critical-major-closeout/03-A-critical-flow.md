# Critical Flow: Critical/Major DEF Closeout

> **Document**: 03-A-critical-flow.md
> **Parent**: [Index](00-index.md)

## Overview

`DEF-543` is the remaining critical: a local view is written on one branch and a
static value on another, and the store judge only sees the last assignment. The
same-frame store accepts; the false path leaves a frame-backed view in an
out-slot. This component adds the smallest source-state union needed to make the
store judgment path-sensitive at the point that matters.

## Architecture

### Current Architecture

`Binding` carries `provenance` (a view's origin), `copy_source` (a maybe-owning
local's declaration source), and `rebound` (set by `bind_value` on an
assignment). `store_source_provenance` stops following a rebound local and
returns `Local(id)`, which makes the whole-out-slot duty skip. The caller guard
is conservative for a cross-frame call, but a same-frame call has no caller
`E10030` to catch the false path. The flow-sensitive joined state already lives
in `FlowState` (`own/flow.rs`); its per-binding state is `PlaceState`. The new
source set belongs on `PlaceState`, not on the non-flow-sensitive `Binding` and
not in a parallel map: `PlaceState` is already cloned and joined by `merge`,
retained by `drop_bindings_from` after a speculative/defer rollback, and removed
by `edge_state` when a scope closes. Putting the source set there makes every
one of those lifecycle hooks apply automatically, so a compiler temporary's id
cannot be reused with a stale source.

### Proposed Changes

Keep a **flow-sensitive** bounded source set on `PlaceState`:

```rust
impl PlaceState {
    /// The sources a rebound local may hold at this program point.
    ///
    /// A vector because `Provenance` has no total order; the set is deduped
    /// and flattened from `Provenance::Composite` before it is stored.
    sources: Vec<Provenance>,
}
```

- Seed the set from the declaration initializer when it has a source.
- **Every assignment strong-updates the set**: `bind_value` replaces the
  binding's sources with `store_source_provenance(value)`. Joins happen only in
  `FlowState::merge` (branches, loop heads, `break`/`continue`, conditional
  expressions already produce a `Composite` value). Do not add a "maybe
  assignment" flag to `bind_value`; it cannot see the branch, and the merge
  already joins the arm states.
- Flatten the stored provenance with `storage_atoms` before adding it: a
  conditional or array initializer yields `Provenance::Composite`, and adding
  it as one atom would hide a parameter root.
- At a whole-out-slot store, flatten the binding's set by root and apply one
  report, parameter-first:
  - any non-`take` parameter root uses the existing foreign-store rule
    (`E10034`, naming the first parameter in set order), **except the slot's
    own parameter** (`owner == destination`), exactly as the existing
    `whole_slot_store_root` does;
  - else any local/temporary root uses the local-view escape (`E10030`,
    naming the first local or "a statement temporary");
  - a `take` root is **not** short-circuited by the set: leave it to the
    existing ordinary path, which accepts an owned `take` value and reports
    `E10030` for a `take` view that dies with the frame. The set cannot tell a
    view take from an owned take, so the existing value/target view-like
    judgment remains the discriminator;
  - an all-static or newly owned set keeps the store accepted.
- If the destination type or the stored value still mentions a type parameter,
  do not apply the concrete set judgment; leave the existing per-instance duty
  to judge the substituted pair. This keeps a safe `T = str` view instance
  accepted.
- Do not change the ordinary view-provenance path for a non-rebound local; the
  existing E10030/E10034 rules stay in place.
- The source set is bounded by the distinct live sources and is never a full
  control-flow lattice; a binding's set is replaced, not accumulated, across a
  definite overwrite.

The direct callee scan in `own/call_check.rs` is a **syntactic** scan and cannot
read the checker's `FlowState`. Extend it with a small **must-source** walk: at
each whole-out-slot store, carry the sources assigned on *every* path that
reaches the store (intersection at `if`/`switch` joins, a conservative
single-iteration treatment at loop bodies and `defer` blocks), and withhold the
caller guard only when that walk proves the callee's store is foreign. A
whole-body union or a linear prefix would see a foreign source assigned after
the store or on a sibling branch and could withhold the caller guard unsoundly.
When the walk cannot prove the store, keep the caller guard; the callee's own
report is the safety net, and a resulting duplicate is a test failure to
resolve, never an unsafe acceptance. This is a direct extension of the existing
scan, not a general summary.

## Implementation Details

### New Types/Interfaces

- `PlaceState.sources: Vec<Provenance>` — the joined sources at the current
  program point, deduped and `storage_atoms`-flattened.
- `PlaceState::store_sources(&self) -> Vec<Provenance>` — the flattened atoms
  for the store judge.
- `PlaceState::join_sources(&mut self, source: Provenance)` and
  `PlaceState::strong_update_sources(&mut self, source: Provenance)`.

### New Functions/Methods

- `bind_value`: **strong-update** the place's source set from the assigned
  value (replace). Joins happen only in `FlowState::merge`; the assignment does
  not need to know its branch shape.
- `store_source_provenance`: a rebound local keeps returning `Local(id)` for the
  ordinary path; the source-set judgment is separate.
- `check_escaping_view_store`: before the whole-slot branch, consult the
  flattened set for a rebound local destination value; parameter-first, with
  the destination-root exclusion, the `take` deferral, and the type-parameter
  deferral described above.
- `callee_has_whole_out_slot_store`: compute the must-source set for each
  written local and include a non-`take`, non-destination parameter source.

### Integration Points

The ownership walk is the only writer of `PlaceState.sources`. The emission
stage never reads it. `FlowState::merge` joins the set with the rest of
`PlaceState`; `drop_bindings_from` and `edge_state` remove the whole place, so
the set is pruned with it. The instance audit is unaffected because Batch A is
concrete-first; a destination or value that still mentions a type parameter
keeps the existing whole-slot duty.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| Mixed static/foreign-parameter local stored into an out-slot | One `E10034` at the store, naming the first parameter root | AR-5 |
| Mixed static/local-root view stored into an out-slot | One `E10030` at the store, naming the first local root | AR-5 |
| Mixed parameter + local union | Parameter-first: one `E10034`, no second `E10030` | AR-5, AR-8 |
| The slot's own parameter | Destination-root exclusion (`owner == destination`); the store is accepted | AR-5 |
| `take` parameter root | Leave it to the existing ordinary path: an owned take value is accepted, a frame-dying take view reports `E10030` | AR-5 |
| A destination or value mentioning a type parameter | Do not apply the concrete set; leave the existing per-instance duty | AR-5, AR-12 |
| Definite overwrite before the store | Strong update removes the earlier source; the store is accepted | AR-5 |
| Definite conditional-RHS assignment | Strong update stores the value's `Composite`; an all-static conditional RHS is accepted | AR-5 |
| Conditional assignment before the store | The branch merge joins the arm states; flatten the `Composite` atoms; a parameter root is not hidden | AR-5 |
| All-static or own-value union | Accept and emit normally | AR-5 |
| Cross-frame caller duplicate | The must-source scan proves the callee's store and keeps one report; an unprovable store keeps the caller guard | AR-8 |

## Testing Requirements

- Same-frame mixed branch with a parameter root rejects with one `E10034`;
  the local-root twin rejects with one `E10030`; no translation unit.
- A mixed parameter + local union reports one `E10034` (parameter-first), not
  two diagnostics.
- A definite overwrite (`let s: str = p; s = "AA"; slot = s;`) is accepted.
- A conditional assignment (`s = c ? p : "AA"; slot = s;`) is rejected once
  with E10034; the `Composite` is flattened.
- A `take` parameter root is accepted; a frame-dying `take` view reports E10030.
- A rebound copy of the slot's own value is accepted (destination-root
  exclusion); a generic `T = str` view control is accepted.
- A definite conditional-RHS assignment whose arms are all static is accepted;
  the strong update does not retain the earlier parameter.
- Cross-frame mixed branch reports once, not twice; a later assignment that
  adds a foreign source must not suppress the earlier store's report. Add a
  branch/loop control where the callee store is not provable and the caller
  guard is kept.
- Static-only branch control is accepted, emits, runs, and is ASAN clean.
- Owned rebind control is accepted and releases once.
- The accepted controls re-emit byte-identically.
