# Guarded, Discarded, and Carrier Roots: Conditional, Projection, and Update Residue II

> **Document**: 03-02-guarded-discarded-carrier-roots.md
> **Parent**: [Index](00-index.md)

## Overview

Four rows in the root-binding machinery batch I built: the guarded argument whose view element
wraps a fresh call root, the conditional carrier literal's taken arm, the discarded arm's
residual-owning root, and the bare discarded literal statement. The frozen rules are one owner
per created value, the guarded position evaluating only on its path, and an expression statement
running its effect and dropping the value. Every fix extends the batch-I root binders
(`bind_guarded_projection_root`, `hoist_ownerless_conditional_arms`, `defer_discarded_arm`,
`materialize_discarded_field`); none adds a pass (AR-14).

## Design

### 1. The deferred call-wrapped element root (`DEF-284`)

`g ? use(Holder { names: [id(mk().v)] }) : 0` hoists `__cl_owned0 = mk().v` outside the guard:
the call root's residual never releases (11 B leaked) and the skipped path still runs the call
(base-equal defect). The batch-I adoption covers the direct element positions, not a
call-wrapped element inside a deferred argument.

Extend the deferred branch under `defer_move_outs > 0`: the call-wrapped position's root binds
for its residual release on the taken path (the deferred fill materializes the root inside the
guard), and nothing runs on the skipped path. The direct element positions keep their pinned
shapes.

### 2. The conditional carrier literal's taken arm (`DEF-292`)

`let h: Holder = g ? Holder { names: [mk().v] } : Holder { names: ["zz"] };` with `g=true` reads
the correct byte but leaks the taken arm's fresh element root (11–22 B): the arm's fresh element
binds no scoped owner in the let position.

Bind the taken arm's fresh element root for the local's frame — the loop-condition and guarded
literal paths already do this — so the root releases at the local's scope end while the carried
view stays valid. The literal-arm spelling of the other arm is unaffected.

### 3. The discarded arm's residual-owning root (`DEF-293`)

`c ? i32(mk().v.len()) : t;` over a `Box { v: String; w: String; }`-shaped fresh root (the
residual sibling field is what leaks; a sole-field root is clean at the planning head)
leaks 11 B: the discard-arm gate keys on the selected type (an `i32`, which owns nothing),
so the fresh root's residual never releases. The root still owns storage even though the
selected value does not.

Judge the discarded arm's release need by the root's residual ownership, not only the selected
type: an arm whose root would leak residual storage binds the root for release after the
statement. An arm with no residual-owning root stays inline.

### 4. The bare discarded all-fresh literal (`DEF-297`)

`[mk().v];` as a bare statement leaks 42 B/4 allocations: the discarded-literal materialization
covers conditional arms only, so a non-conditional discarded literal binds no temporaries and
releases nothing.

Extend the discarded-literal materialization to the bare statement spelling: each fresh owning
leaf materializes into its own released statement temporary (the `materialize_discarded_field`
recursion the batch-I phase-4 correction built), and a place leaf stays a read.

## Integration Points

- `own/lower.rs`: `defer_discarded_arm` (:1165), `materialize_discarded_field` (:1232),
  `discard_binds_owner` (:1330), `bind_discarded_owner` (:1341),
  `bind_guarded_projection_root` (:3429), `own_ownerless_conditional_arms` (:3695),
  `hoist_ownerless_conditional_arms` (:4003), `adopt_view_backings` (:3300),
  `bind_ownerless_projection_root` (:3122).
- `codegen/guards.rs`: the deferred/guarded fill — the taken-path materialization.
- `codegen/function.rs`: the let-position arm fill and the discard emission.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A guarded argument with a call-wrapped element root | The root binds inside the deferred fill (taken path only); the skipped path runs nothing | — |
| A conditional carrier literal's place arm | Stays a read; no second owner | — |
| A discarded arm whose selected type owns nothing but whose root owns residual | The root binds; the selected value stays inline | — |
| A bare discarded literal with a place leaf | Stays a read; the fresh leaves release | — |
| A loop/re-execution spelling | One owner per execution; the release sits at the statement edge | — |

## Testing Requirements

- Spec: the guarded call-wrapped element case (both guard paths plus the call count; ASan), the
  conditional carrier literal both selections (ASan; the taken arm leak free), the discarded
  residual-root case (ASan), and the bare discarded literal (ASan; 42 B/4 allocs before); the
  direct-literal and conditional-discard controls stay green.
- Impl: the deferred fill shapes, the arm root bindings, the release counts, and the re-emission
  determinism pins.
