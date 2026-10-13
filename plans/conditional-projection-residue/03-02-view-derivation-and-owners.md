# View Derivation and Owners: Conditional, Projection, and Update Residue

> **Document**: 03-02-view-derivation-and-owners.md
> **Parent**: [Index](00-index.md)

## Overview

Three in-scope rows in the view machinery: the owner walk through a view-returning call (the
call-wrapped conditional's every arm), the view-element array in argument and field positions,
and the `str[N]` return spelling's false judgment. Two further rows (`DEF-268`, `DEF-279`) are
parked for rulings and stay at current behavior (AR-2). The operative rules are the single-view
parity (a `str[N]` repeats the borrows of the single `str`; pinned by the batch-H suites and
`spec/16-borrow-rules.md:150,193`) and the derivation rule (a view return must name the single
view-like lend candidate — BR-7/EL-2).

## Design

### 1. The call-wrapped conditional's owner (`DEF-261`, AR-19)

`let s: u8[] = bytes((c ? b : d).v);` (both arms are borrow parameters) and the two-arm spelling
`bytes(c ? b.v : d.v)` are accepted but record no owner for the call-wrapped conditional: the
owner walk reaches the view-returning call through its fallback and short-circuits at the
conditional, so the parameter-backed arms contribute no owner (the local-backed arm is the only
one the current provenance rule preserves). The view then carries no loan on the selected arm's
owner, so a later use of the view beside a writable alias reads freed storage (heap-use-after-free
at base and head). The local-arm spelling (`bytes((c ? b : mk()).v)`) already reports its judgment
at the head (`E10031` at the mutation) and is a control, not a red.

Extend `collect_view_owners`/`call_provenance`: the call path recurses through the view-returning
call's argument (the Conditional included) and collects *every* arm's owners — exactly the rule
the direct walk applies (a local-backed arm keeps its provenance; a fresh arm contributes none).
The plain twins (`bytes(b.v)`, `let s: str = b.v`) already record the owner; the call-wrapped and
two-arm spellings must match.

### 2. View-element positions (`DEF-266`)

`use([mk().v])` (an argument) and `Holder { names: [mk().v] }` (a field) leak 11 B per element.
The in-place element lowering and the field check cover a direct literal value only.

Extend the view-element treatment to the argument and field positions: each temporary element's
backing binds (adopts a scoped owner) or the element retains its place, exactly as the direct
binding spelling does; the direct binding and the single-view twins keep their pinned shapes.

### 3. The `str[N]` return spelling (`DEF-267`)

`func f(s: String): str[1] { return [s]; }` reports a false `E10034` (the element consume marks the
source as a released borrow) and `return [h.v]` gains an extra `E10036` where the single-view twin
(`return s`) is accepted.

Run the return spelling through the same view-element lowering as the binding: the return's array
literal evaluates its elements with the view-element guard, so the array reports exactly its
single-view twin's judgment (accepted where the twin is accepted; the same report where the twin
reports). The single-view parity is pinned by the batch-H suites and
`spec/16-borrow-rules.md:150,193`.

### Parked rows (`DEF-268`, `DEF-279`, AR-2)

`DEF-268`'s disagreeing conditional return and `DEF-279`'s duplicate escape report stay at current
behavior: each fix would require re-pinning an immutable batch-H suite pin through a user ruling
(`conditional_projection_integrity_spec_test.rs:2313-2333` pins the conditional twin accepted;
`callable_release_integrity_spec_test.rs:1772-1802` pins exactly one `E10030` for a shape with a
local arm). The rows' roadmap entries return to Backlog with the ruling notes (AR-2).

## Integration Points

- `own/place.rs`: `view_provenance` (:605), `collect_view_owners` (:826), `call_provenance` (:966).
- `own/call_check.rs`: `argument_owner` (:680).
- `own/statements.rs`: `check_return` (:1163) and its loan record.
- `own/lower.rs`: the view-element lowering.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A call-wrapped conditional argument | Every arm's owners are recorded; the loan covers the selected arm | AR-19 |
| A disagreeing conditional return | Parked: current behavior unchanged pending a ruling | AR-2, AR-7 |
| A local-backed arm at a return | Parked: current behavior unchanged pending a ruling | AR-2, AR-8 |
| A view-element argument/field with a fresh element | The element's backing adopts a scoped owner (or the element retains its place) | AR-6-adjacent |
| The `str[N]` return whose twin is accepted | Accepted, judgment-identical to the twin | — |

## Testing Requirements

- Spec: the call-wrapped owner case (the parameter/parameter and two-arm spellings; the view used
  beside a writable alias; ASan), the view-element argument/field cases (ASan), the `str[N]` return
  cases (accepted and rejected twins), and the chained view-returning callee twin; the parked rows'
  shapes are not authored; the oracle suites stay green.
- Impl: the owner-record shapes, the element-owner bindings, the report counts and spans, and the
  re-emission determinism pins.
