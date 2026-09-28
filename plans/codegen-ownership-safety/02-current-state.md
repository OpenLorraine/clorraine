# Current State: Codegen Ownership Safety

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The ownership stage (`compiler/src/own/`) checks and lowers each function into
the IR; the code generator (`compiler/src/codegen/`) emits C99. Two behaviors
are wrong today, both on the path between a checked program and its C.

An element loop lowers to an index loop. For a `borrow` element the checker
binds a local, copies the element into it, and records a write-back on the
loop's exit frame:

- `compiler/src/own/loops.rs:294` `bind_for_in_element` declares the binding
  and emits `Statement::Assign { target: element binding, value: Index { .. } }`.
- `compiler/src/own/loops.rs:338` `set_for_in_write` records a `ForInWrite` on
  the innermost exit frame.
- `compiler/src/own/loops.rs:365` `store_for_in_element` emits the write-back
  `StorePointer` to the element.
- `compiler/src/own/statements.rs:523` and `:547` run the write-back before a
  `break` or `continue`.
- `compiler/src/own.rs:677` `binding_place` spells every local place with the
  binding's source name.
- `compiler/src/codegen/function.rs` `emit_assign` decides drops from the
  target place's type alone; `compiler/src/codegen/function.rs:87` maps one C
  name per source name.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/own/loops.rs` | Element-loop checking and binding | Alias borrow elements through a `raw` temp; delete `set_for_in_write`, `store_for_in_element` |
| `compiler/src/own/statements.rs` | `break`/`continue` hooks | Delete `run_for_in_write` and the frame reads |
| `compiler/src/own/flow.rs` | `ForInWrite`, `ExitFrame`, `Binding` | Delete `ForInWrite`; add `Binding.ir_name` |
| `compiler/src/own.rs` | `declare`, `binding_place`, `Function.locals` | Unique IR spelling; use it in `binding_place` and locals |
| `compiler/src/own/lower.rs` | Lifted-function locals | Use the unique IR spelling |
| `compiler/src/ir/place.rs` | `PlaceBase` | Add `DerefTemp { temp, ty }` |
| `compiler/src/ir.rs` | `Local` | Doc the unique spelling (no schema change) |
| `compiler/src/codegen/expr.rs` | `place_base`, `place_base_type` | Render and type the new deref base |
| `compiler/src/codegen/function.rs` | `emit_assign`, local declarations, `local_names` | No behavior change beyond the new base; verify unique names flow through |
| `compiler/tests/codegen_ownership_spec_test.rs` | New specification suite | Create |
| `compiler/tests/codegen_ownership_impl_test.rs` | New implementation suite | Create |

### Code Analysis

**Defect A — the element cache owns what it aliases.** This program frees an
element twice:

```clorraine
let arr: String[2] = [stringOf("a"), stringOf("b")];
for (borrow e in arr) {
    e = stringOf("c");
}
```

The emitted C (reproduced at `build/defs/f13.c`) is:

```c
for (; (__cl_tmp0 < 2); __cl_tmp0 = (__cl_tmp0 + 1u)) {
    if (e) { FreeString(e); }      /* copy-in: frees the PREVIOUS element */
    e = NULL;
    e = arr[__cl_tmp0];
    if (e) { FreeString(e); }      /* body write: O3 replacement */
    e = NULL;
    e = cl_std_text_stringOf(cl_str("c"));
    *(&(arr[__cl_tmp0])) = e;      /* write-back */
}
/* scope cleanup frees arr[0] and arr[1]: arr[0] is already dangling */
```

The first drop must not treat the cache as an owner: it aliases the element
written back in the previous iteration. A `return` inside the loop is also
wrong because the write-back only runs for fallthrough, `break`, and
`continue`; the element stays dangling and the cleanup frees it again
(reproduced at `build/defs/f13r.c`, ASan double free on both reproducers).

The code path never runs for the existing scalar element-loop tests
(`u32[4]`, `compiler/tests/codegen_run_spec_test.rs` `element_loops_copy_and_write_through`
and `element_loop_write_back_survives_break_and_continue`) because scalars do
not own storage, so the type-driven drop is a no-op there.

**Defect B — one C variable per source name.** The specification's own MOD-16
example miscompiles:

```clorraine
func pick(LIMIT: i32): i32 {
    let doubled: i32 = LIMIT * 2;   /* must read the parameter */
    let LIMIT: i32 = doubled;       /* shadows the parameter */
    return LIMIT;
}
```

The emitted C declares `i32 LIMIT` (the parameter) and `i32 LIMIT_1` (the
local), but the `LIMIT * 2` reference is rendered as `LIMIT_1`, which is still
its zero initializer at that point, because the local's entry overwrites the
parameter's entry in the single `local_names` map
(`compiler/src/codegen/function.rs:87`). The C compiles only without
`-Werror`; the value is wrong. The same mechanism makes two same-named locals
share one C variable (the declaration loop skips the duplicate name), so a
shadowed `String` is freed through the wrong name and the outer value is
destroyed: a block-local shadow in `build/defs/f14d.clo` segfaults under ASan.

## Gaps Identified

### Gap 1: Element views own their cache (DEF-13)

**Current Behavior:** the copy-in drops the previous element, and `return`
skips the write-back.
**Required Behavior:** the element binding aliases the element for the whole
iteration; all exits observe writes; replacement follows O3 on the element.
**Fix Required:** `PlaceBase::DerefTemp { temp, ty }` plus a per-iteration
`raw` temp address, replacing the copy-in/copy-back (AR #2, AR #6, AR #7).

### Gap 2: Binding identity is a source name (DEF-14)

**Current Behavior:** places and locals carry only the source name; codegen
maps collide under shadowing.
**Required Behavior:** every binding has a unique, deterministic IR spelling;
diagnostics keep the source name.
**Fix Required:** `Binding.ir_name` used by `binding_place` and both
`Function.locals` builders (AR #3, AR #9).

## Dependencies

### Internal Dependencies

- `compiler/src/own/` ownership stage (element loops, bindings, temps).
- `compiler/src/ir/` place and function schema.
- `compiler/src/codegen/` place rendering and local declaration.
- New test suites driven by the compiled `clc` binary.

### External Dependencies

None. The workspace stays std-only; no new crates, scripts, or harnesses.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| The alias changes emitted C for every borrow-element loop, including scalar loops | High | Medium | `make check` plus `make verify` cover all profiles and both toolchains; existing element-loop tests assert behavior |
| The `Function.locals` predicate is written too loosely while adding the alias filter | Low | High | Keep the full predicate `kind == BindingKind::Local && binding.base.is_none()`; captures are `BindingKind::Param` and were already excluded, so only the new alias is filtered |
| The new `PlaceBase` variant is missed by an exhaustive consumer | Low | High | The compiler fails to build on a missed match; add a unit test for rendering and typing |
| Speculative loop passes allocate the alias temp twice | Low | Medium | Temp ids come from the same pre-loop list on the clone and the real pass; a determinism test pins identical output |
| Behavioral tests only observe exit codes, so a silent leak stays invisible | Medium | Low | Host ASan probes are recorded evidence; the implementation suite adds a structural assertion that no write-back or spurious local is emitted |
