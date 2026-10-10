# Owner Release Completeness: Callable Release Integrity

> **Document**: 03-03-owner-release-completeness.md
> **Parent**: [Index](00-index.md)

## Overview

Five defects are holes in the owner/release machinery: a temporary receiver's siblings leak
(DEF-139), defer bodies suppress materialization (DEF-158), mixed conditional view arms leak
(DEF-159), a fixed array of view elements returned from a temporary leaks (DEF-163), and a
discarded owned call statement leaks its result (DEF-164). The fixes extend the existing owner
binder, guarded materialization, cleanup edges, and the view-return postlude; exactly one release
per created owner is the invariant every fix must preserve (R2, R8, R9, R10, R11; AR-7, PD-4).

## Architecture

### Current Architecture

- `bind_ownerless_projection_root` (`own/lower.rs:1411`) roots call results and struct literals
  for member projections; the func-field call path extracts its receiver
  (`own/call.rs:183`) without routing the receiver aggregate through that binder — a temporary
  receiver's siblings never bind (DEF-139).
- `check_defer` (`own/statements.rs:835-880`) lowers the body with
  `materialize_owned_temporaries = false` (`:846-847`), so view backings and func-value
  temporaries inside `defer` keep the value unchanged and leak (DEF-158). The block's own locals
  are kept and their cleanup is the block's own (`:857-873`), which is the shape the fix needs.
- `bind_view_projection_root`'s conditional arm (`own/lower.rs:1533-1545`) returns a
  place-borrowing conditional unchanged (`conditional_borrows_place`), so a mixed place/call
  conditional keeps the call arm inline and its backing leaks; the both-call spelling is already
  owned. The guarded sibling (`bind_guarded_projection_root`, `:1562`) already materializes
  conditionally evaluated owning operands under a registered release (DEF-159).
- The view-return elision prelude/postlude (`own/place.rs:1099-1128`) and `is_elided_view`
  (`borrow.rs:66`) treat only single view types as elided views, so a fixed array of view
  elements skips both the candidate count and the escape checks (DEF-163).
- Statement temporaries release through pending-release copies on edges
  (`own/statements.rs:895`); a bare expression-statement call's owned result is never registered
  (DEF-164).

### Proposed Changes

1. **Temporary receiver roots (DEF-139).** The func-field call lowering routes its receiver
   aggregate through the existing ownerless-root binder, so a temporary receiver with owning
   fields binds to a body-scope owner and its cleanup releases the siblings. A receiver that is
   a named place stays untouched.
2. **Defer-body materialization (DEF-158).** `check_defer` lowers the body with materialization
   enabled; owners declare in the defer block's scope, whose cleanup is already the block's own,
   and the block's locals already stay in the function's local list for the emitter. The region
   range recorded for the bounded copy already covers the body's blocks.
3. **Mixed conditional view arms (DEF-159).** When a conditional view projection has at least one
   owning arm, each owning arm's root is bound at its evaluation site through the guarded
   materialize machinery; place-rooted arms stay borrows; the owner's release is registered with
   the view binding's scope, so the backing outlives the view's last use and releases exactly
   once on the taken path. A both-place conditional (a borrow) and the both-call spelling are
   unchanged.
4. **View-element arrays (DEF-163).** The elided-view and escape predicates cover a fixed array
   whose element is a view, so the function-level checks run for the array spelling and report
   the same diagnostics the single-view spelling reports for the same derivation; an array
   derived from exactly one view-like lend parameter stays accepted.
5. **Discarded owned calls (DEF-164).** An expression statement whose call result is owned and
   unused binds the result to a statement temporary that releases at the statement's end,
   reusing the existing statement-temporary release machinery rather than a new cleanup path.

## Implementation Details

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `own/lower.rs` | receiver rooting; conditional-arm materialization; the guarded-materialize sibling |
| `own/statements.rs` | defer materialization; the expression-statement path and pending releases |
| `own/place.rs`, `borrow.rs` | the elided-view/escape predicates for array elements |
| `codegen/cleanup.rs` | the registered releases (unchanged shapes; new owners flow through) |
| `own/instance_check.rs` | duty audits over the new owner shapes |

### Invariants

- Every created owner releases exactly once on every path that created it; a path that created
  none releases none.
- A view's backing outlives the view binding's last use and is released no later than the
  binding's scope end.
- No evaluation-order change: an arm or deferred body runs exactly as before; only cleanup is
  added.
- Accepted programs outside the defect shapes keep byte-identical emission where pinned.

### Code Examples

```clorraine
H { call: addOne, s: stringOf("x") }.call(5);   // DEF-139: siblings released
defer { let v: str = (Box { v: mk() }).v; }     // DEF-158: backing owned by the defer block
let v: str = (c ? b : makeBox()).v;             // DEF-159: the taken call arm is owned
func make(): str[1] { return [tmp.v]; }         // DEF-163: reports as the single view does
make();                                         // DEF-164: the result is released
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Temporary receiver with owning siblings | Accepted; body-scope owner releases them | AR-7 |
| Defer-body owned temporary | Accepted; block-scope owner releases once per execution | AR-7 |
| Conditional with a place arm | Place arm stays a borrow; owning arms own | AR-7 |
| Array of view elements from a temporary | Reports the single-view spelling's diagnostics | AR-8 |
| Discarded owned call result | Accepted; released at the statement's end | AR-7 |
| Any release on an unfilled owner | Forbidden — the guarded forms only register filled owners | AR-7 |

## Testing Requirements

- Specification cases: ST-8 – ST-12 (`07-testing-strategy.md`), each with an ASan run and the
  filed reproductions; ST-11 asserts diagnostic parity between the array and single-view
  spellings.
- Implementation pins: the owner binding shape per hole; release counts per execution; the
  registered-release placement; byte-identity for the unaffected corpus.
- Review lenses: correctness + memory-safety + emission-integrity + semantics (view lifetimes).
