# Immutable-View Writes: Store-Emission Integrity

> **Document**: 03-02-immutable-view-writes.md
> **Parent**: [Index](00-index.md)

## Overview

Phase 2 rejects writes to elements of immutable views. The specification's mutability table
(Ch 02 §6) declares `str` and `static str` immutable, but the implementation accepts the write and
emits C that assigns through a `const` pointer. The fix adds the user-approved `E10042` diagnostic
(AR-2, AR-7), its registry row and golden (AR-12), and a shared write-path check (AR-6).

## Architecture

### Current Architecture

Write mutability is judged in the ownership layer (`compiler/src/own/place.rs`):

* `write_place` (99-121) reports `E10023` for a `const` binding and `E10024` for a read-only
  parameter mode (`is_readonly_param`, 193-196). Plain, compound, and update local writes all
  route through it (`check_assign` and `check_update` in `own/expr.rs`).
* Module-level targets take the `PlaceTarget::Global` branches of `check_assign`
  (`own/expr.rs:612-651`) and `check_update` (`:835-861`), which check only `const`ness.

Neither path models view-type immutability. The emitted C exposes it: `ClStr.ptr` is
`const u8 *`, so `s[0] = 65` fails the strict build (02-current-state.md probes d114, d114c,
d114d, p2a/p2b/p2c/p2f).

### Proposed Changes

A shared helper beside the existing write checks:

```rust
/// Reports `E10042` when the written place ends in an element of an immutable
/// view.
///
/// A `str` or `static str` view names storage the language never lets a write
/// reach: the bytes may be a literal's static storage or another owner's
/// memory under a shared loan. The check looks at the type the final element
/// projection is applied to, so a parameter, local, field, or module-level view
/// base is caught alike. A whole-view rebind and a write through a writable
/// (borrow-derived) slice are not element writes and stay accepted.
fn check_immutable_view_write(&mut self, id: usize, projections: &[ir::Projection], span: Span)
```

plus a global-base variant used by the two global branches (same predicate over the global's
declared type and projections).

Predicate: the last projection is an element projection (`Element` or `ElementTemp`) **and** the
type at the projection prefix is `TyKind::Str` or `TyKind::StaticStr`. The diagnostic name is the
binding (or `binding.field` when the prefix ends in a field projection).

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `own/place.rs` | the local call site inside `write_place`, before the state mutation |
| `own/expr.rs` | the global call sites in `check_assign` / `check_update` |
| `spec/14-diagnostics.md` | the authorized `E10042` row |
| `spec/tests/compile-fail/` | the authorized golden |
| the emitter | unchanged; an accepted program never reaches the bad store |

## Implementation Details

### Exact Registry Row (design detail of AR-7)

```markdown
| E10042 | Write to an element of an immutable view (`str` or `static str`) | `cannot write through immutable view '{name}'` | copy the bytes into an owned `String` or a `borrow`-derived `u8[]` view |
```

The golden `spec/tests/compile-fail/E10042_write_immutable_view.clo` follows the existing golden
shape (`// expect-error E10042`, a comment trigger, a module declaration, and the minimal
function) and is registered by the existing `spec/check.sh` mechanism.

### Check Order

The helper runs after `E10023`/`E10024` (a `const` view binding keeps its existing report) and
before the place-state mutation, so a rejected write changes no state.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Element write over `str`/`static str` (parameter, local, field, global) | one `E10042`; no translation unit is written | AR-2, AR-6 |
| `const` binding / read-only mode | unchanged `E10023` / `E10024` (checked first) | AR-6 |
| Whole-view rebind (`t = "xyz"`) | accepted | AR-6 |
| Write through a `borrow`-derived `u8[]` slice | accepted | AR-6 |
| Read of the same element | accepted | AR-6 |
| `cstring` bases | unchanged (not this type; read-only modes already report `E10024`) | AR-15 |

> **Traceability:** every design choice above back-references its register entry — see
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification cases ST-10 – ST-18 ([07-testing-strategy.md](07-testing-strategy.md)).
- The golden runs through `./spec/check.sh` (part of `make check`).
- Implementation pins: one report per form, no translation unit for a rejected write, the
  accepted controls' emission unchanged, byte-identical re-emission.
