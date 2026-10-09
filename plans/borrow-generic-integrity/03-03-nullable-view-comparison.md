# Nullable View Comparison: Borrow/Generic Integrity

> **Document**: 03-03-nullable-view-comparison.md
> **Parent**: [Index](00-index.md)

## Overview

This component owns the P3 fix for DEF-81: the operator rules exclude views from `==`/`!=`, so a
nullable view cannot be compared with `null` and therefore cannot be narrowed, even though the
frozen decision T8 defines views as nullable and the emitter already lowers the null test.

## Architecture

### Current Architecture

- `compiler/src/sema/ops.rs comparable` accepts scalars, enums, `raw`, handles, `Str`,
  `CString`, and `StaticStr`; a slice (an array with an omitted extent) is not comparable, so the
  equality arm reports `E10222` on the view operand.
- The null literal types as `TyKind::Unknown` with `nullable: true`, which `comparable` already
  accepts, so only the view operand blocks the comparison.
- `compiler/src/codegen/expr.rs view_null_test` already lowers a view-versus-`null` comparison to
  `(view).ptr == NULL` (or `!=`), including the two-null-literal case.
- The narrowing machinery is type-agnostic: `own/place.rs narrowing` matches any
  `place == null` / `!= null` condition shape and `own/statements.rs apply_narrowing` records the
  narrowed place path; E10010 then stops reporting a use of the narrowed view.

### Proposed Changes

In the equality arm of `sema/ops.rs binary_result`, before the per-operand `comparable` checks,
detect the view-versus-`null` pairing: when one operand is a view (`is_view` in
`compiler/src/borrow.rs` — `str`, `static str`, `cstring`, or a slice) and the other operand is
the null type (`TyKind::Unknown`), accept the comparison. Every other combination keeps today's
rule:

- `view == view` (neither side null) stays `E10222` on the view operand;
- ordering (`<`, `<=`, `>`, `>=`) on a view stays `E10222`;
- handle, `str`, and scalar comparisons are unchanged.

The view's own nullability does not restrict the pairing (AR-7): a non-nullable view compared
with `null` is accepted, consistent with the handle path.

## Implementation Details

### New Types/Interfaces

None — one arm of `binary_result` gains a pairing test.

### Integration Points

- The narrowing path needs no change (verified in [02-current-state.md](02-current-state.md));
  the run tests pin it end-to-end.
- The emitter path needs no change (`view_null_test`); the acceptance tests pin the lowered
  `.ptr` form through the run behavior.

## Code Examples

```clorraine
func firstOr(xs: u8[]?, fallback: u8): u8 {
    if (xs != null) {
        return xs[0];          // narrowed; reads through the view
    }
    return fallback;
}

// Rejected: two views have no defined equality.
// let same: bool = xs == ys;   // E10222
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| View versus `null` (either order, any view nullability) | Accepted | AR-7 |
| View versus view | `E10222`, unchanged | AR-7 |
| View ordering | `E10222`, unchanged | AR-7 |
| Handle/`str`/scalar comparisons | Unchanged | — |

> **Traceability:** every rule above back-references the register entry that resolved it. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification tests ST-13 – ST-16 ([07-testing-strategy.md](07-testing-strategy.md)): acceptance
  in both operand orders, narrowing with a run-time read, the preserved rejections, and the
  non-nullable view form.
- Implementation tests pin the emitted `.ptr` comparison and the diagnostic for view-versus-view.
