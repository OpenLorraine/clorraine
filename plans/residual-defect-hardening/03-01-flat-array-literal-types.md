# Flat Array Literal Types and the Deferred Row Store: Residual Defect Hardening

> **Document**: 03-01-flat-array-literal-types.md
> **Parent**: [Index](00-index.md)

## Overview

This component fixes DEF-46 in two parts: the canonical type of a no-context array literal, and
the deferred expression-form store for a nested row that is not itself a literal. Both are needed;
the type fix alone leaves the deferred spelling rejecting valid C, and the store alone cannot
fire because the nested shape never produces a row type.

Per AR #3 and AR #9, this is a representation change with no new acceptance or rejection rules:
the existing per-element checks, the fill rule, and the mismatch diagnostics are preserved.

## Architecture

### Current Architecture

- `Ty::array(element, extents)` stores the element and the extent list without folding
  (`compiler/src/types/mod.rs:247-255`); `substitute` rebuilds `TyKind::Array` directly
  (`:423-449`), so generic substitution can produce the nested shape even after a constructor
  fold.
- A context-typed literal takes the base element and the full extent list
  (`compiler/src/sema/expr.rs:693-705`); the no-context branch takes the first element's checked
  type as the element and appends one extent (`compiler/src/sema/expr.rs:726-760`), so nested
  literals become `array(array(T, [M]), [N])`.
- C spelling folds only the recorded extents: `c_type` prints the base element
  (`compiler/src/codegen/types.rs:52-58`) and `declarator` appends the extents (`:242-250`), so a
  nested element's extents vanish.
- `array_assign_expr` (`compiler/src/codegen/guards.rs:342-420`) recurses a row only when the row
  value is `ValueKind::ArrayLit`; every other row takes the scalar store (`:364-381`).

### Proposed Changes

1. **Fold fixed-array elements on every reachable construction path.** In `Ty::array`, when the
   outer extents are all present and `element` is itself a fixed array (all its extents present),
   fold: `element = element.element`, `extents = extents ++ element.extents`. Make `substitute`
   route through `Ty::array` (`compiler/src/types/mod.rs:429-436`) so a generic `T[N]`
   instantiated with a fixed-array argument is flat too. A slice element (any omitted extent) or
   a slice outer extent is left untouched, because an array *of slices* and a slice *of fixed
   rows* are distinct types. `resolve_ty_with` (`types/table.rs:566-575`) cannot nest fixed
   arrays because the parser flattens written multi-extent types; leave it unchanged and note
   why.
