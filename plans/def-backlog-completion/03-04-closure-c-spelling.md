# Closure C Spelling: Def Backlog Completion

> **Document**: 03-04-closure-c-spelling.md
> **Parent**: [Index](00-index.md)

## Overview

Close DEF-57 and DEF-58. DEF-57: a specialized capturing closure passed to a callable-bound generic
parameter is recorded without its enclosing instance, so the callee's parameter type and the
call-site value spell different C structs. DEF-58: the closure call renderer writes a trailing comma
for an empty argument list. This document owns both behaviors; the reproduction evidence is in
[02-current-state.md](02-current-state.md) §Reproductions, and the ruling is AR-3 (its resolution
note covers the separator rule).

## Architecture

### Current Architecture

Sema creates `ClosureTy { id, params, returns, instance: None }`
(`compiler/src/types/mod.rs:121-135`, `compiler/src/sema/expr.rs:209-217`). The inferred
callable-bound argument is recorded unchanged during instance collection
(`compiler/src/own/call.rs:226-248`, `compiler/src/own/generics.rs:734-758`). Only the emitter tags
closures, from the active instance key (`compiler/src/codegen.rs:331-367`), so the call-site value
spells `ClClosure_0__i32` while the callee signature — built from the recorded argument through
`instance_subst` (`codegen.rs:542-548`) and `closure_type` (`codegen/types.rs:334-339`) — spells
`ClClosure_0`. The closure arm of **`type_key`** (`codegen/types.rs:277-309`, closure at `:306`),
which names C symbols through `instance_typekey` (`codegen.rs:555-558`), ignores `.instance`, as
does `dedupe_arg` (`ir/mono.rs:44-49`), which feeds the instance dedupe/diagnostic key
(`Instance::dedupe_key`, `ir/mono.rs:33-36`).

## Proposed Changes

1. **Relocate the key renderer.** Move `type_key` and its two pure helpers
   (`slice_base_element`, `slice_element_key`, `compiler/src/codegen/types.rs:317-328`) from
   `compiler/src/codegen/types.rs` to the stage-neutral `compiler/src/types/` module as
   `pub(crate)`, so the ownership stage can render an instance key without importing codegen
   (AR-3, PF-019). The slice arm keeps calling `slice_element_key`, so every `ClSlice_`/`slice_`
   spelling is byte-identical.
2. **Include the instance in the closure key.** The closure arm of `type_key` renders
   `closure_{id}__{key}` when `instance` is set and `closure_{id}` otherwise. The C symbols come
   from `type_key` through `instance_typekey` (`codegen.rs:555-558`, joined with `_`), so two
   enclosing specializations of one callable-bound instance stay distinct; `dedupe_arg`
   (`ir/mono.rs:44-49`) gets the same distinction for the diagnostic/obligation key
   (`dedupe_key`, `ir/mono.rs:33-36`) — `type_key` is the symbol renderer, `dedupe_arg` the
   dedupe key (PF-030, ST-27).
3. **Stamp the key at recording.** The instance-collection walk (`own/generics.rs:453-510`) knows
   the enclosing instance's type arguments while it walks the body that creates the closure. The
   order matters: the walk records the callee instance before the loop records the enclosing
   lifted specializations (`own/generics.rs:489` vs `:494-508`), so the `record_lifted` block moves
   **before** `walk_function` in each iteration (PF-008). The stamp then applies to every
   closure-typed argument whose `instance` is `None` when the site has a lifted specialization for
   the enclosing instance — the same condition the emitter's `tag_closure_instance` uses
   (`codegen.rs:339-367`). A non-capturing or shared closure keeps `instance: None`, so non-generic
   callers keep their current spelling. The first pass over generic templates must also skip
   template bodies: a closure-only argument has no type parameter to mark it unresolved, so a
   template walk would otherwise record a plain copy of a closure-bearing instance alongside the
   stamped per-instance copy (found while closing ST-27).
4. **Omit the empty separator.** In `render_call`, the closure-typed-parameter branch
   (`codegen/expr.rs:458-461`) writes `.env, {arguments}` unconditionally. Branch on
   `argument_list.is_empty()`, mirroring `closure_call_with_destination`
   (`codegen/expr.rs:636-640`).

## Implementation Details

### Integration Points

| Site | Change |
| ---- | ------ |
| `compiler/src/types/` | Host the relocated `type_key`, `slice_base_element`, and `slice_element_key` (`pub(crate)`, `Ty`-only) |
| `compiler/src/codegen/types.rs` | Use the relocated renderers; the closure C name keeps reading `.instance` |
| `compiler/src/own/generics.rs` | Record the enclosing lifted specializations before walking the body; stamp recorded closure arguments that have a lifted specialization |
| `compiler/src/ir/mono.rs` | `dedupe_arg` includes the closure instance (diagnostic/obligation key) |
| `compiler/src/codegen/types.rs` (`type_key` closure arm) | Instance appended to the closure key |
| `compiler/src/codegen/expr.rs` | Empty-argument closure call |

### Behavior Rules

- A closure argument with `instance: None` keeps `ClClosure_{id}` and `closure_{id}` everywhere
  (ST-18).
- A specialized capturing closure records its enclosing key and spells
  `ClClosure_{id}__{key}`/`closure_{id}__{key}` at both the call site and the callee.
- The enclosing key is the same instance key the emitter computes, so the stamp is stable across
  re-emission.
- Two enclosing specializations of one site produce two C symbols and both run (ST-17).
- A zero-argument call through a closure value renders `((f).fn((f).env))` (ST-19).

## Code Examples

```clorraine
// DEF-57: the callee parameter and the call-site value must spell the same
// callable struct for each enclosing instance.
func apply<T, F: func(T): T>(x: T, f: F): T { return f(x); }
func outer<T>(x: T): T { return apply(x, [x] (y: T) => y); }
let a: i32 = outer<i32>(41);
let b: str = outer<String>(stringOf("hi"));   // second enclosing specialization
```

```clorraine
// DEF-58: a zero-argument call through a closure value.
func consume<F: func(): i32>(f: F): i32 { return f(); }
let n: i32 = consume([x] () => x);
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Specialized closure argument recorded without the enclosing key | Record the enclosing lifted specializations first, then stamp during the walk | AR-3, PF-008 |
| Closure without a lifted specialization for the enclosing instance | Leave `instance: None`; spelling unchanged | AR-3 |
| Two enclosing specializations of one site | Distinct keys in `type_key` (symbols) and `dedupe_arg` (dedupe/diagnostics) | AR-3, PF-030 |
| Key join format | The enclosing key joins with `_` through `instance_typekey`, matching the instance symbol | PF-030 |
| Zero-argument closure call | Omit the separator; no other rendering change | AR-3 |
| Non-generic caller | Unchanged `ClClosure_{id}` spelling | AR-9 |
| Stage boundary | `type_key` and its slice helpers live in `types`; `own` does not import `codegen` | AR-3, PF-019 |

> **Traceability:** Every error-handling strategy references the Ambiguity Register entry (AR #)
> that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification cases in [07-testing-strategy.md](07-testing-strategy.md): ST-16 (the DEF-57
  reproduction), ST-17 (two enclosing specializations grounded on a buildable program), ST-18 (the
  non-generic spelling regression), ST-19 (the zero-argument call), and ST-27 (the closure-only
  instance dedupe that the existing two-type-argument case cannot exercise).
- Implementation tests: the emitted struct names for both specializations, the empty-argument call
  text, the stamped instance present at recording time, byte-identical re-emission, and the symbol
  key shapes.
- Full verification: `make check`.
