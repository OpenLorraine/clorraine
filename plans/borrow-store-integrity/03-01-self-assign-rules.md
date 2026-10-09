# Self-Assign Rules: Borrow-Store Integrity

> **Document**: 03-01-self-assign-rules.md
> **Parent**: [Index](00-index.md)

## Overview

This component owns the whole-binding self-assign judgment on borrowed roots: DEF-100 extends it
to parameter-typed lend roots through the per-instance duty pattern, and DEF-102 documents and pins
the conservative comparison the out-slot element rule already applies. Both judgments live in
`own/expr.rs::check_lend_target_store` and are audited (where deferred) by
`own/instance_check.rs`.

## Architecture

### Current Architecture

`check_lend_target_store` judges a store whose target is a lend parameter or out-parameter slot:

1. `OutSlot` roots (nullable `borrow` parameters): an exact projected self-assign
   (`source_projections == projections`) reports `E10020`; every other store is accepted.
2. `Param ∧ Mode::Borrow` roots with no projections: the site defers to `lend_moves_judged` — the
   existing lend-move path reports a read-only lend source — and otherwise reports `E10034` for an
   owning type or records a storable-slot duty while the type is a parameter.
3. Any projected store on a non-out-slot root keeps its existing rules.

The `OutSlot` branch's comparison treats two run-time index projections
(`Projection::Element(None)`) as equal, so `xs[i] = xs[i]` *and* `xs[i] = xs[j]` are both rejected;
literal indices compare by value (`Element(Some(n))`).

### Proposed Changes

**DEF-100.** In the `Param ∧ Mode::Borrow` whole-binding branch, before the `lend_moves_judged`
deferral, detect the exact self-assign (`place_target(value)` is the same binding with equal
projections) when the root's declared type still mentions a type parameter (`maybe_owns`). Record
one duty and return:

- new record `ir::RequiresSelfAssign { function, parameter, ty, span }` (re-exported through
  `ir.rs`), pushed with the same recording-time dedupe as the other duties
  (`recorded_not_lend`-style keyed insert);
- the reachable concrete spellings keep their current paths: `borrow String` still reports
  `E10034` from the `owns_drop` tail, `borrow String?` still reports `E10020` from the `OutSlot`
  branch, and duplicable roots stay accepted.

**Mechanism choice (AR-2).** A new record rather than a flag on `RequiresStorableSlot`: the
storable-slot judgment rejects *any* owning store into the slot, while the self-assign judgment
fires only on the exact self-assign and maps nullable and owning substitutions to two different
diagnostics (AR-7). Folding both into one record would conflate two conditions and force the audit
to re-test which one was recorded. The new record mirrors the existing four `Requires*` records.

**DEF-102.** No semantic change (AR-4). The plan:
- pins the rejected pairs (`xs[i] = xs[i]`, `xs[0] = xs[0]`, `xs[i] = xs[j]`) and the accepted pair
  (`xs[1] = xs[0]`, plus the mixed constant/runtime pairs) with specification tests;
- states the conservative comparison explicitly in the `check_lend_target_store` documentation
  ("two run-time index projections compare equal, so any same-root run-time pair is judged a
  self-assign") so the behavior is recorded where it is implemented.

## Implementation Details

### New Types/Interfaces

```rust
/// A recorded whole-binding self-assign site on a lend root whose type
/// already mentions a type parameter.
pub struct RequiresSelfAssign {
    /// The generic function owning the site.
    pub function: String,
    /// The lend parameter's source name (for the diagnostic).
    pub parameter: String,
    /// The declared (parameter-typed) root type.
    pub ty: Ty,
    /// The store site.
    pub span: Span,
}
```

### New Functions/Methods

- `FunctionChecker::record_self_assign(...)` — records the duty once per site (keyed insert on
  the IR name and span, mirroring `recorded_not_lend`).
- `check_self_assign_requirements(program, sema, mono, diagnostics)` in `own/instance_check.rs` —
  for each recorded duty, walks the recorded instances of the function, substitutes the type
  arguments, and:
  - skips a duplicable, view, or unknown substitution;
  - reports `E10020` ("'{parameter}' was moved and cannot be used", the local moved-value message)
    for a nullable substitution — the concrete `borrow T?` out-slot rule;
  - reports `E10034` ("cannot release or store borrowed parameter '{parameter}'") otherwise;
  - breaks after the first offending instance, so one site reports once.

### Integration Points

- The duty vector lives beside the existing ones in `own.rs` and is drained by the audit chain in
  `own.rs::check()` (the same place `check_not_lend_requirements` and
  `check_storable_slot_requirements` run).
- The exact self-assign test reuses `place_target` equality, the same shape the `OutSlot` branch
  uses; no new projection comparison is introduced (AR-4).

## Code Examples

### Example 1: DEF-100 (fixed)

```clorraine
func copySelf<T>(borrow slot: T): void {
    slot = slot;                  // instance String  -> E10034
}                                 // instance String? -> E10020
                                  // instance i32     -> accepted
```

### Example 2: DEF-102 (pinned, unchanged behavior)

```clorraine
func bump(borrow xs: i32[]?): i32 {
    if (xs != null) {
        xs[i] = xs[i];            // rejected E10020 (run-time pair, conservative)
        xs[1] = xs[0];            // accepted (distinct literal indices)
    }
    return 0;
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Owning non-nullable substitution | `E10034` at the store site, once per site | AR-2, AR-7 |
| Nullable substitution | `E10020` (the out-slot self-assign message) | AR-7, AR-4 |
| Duplicable / view / unknown substitution | Accepted (the concrete equivalents are accepted) | AR-7 |
| Run-time index pair on an out-slot element | `E10020`, conservative and documented | AR-4 |

> **Traceability:** every error-handling strategy and design choice back-references the register
> entry that resolved it. Only universally obvious facts and zero-semantic-impact formatting are
> exempt.

## Testing Requirements

- Specification cases ST-1 – ST-7 ([07-testing-strategy.md](07-testing-strategy.md)).
- Implementation pins: the emitted/reported shape per instance (one report per site), the duty
  naming, and byte-identical re-emission.
- Memory probes: the accepted duplicable and distinct-constant shapes are ASan-clean.
