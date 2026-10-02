# Type Keys and Call Arguments: Residual Defect Completion

> **Document**: 03-01-type-keys-and-call-arguments.md
> **Parent**: [Index](00-index.md)

## Overview

This component fixes DEF-50 (distinct fixed-array type arguments share a C symbol) and DEF-54 (a
generic-struct borrow argument is rendered by value). Both are wrong answers from a type-shaped
decision: the symbol key ignores array extents, and the argument pointer decision reads a type
before substitution.

## Architecture

### Current Architecture

`type_key` renders a stable key per type for symbol names (`compiler/src/codegen/types.rs:273-295`).
Its array arm returns the element's key and drops the extents (`:290`), so `i32[2]` and `i32[3]`
key identically and `Grid<i32[2]>`/`Grid<i32[3]>` share `ClGrid__i32`. The key also names slice
typedefs (`:56`, `:317`), instance symbols (`compiler/src/codegen/program.rs:382`), and text
helpers (`compiler/src/codegen/expr.rs:660`), so the fix must be correct for every use at once.

`call_arguments` decides the C argument form per parameter
(`compiler/src/codegen/calls.rs:132-184`). For a function-typed callee it substitutes the expected
parameter type before `is_pointer_param` (`:169-171`); for a named user call it tests `&arg.ty`
directly (`:174`). A written `T` argument carries the parameter at codegen time, so the decision
reads "value" while the substituted C parameter is a pointer.

### Proposed Changes

1. **Fixed-array keys carry their extents.** The array arm renders
   `arr<d0>x<d1>..._<element key>` when every extent is known, and `slice_<element key>` when an
   extent is unknown (a slice). Examples: `i32[2]` → `arr2_i32`, `i32[2][2]` → `arr2x2_i32`,
   `u8[]` → `slice_u8`. The element key recurses, so nested and struct element keys stay
   compositional.
2. **Slice keys use the base element.** A slice of a fixed row (`i32[3][]`) and the equivalent
   written row slice (`i32[][3]`) describe the same view, so both render `slice_<base element>`
   and share the declared `ClSlice_<base element>` name. The `slice` intrinsic's build sites use
   the same key, so a built view always matches the typedef the parameter declares.
3. **The named-call pointer decision substitutes first.** The named-call branch renders the
   decision from `self.substitute(&arg.ty)`.

## Implementation Details

### Changed functions

- `compiler/src/codegen/types.rs` — `type_key` array arm:

  ```rust
  TyKind::Array { element, extents } => {
      if extents.iter().all(Option::is_some) {
          let dims: Vec<String> = extents.iter().map(|size| size.unwrap().to_string()).collect();
          format!("arr{}_{}", dims.join("x"), type_key(element))
      } else {
          format!("slice_{}", type_key(element))
      }
  }
  ```

  The exact rendering is an implementation shape; the required behavior is R1 (distinct extent
  lists key distinctly, deterministically, and slices never collide with fixed arrays).
- `compiler/src/codegen/calls.rs` — inside the named-call `pointer` branch, evaluate
  `is_pointer_param(&self.substitute(&arg.ty), mode.unwrap_or(Mode::None), &self.sema.table)`.

### Integration Points

- The struct instance symbol (`types.rs:49`), the instance symbol (`program.rs:382`), slice
  typedefs (`types.rs:56`, `:317`), and the text helper (`expr.rs:660`) all consume `type_key`;
  one change covers them.
- `collect_slice_types` (`types.rs:310-324`) keeps declaring slice typedefs under the base
  element key; a slice built over a fixed row flattens to the same base element a written row
  slice uses, so one declared `ClSlice_<base>` covers both spellings and the typedef surface
  gains no undeclared names.

## Code Examples

```clorraine
struct Grid<T> { value: T; }

func main(): i32 {
    let a: Grid<i32[2]> = Grid<i32[2]> { value: [1, 2] };
    let b: Grid<i32[3]> = Grid<i32[3]> { value: [1, 2, 3] };
    return a.value[0] + b.value[0];
}
```

Emits two struct definitions (`ClGrid__arr2_i32`, `ClGrid__arr3_i32`) and builds.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A slice argument keys the same as a fixed array | `slice_` prefix; unit test pins both keys | AR-3 |
| A user struct name equals a generated key | Existing name-sanitization remains the last resort; structural encoding makes the collision the same class as today's `Name__arg` keys | AR-3 |
| A named call's argument is a param under a concrete caller | Regression pin: the substituted type drives the decision, the emitted C builds and runs | AR-3 |

> **Traceability:** Every error-handling strategy references the Ambiguity Register entry. See
> `00-ambiguity-register.md`.

## Testing Requirements

- Unit tests for `type_key`: scalar, multi-extent, nested-array, slice, and struct element keys
  (`compiler/src/codegen/types.rs` test module).
- Specification cases ST-1 – ST-6 (07-testing-strategy.md).
- Implementation tests: emitted symbol names differ per extent; a view built over a fixed row uses
  the declared `ClSlice_<base>` name; the borrow projection renders `&(p.v)`; re-emission is
  byte-identical.