2. **No sema rule change.** `check_array_literal` keeps checking every element against the first
   element's type and keeps building its result through `Ty::array`; folding makes the result flat
   automatically. `iterated_element` and instance recording need no change. `type_key` stays
   base-element only: distinct fixed-array type arguments share C symbol names, an out-of-scope
   residual (DEF-50; see `01-requirements.md` Won't Have).
3. **Add the expression-form row store.** A new `array_row_expr` helper in
   `compiler/src/codegen/guards.rs` renders one nested row as a comma expression, mirroring the
   statement core's cases (`compiler/src/codegen/function.rs:1182-1294`). `array_assign_expr`
   calls it for a non-literal row instead of the scalar branch. Every arm of a conditional row
   ends in the same C value (a trailing `, 0` after the arm's store/move tail), applied uniformly
   through nested conditionals, so the ternary's operands have a common type. A storage-row move
   empties a `ValueKind::Place` source with the place-aware `move_zero_expr`
   (`codegen/expr.rs:367-378`), so a C-adjusted array-parameter row walks its elements instead of
   `cl_zero`-ing the adjusted pointer; a rendered non-place source uses `move_zero_text_expr`
   (`:385-393`).

## Implementation Details

### `Ty::array` folding

```
Ty::array(element, extents):
    if extents.all(present) and element is TyKind::Array { element_extents, element_base }
       and element_extents.all(present):
        return array(element_base, extents ++ element_extents)
    else:
        return Array { element, extents }
```

The rule is idempotent for an already-flat type and never fires for a slice shape. `is_slice`,
`is_fixed_array`, `iterated_element`, and `c_type`/`declarator` keep their current logic and now
agree by construction; `type_key` folds an array to its base element by design (DEF-50).

### `array_row_expr`

```
fn array_row_expr(&self, target_c: &str, row_ty: &Ty, value: &Value) -> Option<String>
```

The expression writes one row (an array lvalue `target_c`) from `value`, in source order, with
each operand's guards embedded where the operand is evaluated. Cases, matching
`emit_fixed_array_assign_arm`:

| Row value | Expression form |
| --------- | --------------- |
| `ArrayLit` | The existing `array_assign_expr` recursion for the row |
| `Conditional` | `({cond} ? {then_row} : {else_row})`, each arm rendered by this helper and ending in the same C value (a trailing `, 0` after the arm's store/move tail) |
| `Call` | `({arg and receiver guards}, call_with_destination(..., target_c), 0)` |
| `ClosureCall` | `({callable and arg guards}, closure_call_with_destination(..., target_c), 0)` |
| Storage value (place, member, index, temp, materialize) | `(memcpy({target_c}, {source address}, {bytes}))` plus a move-zero tail when the row type needs a drop |
| Any other | `(memcpy({target_c}, {source address}, {bytes}), 0)` |

Every arm of a conditional row ends in the same C value (the `, 0` tail), applied uniformly
through nested conditionals, so the ternary has one operand type. An owned row is a move: a
`ValueKind::Place` source is emptied with the place-aware `move_zero_expr`
(`compiler/src/codegen/expr.rs:367-378`, which walks a C-adjusted array parameter's elements),
and a rendered non-place source with `move_zero_text_expr` (`:385-393`), gated exactly as the
scalar element path gates it with `needs_drop(row_ty)` and the storage-root test.
`array_assign_expr` also uses the helper for the fill form's non-literal value, so a filled
multi-extent literal with a conditional fill works.

### Integration Points

- `compiler/src/types/mod.rs:247-255` — the constructor fold; `:423-449` — `substitute` routes
  through it.
- `compiler/src/sema/expr.rs:756-764` — unchanged; it benefits from the fold.
- `compiler/src/codegen/guards.rs:342-403` — the helper and its two call sites.
- `compiler/src/codegen/types.rs:52-58`, `:242-250`, `:290` — unchanged; verified by tests.

## Code Examples

### Example 1: Deferred conditional row

```clorraine
let hit: bool = ok && [c ? rows[0] : rows[1], [5, 6]][0][0] == 1;
```

The literal materializes into a flat `i32[2][2]` temp; the first row is a conditional, the second
a literal. The emitted C is a comma expression of two row stores followed by the element read.

### Example 2: Plain no-context literal

```clorraine
let lit = [c ? rows[0] : rows[1], [5, 6]];
```

`lit` is a flat `i32[2][2]` local; the statement form (which already handles both row shapes)
fills both rows.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Rows of different lengths | Unchanged: the later element is checked against the first element's type and rejected as today | AR #9 |
| Slice-valued element | Unchanged: a slice element does not fold, so an array of slices keeps its nested spelling | AR #9 |
| Fill with a non-duplicable element | Unchanged: the fill rule still reports `E10215` | AR #9 |
| Non-literal row of a fixed array with an owned element | The source is emptied in the row expression, so the literal owns the only reference | AR #3 |
| Non-literal row whose source is a C-adjusted array parameter | The place-aware move zero walks the elements, so the source is emptied exactly once | AR #3 |

## Testing Requirements

- Unit tests for the fold: a nested fixed element folds; a slice element and a slice outer extent
  do not; `substitute(T[2], T = i32[3])` is flat.
- Spec cases ST-1 – ST-5 and ST-17 (`07-testing-strategy.md`).
- Impl tests for the emitted row expression (conditional, place, call, and fill forms), the mixed
  literal/place conditional arm tail, the C-adjusted-parameter row move, the flat declarator
  (`i32 lit[2][2]`), and byte-identical re-emission.
- Existing tests to update: `types/table.rs`'s nested-array and depth-cap cases construct nested
  shapes through `Ty::array`; build the pathological shape with an explicit `TyKind::Array` once
  the constructor folds (`99-execution-plan.md` task 1.3.2).
