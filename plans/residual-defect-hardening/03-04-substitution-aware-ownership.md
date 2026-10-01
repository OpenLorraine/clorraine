# Substitution-Aware Ownership: Residual Defect Hardening

> **Document**: 03-04-substitution-aware-ownership.md
> **Parent**: [Index](00-index.md)

## Overview

This component fixes DEF-49: ownership decisions are made once, before monomorphization, so a
generic body whose type parameter owns under a recorded instantiation emits no release, copies
where it must move, and skips the actions the lowerer deferred for parametrized types.

Per AR #17 the fix deposits **symbolic ownership duties** at the checker's decision points and
discharges them per instance, where the substitution is available. No per-instance body is built,
no flow analysis is reconstructed, and the checker keeps its single pass. The discharge predicates
already exist in codegen: `emit_drop` substitutes the place type
(`compiler/src/codegen/drop.rs:23`), `emit_local_declarations` substitutes local and temporary
types (`compiler/src/codegen/function.rs:237`, `:252`), and `Statement::ReleaseTemp` routes
through `emit_drop` (`:518-526`). `CopyObligation` (`compiler/src/ir/mono.rs:85-95`) records
substitution-dependent copy duties but has no discharge consumer yet; the per-instance audit this
design extends is `own/instance_check.rs`'s fill/view pass.

## Architecture

### Current Architecture

- `owns_drop` (`compiler/src/own.rs:720-726`) and `duplicable`
  (`compiler/src/types/table.rs:377-409`) treat a type parameter as non-owning and duplicable, so
  `declare_local`/`declare_param` set `needs_drop = false`, `run_scope_cleanup` emits no drop
  (`own.rs:435-443`), and `register_pending_release` registers no release (`own.rs:965-972`).
- Store decisions are equally blind: `prepare_store` (`own/statements.rs:272-292`),
  `consume_place` (`own/lower.rs:701-720`), and the flow recorders
  (`own/expr.rs:50`, `:85`, `:108`) skip a param-mentioning type.
- Fourteen sites are skipped or substitution-blind for param-mentioning types (the site table
  below), including every materialization helper.
- Codegen substitutes types at emission but has no duty to discharge, because the checker put none
  in the IR.

### Proposed Changes

Five duty kinds, deposited by the checker and discharged per instance:

1. **Maybe-owning registration.** A binding or temporary whose declared type mentions a type
   parameter and is not a view, unknown, or void is registered as *maybe-owning*: the checker
   emits its `Statement::Drop` and registers its pending releases exactly as for a known owner.
   Discharge: `emit_drop` substitutes the type, so a duplicable or scalar instance emits a no-op
   and an owning instance releases. Drops are null-safe and idempotent, so a moved-from binding's
   drop is a no-op. A `maybe_owns(ty)` helper centralizes the predicate;
   `declare_local`/`declare_param`/`register_pending_release` use it. **Parameter mode is
   preserved:** `declare_param` keeps the existing `mode == Mode::Take` gate
   (`compiler/src/own.rs:630`) around `owns_drop(ty) || maybe_owns(ty)`, so a `borrow` parameter
   or an out-slot is never maybe-owning and the callee never drops a lent value.

2. **Move-if-owning stores.** The checker stops skipping a param-typed value that is consumed from
   a place: `prepare_store` and `consume_place` emit the existing `Statement::Move` shape (copy
   plus source zeroing) for a param-mentioning source. Discharge: the move emission
   (`codegen/function.rs:369-408`) checks the substituted source type; when it owns, it emits the
   move as today; when it does not, it emits a plain copy and leaves the source intact, which is
   exactly the behavior the duplicable flow expects. The same rule covers a param-typed `take`
   argument place: `check_argument_place` (`own/call_check.rs:281-284`) records the
   move-if-owning marker instead of skipping, and `take_move_out_places`
   (`own/call.rs:346-368`, skip at `:352`) empties the caller's place through the same non-owning
   codegen branch, so the callee's maybe-owning drop cannot double-free the caller's value.

