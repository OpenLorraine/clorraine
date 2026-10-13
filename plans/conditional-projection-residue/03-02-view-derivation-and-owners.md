# View Derivation, Owners, and Escape Reports: Conditional, Projection, and Update Residue

> **Document**: 03-02-view-derivation-and-owners.md
> **Parent**: [Index](00-index.md)

## Overview

Five rows in the view machinery: the owner walk through a view-returning call (its conditional
argument records no owner), the view-element array in argument and field positions, the `str[N]`
return spelling's false judgment, the disagreeing conditional return, and the duplicate escape
report. The frozen rules are the single-view parity (a `str[N]` repeats the borrows of the single
`str`), the derivation rule (a view return must name the single view-like lend candidate —
BR-7/EL-2), and one report per site.

## Design

### 1. The call-wrapped conditional's owner (`DEF-261`)

`let s: u8[] = bytes((c ? b : mk()).v);` records no owner: the owner walk reaches the
view-returning call through its fallback, and the call's argument is a conditional whose arms
disagree (so `view_provenance` yields nothing to *this* walk's derivation check).

Extend `collect_view_owners`/`call_provenance`: a view-returning call contributes its arguments'
owners through the *same* conditional rule the borrow rules use (a local-backed arm keeps its
provenance; a fresh arm contributes none). The plain twins (`bytes(b.v)`, `let s: str = b.v`)
already record the owner; the call-wrapped spelling must match.

### 2. View-element positions (`DEF-266`)

`use([mk().v])` (an argument) and `Holder { names: [mk().v] }` (a field) leak 11 B per element.
The in-place element lowering and the field check cover a direct literal value only.

Extend the view-element treatment to the argument and field positions: each temporary element's
backing binds (adopts a scoped owner) or the element retains its place, exactly as the direct
binding spelling does; the direct binding and the single-view twins stay byte-identical.

### 3. The `str[N]` return spelling (`DEF-267`)

`func f(s: String): str[1] { return [s]; }` reports a false `E10034` (the element consume marks the
source as a released borrow) and `return [h.v]` gains an extra `E10036` where the single-view twin
(`return s`) is accepted.

Run the return spelling through the same view-element lowering as the binding: the return's array
literal evaluates its elements with the view-element guard, so the array reports exactly its
single-view twin's judgment (accepted where the twin is accepted; the same report where the twin
reports).

### 4. The disagreeing conditional return (`DEF-268`, AR-7)

`func pick(x: str, h: Box, c: bool): str { return c ? x : h.v; }` is accepted because
`view_provenance` yields none for a conditional whose arms disagree, and the derivation judgment
flags only a named non-candidate parameter.

A conditional return whose arms do not agree on the single view-like lend candidate reports
`E10036` at the declaration, exactly as the named non-candidate spelling does. The batch-H
phase-5 rule already reports `E10035` when an arm is local-backed; this judgment covers the
parameter-vs-parameter disagreement. (The batch-H widening already makes the mixed spelling build
strictly; this judgment is the derivation rule's.)

### 5. One escape report per site (`DEF-279`, AR-8)

`return (c ? a : mkBox()).v;` now reports `E10035` (the local escape) and `E10030` (the
generated-owner escape) at the same span. Both are true; the repo's convention is one report per
site.

When the return already reports the local escape for the same value, the generated-owner loan that
would report `E10030` is not recorded (the rejected return's elision/loan bookkeeping is moot, and
the local report names the cause). The oracle spec test
(`callable_release_integrity_spec_test::should_reject_a_mixed_conditional_view_return`) keeps
passing: it pins the `E10030` presence for its own shape, which has no local arm.

## Integration Points

- `own/place.rs`: `view_provenance` (:605), `collect_view_owners` (:826), `call_provenance` (:966).
- `own/call_check.rs`: `argument_owner` (:676).
- `own/statements.rs`: `check_return` (:1141) and its loan record.
- `own/liveness.rs`: the `E10030` report (:350).
- `own/lower.rs`: the view-element lowering.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A disagreeing conditional return | `E10036` at the declaration (one report) | AR-7 |
| A local-backed arm at a return | `E10035` (existing), without the duplicate `E10030` | AR-8 |
| A view-element argument/field with a fresh element | The element's backing adopts a scoped owner (or the element retains its place) | AR-6-adjacent |
| The `str[N]` return whose twin is accepted | Accepted, byte-identical judgment to the twin | — |

## Testing Requirements

- Spec: the call-wrapped owner case (mutate after, ASan), the view-element argument/field cases
  (ASan), the `str[N]` return cases (accepted and rejected twins), the disagreeing conditional
  return (`E10036` once), and the single-report case; the oracle suite stays green.
- Impl: the owner-record shapes, the element-owner bindings, the report counts and spans, and the
  byte-identity controls.
