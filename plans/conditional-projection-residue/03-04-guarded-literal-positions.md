# Guarded and Discarded Literal Positions: Conditional, Projection, and Update Residue

> **Document**: 03-04-guarded-literal-positions.md
> **Parent**: [Index](00-index.md)

## Overview

Three rows in the guarded/discarded literal positions: a conditional literal receiver in a loop
condition gives one field two owners (double-free), a discarded struct-literal arm does not
materialize its fresh fields (leak), and a discarded conditional's fresh projection arm does not
bind its receiver root (leak). The frozen rules: one owner per created value; an expression
statement runs its effect and drops the value; a guarded position evaluates only on its path.

## Design

### 1. The loop-condition conditional literal (`DEF-262`, AR-12)

`while (i < 1 && (c ? H { … } : H { … }).call(5) > 0)` aborts with a double-free: the
loop-condition walk descends a whole-conditional `Materialize`'s arms and gives each literal's
owned-call field its own temporary while the literal temporary also owns it. The direct-literal
receiver fix (batch H P2) closed this for the direct spelling; the loop-condition walk and the
array-literal element variant still reproduce.

Reuse the direct-literal fix's per-arm binding for the loop-condition walk: the literal's owned
field keeps exactly one owner — the literal temporary — and the callable-field call reads through
it; the per-iteration re-evaluation builds a fresh literal and releases it at the iteration's
edge. The array-literal element variant of the same shape (an owned-call field reached through an
array-literal element) takes the same per-arm binding; the walk covers both literal forms.

### 2. The discarded struct-literal arm (`DEF-270`, AR-11-adjacent, AR-22)

`ok ? Pair { a: stringOf("x"), b: t } : Pair { a: stringOf("y"), b: u };` keeps the whole literal
inline because the place field aliases owned storage, so the fresh field is never released; and
the head treats the place field as a move, so a later use of `t` reports `E10020`.

Make the discard's per-arm decision per *field*: a fresh owning field materializes into its own
statement temporary (released after the statement); a field that reads existing owned storage (the
`b: t` case) stays inline and is a **read** — the discard never moves a place field, matching the
direct `ok ? t : u;` control (pinned: both arms are reads and `t`/`u` stay usable).
`field_aliases_owned_storage` already walks the field values; the decision moves from the whole
literal to each field, and the fresh field's release does not consume the place fields.

### 3. The discarded conditional's fresh projection arm (`DEF-271`, AR-4-adjacent)

`let t: String = stringOf("t"); c ? mk().v : t;` emits `(void)((c ? mk().v : t));` — the `mk()`
root's storage is never released. The discard-arm gate deliberately excludes projections so the
aliasing decision cannot misfire.

Bind an ownerless projection root inside a discarded conditional arm with the same root-binding
rule the return and store positions use (the statement temporary the discard already uses for
fresh results): the projected member stays a read and the root binds, releasing its un-moved
fields after the statement. The place arm stays a read.

## Integration Points

- `own/lower.rs`: `defer_discarded_arm` (:1019), `field_aliases_owned_storage` (:1069),
  `discard_binds_owner` (:1109), `bind_discarded_owner` (:1120), the loop-condition walk, and the
  conditional arm binders (:2778/:3077).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A place-aliasing literal field in a discarded arm | Stays inline as a read (no move, no second owner, no release); the field stays usable | AR-11-adjacent, AR-22 |
| A fresh literal field | Its own statement temporary, released once | AR-11-adjacent |
| A discarded conditional with a place arm | A read; never emptied | AR-4-adjacent |
| A re-executed loop-condition literal | A fresh literal per iteration; released at the iteration's edge | AR-12 |

## Testing Requirements

- Spec: the loop-condition conditional literal (single-shot and loop, ASan), the array-literal
  element variant, the `?:`/`||` literal positions, the discarded mixed-field literal (leak-free;
  `t`/`u` readable afterwards), and the discarded fresh projection arm (leak-free); the
  direct-literal twin and the batch-H discard pins stay green.
- Impl: the per-field materialization shapes, the root binding in the discarded arm, the release
  counts, and the re-emission determinism pins.