3. **Requires-duplicable conflicts.** A move-if-owning store is honest only if the body does not
   use the source afterwards. The checker keeps its copy flow for param types (so duplicable
   instances stay accepted) and records a `RequiresDuplicable { function, binding, span }` duty
   the first time such a use follows a store on a flow path. The detection hook is the name-read
   funnel `read_place` (`own/place.rs`), which every `Name` use reaches before its consumption
   site (`own/expr.rs:208-212`; `own/statements.rs:213`), so a second consuming move, a borrow, or
   a `take` argument is detected as well. The marker lives per binding in `PlaceState`
   (`own/flow.rs:93-106`), is merged by the existing conservative join (`own.rs:834-865`), and is
   cleared by a whole-binding write (`own/place.rs`'s `write_place`). The per-instance audit
   (`own/instance_check.rs`) looks up each instance of the function, substitutes the binding's
   declared type, and reports `E10020` (`spec/14-diagnostics.md:51`) once per generic function and
   binding, at the body's use site when the substituted type is not duplicable. No later use means
   no duty, and the store simply moves for every instance. Recording the duty lives next to the
   existing `CopyObligation` in `ir/mono.rs`; the audit reuses `substitute_instance` the way the
   fill and view checks do.

4. **Materializations.** The five `mentions_type_param` skips in the materialization helpers are
   removed: a param-typed operand materializes like any other, its temporary is declared with the
   substituted type per instance, and its maybe-owning release is a no-op for a duplicable
   instance. This covers guarded operands, operands used twice, fixed-array read places, `take`
   and owned arguments, and ownerless iterables. The loop-condition walk added for the
   loop-release fix is widened to the same `owns_drop || maybe_owns` predicate (Phase 3 uses
   `owns_drop` alone), so a param-typed owned call result in a condition materializes too (ST-18).

5. **Loan-owner registration.** `collect_lowered_temp_owners` (`own/lower.rs:854-868`) must use the
   same `owns_drop || maybe_owns` predicate at `:865`, so a view derived from a maybe-owning
   materialized temporary keeps its `LoanOwner::Temp` and the `E10030` protection. A bounded
   reachability probe (execution-plan task 4.2.6) runs first; if it finds no such view, the change
   is dropped and the reasoning recorded; if it does, the acceptance consequence (an outliving
   view rejected for every instance, including duplicable ones) requires an explicit ruling before
   the change ships.

### The site table (completeness proof)

Fourteen sites across twelve rows; each is pinned by a specification case or the task 4.2.6 probe
(the site→case map is in `07-testing-strategy.md`).

| Site | Current skip | Duty |
| ---- | ------------ | ---- |
| `own/lower.rs:619` | `materialize_guard_operand` leaves a param-typed operand inline | Materialize (rule 4) |
| `own/lower.rs:705` | `consume_place` does not move a param-typed place | Move-if-owning (rule 2) + conflict (rule 3) |
| `own/lower.rs:980` | `materialize_read_place` skips a param-typed fixed-array read | Materialize (rule 4) |
| `own/lower.rs:1051` | `materialize_operand` skips a param-typed two-use operand | Materialize (rule 4) |
| `own/lower.rs:1090`, `:1104` | `materialize_owned_argument` skips a `take` argument | Materialize (rule 4) |
| `own/lower.rs:1117` | `materialize_owned_argument` skips binding an owned argument | Maybe-owning local (rule 1) + materialize (rule 4) |
| `own/expr.rs:50` | `consume_owned_place` records no move | Move-if-owning (rule 2) + conflict (rule 3) |
| `own/expr.rs:85` | `conditional_move_sources` records no maybe-moved arm | Move-if-owning (rule 2) + conflict (rule 3) |
| `own/expr.rs:108` | `mark_read_conditional_move` records no move | Move-if-owning (rule 2) + conflict (rule 3) |
| `own/loops.rs:503` | `materialize_for_in_iterable` leaves an ownerless iterable inline | Materialize (rule 4) |
| `own/call_check.rs:281`, `own/call.rs:352` | the take-argument move and move-out skip a param-typed place | Move-if-owning (rule 2) + conflict (rule 3) |
| `own/lower.rs:865` | `collect_lowered_temp_owners` skips a maybe-owning materialized temporary | Loan-owner predicate (rule 5) |

The flow recorders (rule 3) keep the checker's copy semantics for param types and deposit the
conflict duty; they never make a param-typed program reject at declaration.

## Implementation Details

### New records

```
// ir/mono.rs, next to CopyObligation
pub struct RequiresDuplicable {
    pub function: String,   // declared generic function
    pub binding: String,    // the IR binding name whose later use conflicts
    pub span: Span,         // the conflicting use, for the body-site diagnostic
}
```

`Monomorphization` gains `require_duplicable(..)` and `duplicable_requirements()` (the
`CopyObligation` accessors are the shape to mirror, but note that record currently has no
discharge consumer). `FunctionChecker` runs before `Monomorphization` exists
(`compiler/src/own.rs:104-122`), so each checked function returns its `Vec<RequiresDuplicable>`
and `own::check` merges them into the store before `instance_check` runs.

### Checker rules

- `maybe_owns(ty)` = `mentions_type_param(ty) && !is_view(ty) && !ty.is_unknown() && !ty.is_void()`.
- Binding registration uses `owns_drop(ty) || maybe_owns(ty)`; `declare_param` keeps the
  `mode == Mode::Take` gate (`own.rs:630`).
- A store or `take` argument from a param-mentioning place emits the move shape; `PlaceState`
  gains a `moved_if_owning` marker per binding, set at the store/argument and merged
  conservatively by the existing join.
- The name-read hook `read_place` records `RequiresDuplicable` once per function and binding when
  it sees the marker; a whole-binding write clears the marker.

### Codegen discharge

- `Statement::Drop` / `ReleaseTemp` / `Statement::Move` already substitute; the move emission
  gains the non-owning branch (`plain copy, no zero`) selected by
  `maybe_owns(declared) && !needs_drop(substituted)`.
- `take_move_out_places` (`own/call.rs:346-368`) uses the same substituted-type branch to empty
  only an owning argument place.
- No per-instance body, no instance-keyed table; the same IR emits correctly for every instance.

## Code Examples

```clorraine
// Owning instance releases the parameter; a scalar instance no-ops.
func keep<T>(take a: T[2]): i32 { return 0; }
let xs: String[2] = ...;
keep<String>(xs);            // frees the parameter's elements

// Move under an owning instance; copy under a duplicable one.
func dup<T>(take a: T): T { let b: T = a; return b; }
// String: b = a; a = NULL; ...   i32: b = a; a stays readable

// A body that needs the source again requires duplicability.
func reuse<T>(take a: T): i32 { let b: T = a; sink(borrow a); return 0; }
// String instance: E10020 once, at the body's use; i32 instance: compiles
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A later use after a move-if-owning store (read, second consume, borrow, or `take` argument) | `RequiresDuplicable` duty; `E10020` once per function and binding, at the body's use site | AR #11, #17 |
| `borrow` parameter under an owning substitution | Never maybe-owning: the `Mode::Take` gate keeps the callee from dropping; the caller releases once (ST-13) | AR #17 |
| `take` argument place under an owning substitution | The caller's place is emptied through the argument move-out; the callee owns and releases (ST-19) | AR #17 |
| View, unknown, or void parameter | Not maybe-owning: no drop, no move, no materialization | AR #17 |
| Duplicable substitution | Every duty discharges as a no-op; the copy flow and the accepted set are unchanged | AR #11 |
| Nested parameter (`T[2]`, `Box<T>`) | `maybe_owns` recurses through `mentions_type_param`; the substituted drop recurses as any owned aggregate does | AR #17 |
| Generic body with no param-typed ownership | No duty deposited; the instance is unchanged | AR #11 |

## Testing Requirements

- Spec cases ST-10 – ST-16 and ST-18 – ST-22 (`07-testing-strategy.md`), mapped per site and per
  duty kind (the site→case map is in `07-testing-strategy.md`).
- An AddressSanitizer pre-fix probe for the filed take-parameter leak and the take-argument
  double-free shape (red-phase evidence).
- Impl tests: the emitted drop and move shapes per instance, the non-owning no-op branch, the
  emptied `take` argument place, and `E10020` reported once per function and binding at the
  body's use site.
- Unit tests for `maybe_owns`, the parameter-mode gate, and the `RequiresDuplicable` discharge in
  `instance_check`; the task 4.2.6 loan-owner reachability probe.
