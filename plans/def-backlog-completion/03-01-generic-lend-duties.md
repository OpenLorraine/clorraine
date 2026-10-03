# Generic Lend Duties: Def Backlog Completion

> **Document**: 03-01-generic-lend-duties.md
> **Parent**: [Index](00-index.md)

## Overview

Close DEF-56 and DEF-61 by rejecting the generic spelling of a lend escaping as owned, with the same
diagnostic the concrete spelling already emits. The fix introduces one post-substitution duty,
mirroring the existing duplicability duty, and records it at the single central maybe-owning move
site. This document owns the behavior; the reproduction evidence is in
[02-current-state.md](02-current-state.md) §Reproductions, and the ruling is AR-2.

## Architecture

### Current Architecture

While a value's type mentions a type parameter, ownership checks take the conservative branch:
`mark_moved_if_owning` (`compiler/src/own/place.rs:295-301`) marks the place instead of moving it,
and a later conflicting read records a `RequiresDuplicable` duty (`own/place.rs:307-322`). The only
`E10034` reporting site is `move_place` (`own/place.rs:204-215`), reached through `move_target`
(`own/place.rs:325-337`). Because all checks run before substitution and no duty covers a lend that
is consumed and never read again, the generic spelling is accepted; after substitution it moves a
lend and both owners release the same storage.

### Proposed Changes

1. **One central recording site.** `mark_moved_if_owning` gains the move site's span and, when the
   target's root is a lend `Param` (binding kind `Param`, mode `None`, `Const`, or `Borrow`; a
   `borrow T?` out-param slot stays exempt), records a `RequiresNotLend` duty. The parameter name
   and the moved type are derived inside the checker from the binding and
   `projected_type` (`own.rs:815-824`), so the eleven existing maybe-owning call sites need only
   the span added — the owned return branch (`own/statements.rs:841-848`), the take-argument branch
   (`own/call_check.rs:281-289`), `consume_owned_place` (`own/expr.rs:48-62`), and the binding
   effect (`own/statements.rs:461-481`), plus the conditional-arm sites. The audit behind this
   change confirmed every `move_target` site has a maybe-owning `mark_moved_if_owning` partner.
2. **One duty record and one audit.** `RequiresNotLend` lives beside `RequiresDuplicable` in
   `compiler/src/ir/mono.rs` and flows through the same collection and per-instance walk. The audit
   substitutes the instance's type arguments and reports the existing `E10034` when the substituted
   type needs a drop; duplicable substitutions are accepted. The audit does **not** inherit the
   duplicability duty's `mentions_type_param` early skip (`own/instance_check.rs:68-70`): a lend
   whose declared type is concrete but sits in a maybe-owning position is judged the same way
   (PF-021). Duties dedup on `(function, ir_name, span.file, span.start)` — `Span` is not `Ord`,
   so the key uses its file/offset fields (PF-014).
3. **No new diagnostic.** The message text and help line are the ones `move_place` already uses
   (`compiler/src/own/place.rs:208-213`; registry `spec/14-diagnostics.md:62`).

## Implementation Details

### New Types/Interfaces

```rust
/// A generic body's obligation that a moved lend may only instantiate to a
/// non-owning type.
///
/// The concrete spelling reports `E10034` directly; while a type is still a
/// parameter the move takes the maybe-owning branch, so the body records this
/// duty and the per-instance audit reports the same diagnostic once the type
/// argument is known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiresNotLend {
    /// The generic function whose body carries the duty.
    pub function: String,
    /// The lend parameter's source name, for the diagnostic text.
    pub parameter: String,
    /// The moved value's declared type, substituted per instance by the audit.
    pub ty: Ty,
    /// The move site, for the diagnostic span.
    pub span: Span,
}
```

`Monomorphization` (or the owning IR aggregate) carries `not_lend_requirements: Vec<RequiresNotLend>`
with a `require_not_lend` method, exactly parallel to `duplicable_requirements`
(`compiler/src/ir/mono.rs:150,208-211`). The checker's duty collection (`compiler/src/own.rs:123-128`)
forwards the record for the function being lowered.

### Integration Points

