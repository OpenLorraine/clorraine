# Module Initializers: Store-Emission Integrity

> **Document**: 03-04-module-initializers.md
> **Parent**: [Index](00-index.md)

## Overview

Phase 4 makes module-level initializers emit valid C for fixed arrays and compiler intrinsics. Two
independent gaps live in the same emission path (DEF-116): the assignment of an array-typed value
and the lowering of a `slice(...)` call (AR-9, AR-10, AR-11).

## Architecture

### Current Architecture

* Module globals are declared with empty states (`emit_globals`,
  `compiler/src/codegen/program.rs:574-601`), then filled by `cl_init_<module>` functions in
  topological order (`emit_init_functions`, `:604-617`).
* `emit_global_init` (`:620-654`) lowers the initializer through `lower_global` (`:664-788`),
  emits the lowered value's guards, then pushes one assignment: `{target} = {rendered};`.
  For an array-typed target the rendered literal (`{ 1, 2 }`) is an invalid array assignment, and
  a slice initializer renders the callee's bare source name.
* `lower_global` handles every initializer form but builds a plain `ValueKind::Call` for calls;
  compiler intrinsics (`slice`, `rawSlice`, `rawStr`) have no symbol, so the emitted C references
  an undeclared `slice(...)`. The function-body path (`own/call.rs::lower_intrinsic_call`) already
  recognizes them and builds `ValueKind::Intrinsic`; the renderer
  (`codegen/calls.rs::intrinsic`) and the slice bounds guard (`codegen/guards.rs:98-104`) exist.
* The function-body element stores are the reference semantics for a fill: `[1, 2; 7]` emits
  `a[0] = 1; a[1] = 2; a[2] = 7;` (02-current-state.md probe fill1).

### Proposed Changes

1. **Array fill.** In `emit_global_init`, when the declared global type is a fixed array
   (`TyKind::Array` with fixed extents), replace the single assignment with a recursive
   element-wise walk over the target symbol and the declared type:

   ```text
   fill(target, element_ty, value, extents):
     for i in 0 .. extents[0]:
       element = literal's element at i, else the literal's fill, else the empty state
       if element_ty has further extents and element is itself an array value:
         fill(target[i], element_ty-minus-one, element, extents-minus-one)
       else:
         emit_guards(element); push "    {target}[{i}] = {render(element)};"
   ```

   A non-literal array value (a place, a call result) is copied element-wise from its rendered
   base: each element renders as an index over the lowered value
   (`ValueKind::Index { base, index }` with a constant index value), mirroring the shapes
   `lower_global` already builds for written index expressions. Element order follows the literal
   and the declared extents; the fill renders per remaining element exactly as the function-body
   element stores already do.
   A conditional array value (`pick() ? x : y`, AR-18) evaluates its condition once into an init
   local, then each element renders as `(cond ? left[i] : right[i])` — the arms are indexed per
   element (or read from their literal elements) so no arm array decays to a pointer.
2. **Intrinsic lowering.** `lower_global`'s call branch detects a compiler-intrinsic callee with
   the same rule as `own/call.rs::lower_intrinsic_call` (the name is a compiler intrinsic and the
   program does not resolve it to a user or `ffi` declaration; such a declaration keeps its own
   symbol) and builds `ValueKind::Intrinsic { name, args, type_arg }` from the lowered arguments
   and the resolved type argument. The renderer and the guard arm are reused as they are.
3. **Operand-once materialization.** For intrinsic arguments that are not pure values (mirroring
   `is_pure_value`, the predicate `materialize_guard_operand` uses), the init function first
   stores the rendered argument into a local (`__cl_itmp<N>`, numbered per init function) and the
   intrinsic value reads that local. The rendered slice expression reads `low` twice, so an
   impure offset would otherwise run twice (probe p4e).

## Implementation Details

### New Functions/Methods

```rust
/// Fills an array-typed global element-wise.
///
/// An array value has no C assignment form, so the init function writes one
/// element per statement, in declaration order. A nested array recurses one
/// extent per level; a literal's fill covers the elements its list omits.
fn emit_global_array_fill(&mut self, target: &str, ty: &Ty, value: &Value)

/// True when the call names a compiler intrinsic (and not a user or `ffi`
/// declaration of the same name), mirroring the function-body lowering.
fn global_intrinsic_call(&self, module: &Module, callee: &Expr/*name*/) -> Option<String>

/// Renders one intrinsic argument, materializing an impure operand into an
/// `__cl_itmp<N>` init local first so it runs exactly once.
fn init_intrinsic_arg(&mut self, value: Value) -> Value
```

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `codegen/program.rs` | `emit_global_init`, `lower_global`, the new helpers |
| `codegen/calls.rs` / `codegen/guards.rs` | unchanged renderer and guard arm, reused |
| `own/call.rs` | the detection rule mirrored (not shared) because the global context has no checker |
| `stdlib.rs` | `is_compiler_intrinsic` supplies the intrinsic list |

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Flat / nested / fill array literal | element-wise stores; guards per element | AR-9 |
| Array value from a place or call | element-wise copy from the rendered base | AR-9 |
| Conditional array value | condition once into an init local; per-element ternary over the indexed arms | AR-18 |
| Intrinsic call (`slice`) in an initializer | lowered to the intrinsic value; the bounds guard rides `emit_guards` | AR-10 |
| User or `ffi` declaration named `slice` | keeps its own symbol (never replaced) | AR-10 |
| Impure intrinsic operand | materialized once into an init local | AR-11 |
| `const` array initializer | unchanged: rejected `E10224` today (out of scope) | AR-15 |

> **Traceability:** every design choice above back-references its register entry — see
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification cases ST-23 – ST-29 ([07-testing-strategy.md](07-testing-strategy.md)).
- The emitted init functions are built under the strict flags and run; the impure-offset case
  asserts a single evaluation.
- Implementation pins: the fill statement order, the init-local materialization shape, the
  intrinsic rendering in an init function, byte-identical re-emission.
