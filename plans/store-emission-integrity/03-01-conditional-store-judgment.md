# Conditional-Store Judgment: Store-Emission Integrity

> **Document**: 03-01-conditional-store-judgment.md
> **Parent**: [Index](00-index.md)

## Overview

Phase 1 stops the conditional-result store through a non-nullable lend root from evading the
lend-store rule (DEF-113). The fix is a target-aware refinement of one suppression predicate; it
introduces no new record, diagnostic, or language surface (AR-4, AR-5).

## Architecture

### Current Architecture

For a whole-value store into a lend parameter, `check_lend_target_store`
(`compiler/src/own/expr.rs:698-773`) asks `lend_moves_judged` (`:785-808`) whether "the existing
path already reports this site". `lend_moves_judged` collects the store's sources — the direct
place value, or the leaves of a conditional via `maybe_owning_move_sources` /
`conditional_move_sources` — and defers (returns `true`) when **any** source is a lend parameter
(`Mode::None | Mode::Const | Mode::Borrow`).

The binding-effect marking path that consumes the same conditional applies a different rule: an
arm that names the target path itself is exempt ("every arm becomes maybe-moved except an arm
that names the target path itself", `compiler/src/own/expr.rs:554-576`). Because the suppression
predicate does not apply that exemption, a conditional whose only lend source is the target arm
(`slot = pick ? slot : other`, with `other` a local or a `take` parameter) is silently accepted:

* the marking path deliberately does not move the self arm, and
* the suppression counts that same self arm as "a lend the source side will report".

The emitted conditional store then frees the caller's saved handle on the non-slot arm
(02-current-state.md probe d113). The direct spellings are all pinned: a direct store from a
read-only lend reports `E10034` on the source (p1), a direct self-store reports `E10034` on the
target (p8), and a conditional with no lend arm already reports `E10034` on the target (p4).

The suppression's `any(lend)` breadth is deliberate: a mixed conditional (`slot = c ? local : v`,
pinned by the borrow-generic-integrity suite) must produce exactly one report at the site, and it
does — because its borrow-mode arm `v` is marked by the source side. The same holds for a
self-arm plus borrow-arm composition (one `E10034` on `v`, probe p9). The refinement below keeps
every one of those reports and only unmasks the composition whose sole lend source is the target.

### Proposed Changes

In `lend_moves_judged`:

1. Receive the target place (`PlaceTarget`) so sources can be compared against it.
2. When the value is not a direct place, filter the conditional walks' leaves: drop every leaf
   whose `(id, projections)` equals the target's (the self arm the marking path exempts).
3. Keep the direct-place branch exactly as today (the place-move path reports those, p1/p8).
4. After the filter, run the existing `any(lend)` test. The store rule then fires whenever the
   remaining sources are non-lends — which is exactly the reported-by-nobody condition.

No new duty record is needed: once the suppression stops firing, the existing flow reaches the
`maybe_owns` branch (`RequiresStorableSlot`, audited per instance) or the `owns_drop` branch
(`E10034`).

## Implementation Details

### New Functions/Methods

```rust
/// True when the lend-move path already judges a source the store's right side
/// would consume.
///
/// The source set mirrors the binding effect the assignment applies. An arm
/// that names the exact target path is exempt, mirroring the binding-effect
/// marking rule: storing a target into itself moves nothing out of it, and the
/// target-side rule must judge it. Every other source keeps the existing
/// suppression, because the place-move path (a direct value) or the per-arm
/// lend duties (read-only arms) report the site there.
fn lend_moves_judged(&self, target: &PlaceTarget, target_ty: &Ty, value: &Expr) -> bool
```

The only caller (`check_lend_target_store`) passes the resolved `PlaceTarget::Local(id, projections)`
it already holds. The doc comment must restate the exemption rationale in plain language (no plan
or requirement references).

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `own/expr.rs` | the suppression predicate and its single call site |
| `own/place.rs` | unchanged: `mark_moved_if_owning` / the duty records keep their behavior |
| `own/instance_check.rs` | unchanged: the storable-slot audit already maps the per-instance diagnostics |
| the emitter | unchanged: the accepted nullable out-slot emission (d113d) stays as is |

## Behavior Table (owned here; pinned by 07's ST cases)

| Shape | Today | After | Note |
| ----- | ----- | ----- | ---- |
| `borrow slot: String`, `slot = pick ? slot : other` (other local/take) | accepted, UAF | one `E10034` naming `slot` | AR-4/AR-5 |
| generic `borrow slot: T`, same shape, `T = String` | accepted, UAF | one `E10034` naming `slot` (duty) | AR-5 |
| generic, `T = i32` | accepted | accepted (duplicable copy) | AR-5 |
| generic, `T = String?` | accepted | accepted (out-slot carve-out) | d113d safety pin |
| nullable out-slot concrete, same shape | accepted, safe | accepted, safe | no change |
| conditional whose only other lend arm is read-only | `E10034` on the arm | unchanged | arity of reports unchanged |
| direct store from a read-only lend | `E10034` on the source | unchanged | regression pin |
| direct self-store | `E10034` on the target | unchanged | regression pin |

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Concrete owning lend root | one `E10034`, message names the target parameter | AR-5 |
| Parameter-typed slot | the existing `RequiresStorableSlot` duty; the audit reports once per first owning instance | AR-5 |
| Duplicable or view substitution | accepted (copy) | AR-5 |
| Nullable substitution | accepted (out-slot carve-out) | AR-5 |

> **Traceability:** every design choice above back-references its register entry — see
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification cases ST-1 – ST-9 ([07-testing-strategy.md](07-testing-strategy.md)).
- Host ASan probes for the memory cases (the red shapes and their accepted alternatives).
- Implementation pins: the exact report count and name per shape; byte-identical re-emission;
  the unchanged direct/read-only-arm reports.