| Site | Change |
| ---- | ------ |
| `own/place.rs` | `mark_moved_if_owning` takes the move span; records the duty for a lend `Param` root (kind plus lend mode, the `borrow T?` slot exempt), deriving the parameter and moved type internally, at most once per `(function, ir_name, span.file, span.start)` |
| `own/statements.rs`, `own/call_check.rs`, `own/expr.rs` | Add the move span at the existing maybe-owning call sites; no control-flow change |
| `own.rs` | Collect the recorded duty into the monomorphization aggregate, mirroring the duplicability duty |
| `own/instance_check.rs` | `check_not_lend_requirements` substitutes per instance and reports `E10034`; reports the first owning instantiation per duty so a site reports once regardless of how many owning instances exist |

### Behavior Rules

- A whole-place move of a lend parameter records the duty with the parameter's declared type.
- A projected move of a lend — field **or element** (`p.field`, `p[0]`) — records the projection's
  type, resolved from the declared type; a partial move of a lend is judged by the moved
  projection's type (PF-026).
- A `take` parameter, a local, a global, and a projection over non-parameter storage record no
  duty; the predicate is the binding kind plus the lend mode, mirroring `move_place`
  (`own/place.rs:204-215`).
- The `borrow T?` out-param slot stays exempt from `E10034`, as the specification states
  (`spec/06-functions.md:141-145`); the duty does not record for it (PF-027).
- A duty for which the substituted type is duplicable is accepted silently (ST-3).
- The audit reports at most one `E10034` per recorded duty, mirroring the duplicability audit's
  break (`own/instance_check.rs:93-95`), so forwarding chains and multiple owning instantiations do
  not multiply the report (ST-2, AR-2).
- A lend moved and then read again keeps its existing duplicability duty as well, so the owning
  instantiation reports `E10034` and `E10020`; R1's wording is relaxed to "reports the existing
  `E10034`" and ST-26 pins the dual report (PF-024). The code change does not suppress either
  duty.
- The duty records at the existing maybe-owning call sites reached while lowering ordinary function
  bodies. This plan does not extend move checking into lifted lambda expression bodies (a
  pre-existing boundary, outside the seven defects); the migrated test programs in task 1.2.5 are
  the empirical inventory for that scope (PF-002, PA-701 scope A).

## Code Examples

```clorraine
// Accepted before the fix; reports E10034 after it, like the concrete spelling.
func apply<T>(x: T): T { return x; }
func outer<T>(x: T): T { return apply<T>(x); }

// Still accepted: i32 is duplicable, so the lend is copied.
let n: i32 = outer<i32>(41);
```

```clorraine
// DEF-61: the take position in a generic body reports E10034 for an owning
// instantiation, matching the concrete `Holder`/`String` spelling.
func f<T>(take h: Holder<T>, v: T) { h.items.push(v); }
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Owning instantiation of a moved lend | Report the existing `E10034` at the recorded move site with the existing message and help | AR-2 |
| Duplicable instantiation | Accept; the emitted move stays a plain copy | AR-2 |
| Multiple owning instantiations of one duty | Report once (first owning instance) so the same body site is not repeated | AR-2 |
| Concrete-typed lend in a maybe-owning position | Judged by the substituted declared type; no param-mention early skip | PF-021 |
| Lend element projection (`p[0]`) | Recorded like a field projection and judged by the element type | PF-026 |
| `borrow T?` out-param slot | Exempt: the duty does not record | PF-027 |
| `take` parameter or non-parameter storage | No duty; behavior unchanged | AR-9 |
| A typo-class body that never instantiates | No report (no instance to judge), matching the duplicability duty | AR-2 |

> **Traceability:** Every error-handling strategy references the Ambiguity Register entry (AR #) that
> resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification cases in [07-testing-strategy.md](07-testing-strategy.md): ST-1 – ST-6 (owning
  rejection direct/forwarded/take, duplicable acceptance, concrete-spelling regression,
  `take`-parameter regression), ST-21 (stored/binding form), ST-22 (concrete-typed lend in a
  maybe-owning position), ST-23 (lend element projection), ST-24 (`borrow T?` exemption), and
  ST-26 (dual report).
- Implementation tests: the duty record shape, one-report dedupe per site, no duty for non-lend
  roots, the `BindingKind::Param` predicate, and byte-identical re-emission for accepted
  duplicable instances.
- Full verification: `make check`.
