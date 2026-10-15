# View Derivation and Owners: Conditional, Projection, and Update Residue

> **Document**: 03-02-view-derivation-and-owners.md
> **Parent**: [Index](00-index.md)

## Overview

Four in-scope rows in the view machinery: the owner walk through a view-returning call (the
call-wrapped conditional's every arm), the view-element array in argument and field positions,
the `str[N]` return spelling's false judgment, and the one-report mixed conditional return
(`DEF-279`). `DEF-268`'s reversed arm order is incidentally closed: the batch-H per-arm conversion
covers both orders, and ST-10 pins the reversed order as a green control (AR-25). The operative
rules are the single-view parity (a `str[N]` repeats the borrows of the single `str`; pinned by the
batch-H suites and `spec/16-borrow-rules.md:150,193`) and the derivation rule (a view return must
name the single view-like lend candidate — BR-7/EL-2).

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
source as a released borrow) and `return [h.v]` reports `E10034` + `E10036` where its single-view twin
(`return h.v`) reports one `E10036`.

Run the return spelling through the same view-element lowering as the binding: the return's array
literal evaluates its elements with the view-element guard, so the array reports exactly its
single-view twin's judgment (accepted where the twin is accepted; the same report where the twin
reports). The single-view parity is pinned by the batch-H suites and
`spec/16-borrow-rules.md:150,193`.

### 4. The reversed arm order (`DEF-268`, incidentally closed — AR-25)

`func pick(x: str, h: Box, c: bool): str { return c ? x : h.v; }` is fixed at HEAD: the batch-H
per-arm conditional-view conversion covers both arm orders, and the emitted C converts the owned
arm on its own path (`return (c ? x : cl_string_view(h.v));`), builds under the suite's strict
flags and under `-fsanitize=address`, and runs exit 0 leak-free. No rejection is added — the
suggested `E10036` would contradict the batch-H pin
(`conditional_projection_integrity_spec_test.rs:2313-2333`). ST-10 becomes the reversed-order
green control so the conversion cannot regress.

### 5. One report for the mixed local/generated return (`DEF-279`, AR-26)

`return (c ? a : mkBox()).v;` with a local `a: Box` reports both `E10035` (`cannot return a view of
local 'a'`) and `E10030` (`borrow of a compiler-generated owner does not live long enough`) at the
same return. The generated-owner report is the one the immutable oracle pins
(`callable_release_integrity_spec_test::should_reject_a_mixed_conditional_view_return` asserts
exactly one `E10030`), so the local-provenance judgment withholds the local-owner `E10035` at a
return whose mixed conditional has, on the other arm, a fresh owning expression whose lowered view
reads a compiler-generated root or a statement temporary — the case the liveness rule reports as
`E10030`. The escape report then carries the rejection, so one report per site and the pin both
hold.

The shapes where the local backing is the only escape keep exactly one `E10035`: `return (c ? b.v :
a)` (a local `b` and a parameter `a`), `return [k]`, and `bytes(h.name)`. A non-dying sibling (for
example a raw-pointer view arm) also keeps its `E10035`, so no acceptance hole opens. The
parameter variant `return (c ? a : mkBox()).v` with a parameter `a: Box` keeps its two judgments
(`E10036` + `E10030`) — different rules, left unchanged.

## Integration Points

- `own/place.rs`: `view_provenance` (:605), `collect_view_owners` (:826), `call_provenance` (:966).
- `own/call_check.rs`: `argument_owner` (:680).
- `own/statements.rs`: `check_return` (:1163) and its loan record — the local-provenance judgment
  that withholds `E10035` at the one-report site.
- `own/liveness.rs`: the generated-owner escape report (`E10030`) — the pinned report at that site.
- `own/lower.rs`: the view-element lowering.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A call-wrapped conditional argument | Every arm's owners are recorded; the loan covers the selected arm | AR-19 |
| A disagreeing conditional return (the reversed arm order) | Accepted; the per-arm conversion emits `(c ? x : cl_string_view(h.v))`; ST-10 pins the green control | AR-25 |
| A mixed local/generated return | Exactly one report: the generated-owner escape `E10030`; the local-owner `E10035` is withheld at that site | AR-26 |
| A local-only or local-plus-parameter backing at a return | Exactly one `E10035` (unchanged) | AR-26 |
| A view-element argument/field with a fresh element | The element's backing adopts a scoped owner (or the element retains its place) | AR-6-adjacent |
| The `str[N]` return whose twin is accepted | Accepted, judgment-identical to the twin | — |

## Testing Requirements

- Spec: the call-wrapped owner case (the parameter/parameter and two-arm spellings; the view used
  beside a writable alias; ASan), the view-element argument/field cases (ASan), the `str[N]` return
  cases (accepted and rejected twins), the chained view-returning callee twin, the reversed-order
  green control (ST-10), and the one-report mixed return (ST-11) with the local-only and
  local-plus-parameter controls; the oracle suites stay green.
- Impl: the owner-record shapes, the element-owner bindings, the report counts and spans (one
  report at the mixed return), and the re-emission determinism pins.
