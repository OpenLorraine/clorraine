# Fixed-Array Value Positions: Call Value Integrity

> **Document**: 03-02-fixed-array-value-positions.md
> **Parent**: [Index](00-index.md)

## Overview

Fix DEF-31 (indexed fixed-array element store emits invalid C), DEF-32 and DEF-33 (conditional
fixed-array arguments with call or literal/duplicable arms), DEF-35 (array-literal projection), and
DEF-38 (struct-literal array field). Every fix routes the value through machinery the
runtime-value-integrity plan already built.

**Decisions per:** AR #7 (struct field), AR #11 (duplicable materialization), AR #13 (sibling
paths).

## Architecture

### Current Architecture

- A store of an element routes through `Statement::StorePointer`
  (`compiler/src/codegen/function.rs:390-399`), whose target is `*(&(rows[i]))`; the element's place
  type comes from `projected_type`'s Element arm (`compiler/src/own.rs:774-776`), which returns the
  base scalar for a multi-extent array, so the scalar branch of `emit_assign`
  (`compiler/src/codegen/function.rs:716-721`) is taken instead of `emit_fixed_array_assign`
  (`:760`).
- Lowering materializes an owned argument only for non-duplicable values
  (`materialize_owned_argument`, `compiler/src/own/lower.rs:950-953`); a duplicable `i32[2]` in a
  conditional argument reaches C as a value expression. Non-duplicable `String[2]` already works
  through the same helper and `emit_fixed_array_call_assign` (`compiler/src/codegen/function.rs:973`).
- A projection lowers through `materialize_guard_operand` (`compiler/src/own/lower.rs:579-589`),
  which returns fixed-array non-call values unchanged, so an `ArrayLit` source has no place.
- Struct literals render fields inline (`compiler/src/codegen/expr.rs:681-708`); C cannot copy an
  array into an aggregate initializer from an expression.

### Proposed Changes

1. **Element stores (DEF-31).** When the store target's element type is a fixed array (constant or
   runtime index), the store uses the fixed-array assign machinery: byte-copy for a value that is a
   place, the branch store for a conditional, and `emit_array_assign_to`
   (`compiler/src/codegen/function.rs:1049`) for a literal. A runtime index cannot live in a
   `Place` (`Projection::Element` carries a constant only, `compiler/src/ir/place.rs:126-131`), so
   the fixed-array assign core (and its conditional and literal arms) is refactored to take a
   **rendered C lvalue** as its target, and a rendered-target entry point handles the
   `Statement::StorePointer` path when the element type is a fixed array. The pointer path's
   index/bounds guard emission (`compiler/src/codegen/function.rs:390-399`) runs first, as today;
   the constant-index path keeps using `emit_assign`/`emit_fixed_array_assign`. The
   `projected_type` Element arm applies the full element type for every extent so nested arrays
   keep their array type.

2. **Conditional and nested arguments (DEF-32, DEF-33).** The duplicable skip at
   `compiler/src/own/lower.rs:953` is gated on the position: a duplicable fixed array materializes
   whenever the C position needs a place (conditional arm, nested call argument, literal projection,
   struct-literal field). The existing destination machinery (`materialize_owned_argument`,
   `emit_fixed_array_call_assign`, `call_with_destination`) then handles both element types
   uniformly.

3. **Projections (DEF-35).** A fixed-array `ArrayLit` source at a projection lowers through
   `materialize_value_type` (`compiler/src/own/lower.rs:888`) before the index, and the argument
   form applies the same materialization. The nested element access then reads a real place.

4. **Struct-literal initialization (DEF-38).** When any field value has no C initializer form (a
   fixed array, or an owned conditional value, sourced from a conditional, call, projection, or
   literal), the literal **materializes into a temporary** that can host statements: the temporary
   is zero-initialized and **every** field is stored in source order — array fields through the
   fixed-array assign machinery (`emit_array_assign_to` or the branch store), owned values through
   the owned conditional store ([03-01](03-01-scalar-conditional-ownership.md)). Storing all
   fields is the single rule: a C aggregate initializer cannot mix with later stores without
   reordering side-effecting fields, and `spec/04-expressions.md` (struct fields evaluate in
   source order) forbids that. The place-requiring materialization extends the AR #11 gate so
   call-argument, `return`, and nested-literal positions get their destination.

## Implementation Details

### Changed Functions

| Function | File | Change |
| -------- | ---- | ------ |
| `emit_indexed_store` / `indexed_target` | `compiler/src/own/expr.rs` | Route a fixed-array element target through the rendered-target array assign (guards first); keep the constant-index place path |
| Fixed-array assign core | `compiler/src/codegen/function.rs` | Take a rendered C lvalue as the target so the `StorePointer` path can reuse the same core |
| `Statement::StorePointer` emission | `compiler/src/codegen/function.rs` | Rendered-target fixed-array assign/branch store when the element type is a fixed array |
| `projected_type` Element arm | `compiler/src/own.rs` | Return the full element type for every array extent |
| `materialize_owned_argument` | `compiler/src/own/lower.rs` | Gate the duplicable skip on "position needs a place"; materialize a struct literal with a place-less field |
| `materialize_guard_operand` | `compiler/src/own/lower.rs` | Materialize fixed-array `ArrayLit` sources at projections |
| `struct_literal` | `compiler/src/codegen/expr.rs` | Render the aggregate initializer only when every field has one; otherwise emit zero-init plus per-field stores at the materialized destination |

### Integration Points

- The new materialization reuses the existing temporaries and the pending-release rules; no new IR
  node is introduced.
- The rendered-target refactor keeps one shared core: the `Place` entry point renders the place
  text and delegates, so the constant-index and runtime-index paths cannot drift.
- Byte-copy semantics stay exactly the fixed-array rule (one full copy per store; the source stays
  owned unless the arm moves it).
- Evaluation order stays left-to-right in source order for literals and arguments; a materialized
  struct literal stores every field in source order.

## Code Examples

```clorraine
let rows: i32[2][2] = [[1, 2], [3, 4]];
rows[i] = a;                      // DEF-31
rows[0] = [x, y];                 // DEF-31 nested literal
eat(ok ? mkA() : mkB());          // DEF-32
eat(ok ? [1, 2] : [3, 4]);        // DEF-33
let s: str = str([a, b][0]);      // DEF-35
let box: Box = Box { arr: ok ? a : b, tag: 1 }; // DEF-38
eat(Box { arr: ok ? a : b, tag: 1 });           // DEF-38 in an argument position
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Duplicable value in a place-requiring position | Materialize; the value copies | AR #11 |
| Runtime-index element target | Rendered-target array assign; the index guard runs before the store | AR #13 |
| Struct field that cannot be initialized | The literal materializes; zero-init plus stores for every field in source order; no C diagnostic | AR #7 |
| Owned conditional value in a struct-literal field | The field is an owning destination (03-01); the field store takes the value | AR #7 |
| Conditional arm that moves an owned array | Existing move rules and join (unchanged) | AR #13 |

> **Traceability:** Every error-handling strategy references the Ambiguity Register entry that
> resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: ST-9 – ST-17 and ST-41 in `07-testing-strategy.md` (host runs).
- Implementation tests: emitted-C shapes for the constant- and runtime-index element store (one
  shared core), the materialized argument and struct-literal temporaries, the projection
  temporary, and the per-field struct store.
