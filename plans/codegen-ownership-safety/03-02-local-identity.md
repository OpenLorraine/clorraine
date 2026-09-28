# Local Identity: Codegen Ownership Safety

> **Document**: 03-02-local-identity.md
> **Parent**: [Index](00-index.md)

## Overview

This document specifies the DEF-14 fix: every declared binding receives a
unique IR spelling, while the source name stays available for diagnostics
(AR #3). Shadowing then follows MOD-15/MOD-16 — the innermost binding wins —
for every form: local over local (nested and sibling scopes), local over
parameter, `const` locals, defer-machinery locals, generic instantiations, and
lifted lambda bodies (AR #9).

## Architecture

### Current Architecture

`Binding.name` (`compiler/src/own/flow.rs:32`) holds the source spelling, and
`binding_place` (`compiler/src/own.rs:677`) spells IR places with it. The C
emitter keeps one `local_names: BTreeMap<String, String>` per function and one
C variable per source name:

- Parameters are inserted by name (`compiler/src/codegen/function.rs:82`).
- Locals insert into the same map (`:87`) and the declaration loop skips
  duplicate names, so a local shadowing a parameter overwrites the parameter's
  entry and two same-named locals share one C variable.
- `place_base_type` (`compiler/src/codegen/expr.rs:185`) resolves a local's
  type by first match on the name.

Result: the specification's MOD-16 example reads an uninitialized renamed
local, and a shadowed droppable local is freed through the wrong name.

### Proposed Changes

1. Add `Binding.ir_name: String` next to `name`.
2. In `declare` (`compiler/src/own.rs:604`), compute the spelling from the
   binding table: the first binding with a given source name keeps it; a later
   binding with the same source name uses `name#<binding-index>` (AR #11).
   The index is the binding's position in `bindings`; the table is not
   truncated when a scope closes (`compiler/src/own.rs:356`), so every binding
   declared so far in the function counts and sibling scopes get distinct
   spellings. Speculative passes truncate and redeclare at the same indices, so
   the names stay deterministic.
3. Use `ir_name` in `binding_place` for both locals and parameters (`ir_name`
   equals `name` for the first binding of a name, which is always true for
   parameters).
4. Build `Function.locals` from `ir_name` for `BindingKind::Local` bindings
   whose base is not overridden
   (`kind == BindingKind::Local && binding.base.is_none()`); an alias declares
   no C local.
5. Update the `ir::Local` documentation: the name is the unique IR spelling,
   not necessarily the source spelling.

Diagnostics are unchanged: `binding_names` (binding id to source name) already
feeds every user-visible message and stays the source spelling.

## Implementation Details

### New Types

No new type; one new field:

```rust
/// The unique IR spelling of the binding.
///
/// The first binding with a given source name keeps that name; a later
/// binding that shadows it appends `#<index>`. The spelling is deterministic
/// and reaches the C emitter through `PlaceBase::Local` and `Local::name`;
/// diagnostics keep the source name. A base-overridden binding (a capture)
/// keeps the source name and is exempt: its places root at the override and it
/// never reaches `Function.locals`.
pub(super) ir_name: String,
```

### New Functions and Methods

- `compiler/src/own.rs` `declare`: compute `ir_name` before pushing the
  binding; store it on `Binding`.
- `compiler/src/own.rs` `binding_place`: spell places with `ir_name`.
- `compiler/src/own.rs` and `compiler/src/own/lower.rs`: both `Function.locals`
  builders map `Local { name: binding.ir_name.clone(), ty }` for
  `BindingKind::Local` bindings with no base override.
- `compiler/src/own/lower.rs` `declare_captures`: initialize `ir_name` with the
  capture's source name; the binding is base-overridden, so the spelling is
  never read.

### Integration Points

- The C emitter keeps its name-keyed maps; keys are now unique per binding, so
  `local_names` no longer collides and `place_base_type` resolves the right
  type. `unique_local` still dedups the final C names after
  `sanitize_identifier`, which is not injective (`#` becomes `_`).
- `liveness::check` receives binding ids and the `binding_names` map, so no
  change is needed.
- `unread_locals` (`compiler/src/codegen/function.rs:721`) operates on unique
  spellings and keeps working.

## Code Examples

### Example: MOD-16 (local shadows a parameter)

```clorraine
func pick(LIMIT: i32): i32 {
    let doubled: i32 = LIMIT * 2;   /* parameter, spelling `LIMIT` */
    let LIMIT: i32 = doubled;       /* local, spelling `LIMIT#2` */
    return LIMIT;                   /* reads `LIMIT#2` */
}
```

The parameter takes binding index 0, `doubled` index 1, and the shadowing local
index 2, so the local is spelled `LIMIT#2`; emitted C declares `LIMIT` and
`LIMIT#2` sanitized to distinct C names, `LIMIT * 2` reads the parameter, and
the function returns 6.

### Example: sibling scopes

```clorraine
{ let x: i32 = 7; }
{ let x: String = stringOf("q"); }
```

The two bindings get `x` and `x#1`; each block sees its own value and type.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Two sanitized spellings collide in C (`name#1` and a source `name_1`) | `unique_local` appends a suffix until the C name is free; the IR keys stay distinct | AR #11 |
| A capture or element alias would declare a spurious local | Base-overridden bindings are skipped in `Function.locals` | AR #7, AR #9 |
| Diagnostics must not leak the mangled spelling | Messages use `binding_names` (source names); no message changes | AR #16 |

> **Traceability:** Every error-handling strategy and design choice must
> reference the Ambiguity Register entry (AR #) that resolved it. See
> `00-ambiguity-register.md`.

## Testing Requirements

- Specification cases ST-8 – ST-15 in `07-testing-strategy.md`, covering every
  MOD-16 form.
- Implementation tests: a unit test that `declare` produces unique spellings
  under nested shadowing; determinism (compile twice, identical emitted C);
  a generic instantiation with a shadowing local; the callable-bound lambda
  form (`apply<T, F: func(T): T>` with `[base]`) for ST-15; a sanitized-name
  collision (`x#1` vs a source `x_1`) and a keyword-named shadow.
- Full behavioral regression: `make check`, then `make verify`.
