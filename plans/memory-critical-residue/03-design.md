# Design

Each fix extends an existing route. No new diagnostic, registry entry, or `spec/` chapter.

## Phase 1 — Carrier sources (`DEF-508`, `DEF-509`, `DEF-512`, `DEF-519`)

### `DEF-508` — a projected member of a carrier call

`frame_owned_view_owner` judges a call only when the call is the whole expression. A member or
element chain rooted at a call reads the same carrier positions, so the walk descends a
`Member`/`Index` base to an inner call. The call's carrier sources (below) decide the backing.
The copy collector follows the same chain: a projection rooted at a carrier call contributes
each source's positions to the destination.

### `DEF-509` / `DEF-512` — several sources, `take`, and owned aggregates

Replace the single-source helper with `call_carrier_sources`, which returns every argument the
returned carrier may read through: each view-like lend parameter, each owned aggregate whose
storage a view can derive from, and each `take` argument. A `take` value transfers ownership to
the callee, so a returned carrier that derives from it hands the same storage back to the
caller; the caller records that source's positions exactly as it does a lend source. Every
consumer iterates the sources and reports the first frame-dying backing, so one call site still
reports once.

The identity return `id(take h): T { return h; }` stops treating the returned take parameter's
own binding as a frame-dying owner at the return. A view stored into that parameter in the
callee is already rejected at the store; a view it inherited from the caller survives the
return. The caller-side store still judges the argument's positions, so the filed escape
reports at the store.

### `DEF-519` — a carrier call in a returned struct literal

`frame_owned_view_owner` descends a struct or array literal through its view-carrying positions
(the declared field or element type decides), exactly as `carried_view_owners` does for the
lowered return. A field whose value is a conditional of a carrier call therefore names the
carrier's frame-backed source at the return.

## Phase 2 — Callable expressions (`DEF-510`, `DEF-511`)

### `DEF-510` — inline lambda and conditional func values

`callable_call_signature` resolves a lambda literal from its recorded closure/function type and
a conditional from its recorded callable type. Both source walks then resolve the call's single
lend source, so an escaping view reports at its destination.

`check_call_expr` gains the value-callee branch it already has for a func-typed binding: a
non-name callee whose recorded type is callable is evaluated as a value and its arguments are
checked against that signature.

`lower_call` builds the callable value for a non-name callee and attaches it as the call's
receiver, so the emitter renders `(callable)(args)` instead of the raw conditional expression.
A closure-typed callee keeps the existing closure-call path.

### `DEF-511` — a by-value capture's view

A by-value capture lives in the closure environment, which dies with the closure value. The
frame-dying predicate treats such a capture like a local: the capture-backed out-slot store
reports the owning escape. A by-pointer (borrow) capture keeps the caller's storage and stays
accepted.

## Phase 3 — Generic conditional wrappers (`DEF-514`, `DEF-515`, `DEF-518`, `DEF-521`)

### `DEF-514` / `DEF-515` — the conditional take move-out

The take lowering materializes an unresolved conditional into a carrier only when every arm
owns its place. A `null` or literal arm owns nothing and blocks that route, so the emitted call
passes the conditional without emptying the selected member. The predicate changes to accept a
conditional with at least one owning place arm: an arm that owns nothing fills the carrier as a
plain store, and an arm that owns is emptied in its own branch. `unresolved_conditional_place_arms`
unwraps `unsafe`/casts first, so the wrapped spelling records the same duties.

### `DEF-518` — an owned array-literal element

The array-literal element path records the type-parameter conditional duty for each element and
fill value, exactly as the struct-literal field path already does. At an owning instance the
audit reports the lend move (`E10034`); a duplicable instance stays accepted.

### `DEF-521` — a wrapped generic source

`type_param_conditional_sources` unwraps `unsafe` and casts before looking for the conditional,
so every consuming position (take, binding, field, assignment, return) records the same duty as
the bare twin.

## Phase 4 — Moved state (`DEF-523`)

`write_place` compares moved paths against the written path instead of the first projection. A
nested write below a partially moved field reports the moved-value error once and keeps the
moved prefix. A whole-field write keeps its re-initialization behavior. The maybe-moved duty set
and the null narrowings use the same prefix rule: a strict prefix survives a nested write, the
written path and its sub-paths are cleared, and unrelated paths stay.

## Test strategy

Each row gets a red specification test that must fail at `6a67880`, an accepted control, and an
implementation pin in the paired suite. Accepted controls are emitted twice for byte identity
where the suite already pins it. The run cases use the suites' existing sanitizer helper.
