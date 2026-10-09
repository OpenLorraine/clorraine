# May-Lend Callee Identity: Borrow/Generic Integrity

> **Document**: 03-04-may-lend-callee-identity.md
> **Parent**: [Index](00-index.md)

## Overview

This component owns the P4 fix for DEF-77: the may-lend gate resolves a lowered callee by name; a
func-typed parameter whose source name equals a module lender keeps its unsuffixed IR name, so the
gate consults the module function, treats the call as lending, and skips binding the owned result
— a leak.

## Architecture

### Current Architecture

- `compiler/src/own.rs unique_ir_name` suffixes a binding's IR name with `#{n}` when the name is
  shadowed or when a `BindingKind::Local` masks a module callee name
  (`self.program.callee(name).is_some()`); parameters are not masked.
- `compiler/src/own/call.rs lower_call` lowers a call through a func-typed binding to the
  binding's IR name; for a parameter this is the plain source name.
- `compiler/src/own/lower.rs value_may_lend` → `lowered_lend_source` resolves that callee name
  through `own/program.rs callee`, which matches a module function, `ffi` declaration, or modeled
  standard-library helper. A collision therefore makes the gate believe the call reborrows an
  owned argument, so the owned result is not bound and leaks.

### Proposed Changes

Extend the masking condition in `unique_ir_name` from `BindingKind::Local` to every binding kind
whose IR name can be a lowered callee (AR-3): suffix the name whenever
`self.program.callee(name).is_some()` (the shadowing clause stays). A func-typed parameter (or
any other binding) named like a module callee then carries a suffixed IR name that
`program.callee` never resolves, so the may-lend gate skips the module lookup as it already does
for locals — and by construction the same holds for every name-based consumer
(`value_may_lend`/`lowered_lend_source`, `collect_lowered_temp_owners`, `take_move_out_places`,
`materialized_ty`).

**Complete the parameter-name plumbing (PF-001).** Unlike locals (mapped through
`function.locals`), a parameter's emitted C name comes from the AST source name: `ir::Param.name`
is built from `decl.params` (`own.rs`), the definition signature is emitted from `decl.params`
(`codegen.rs`), and `local_names` is keyed by the IR param name (`codegen/function.rs`), while
places and lowered callees use `binding.ir_name` (`own.rs`). Masking a parameter therefore
requires carrying the masked spelling into `ir::Param` (synced at declaration), the definition
signature and pointer-param table, and the func/closure lookups, so signature, places, and calls
agree. Sanitized spellings (`give#0` → `give_0`) join the emitter's used-name set so they cannot
collide with a literally-named `give_0` parameter.

Consequences to verify in the emission sweep:

- Emitted C names change only for bindings whose source name equals a module callee name;
  diagnostics keep the source name (`binding.name`).
- The two existing pinned tests that spell a colliding parameter update with the change
  (`runtime_value_integrity_impl_test.rs`, `call_value_integrity_impl_test.rs`; PF-001).
- A genuine module call lowers to the written name and resolves normally (ST-20).
- Determinism: the suffix number is the declaration-order binding count, unchanged.

## Implementation Details

### New Types/Interfaces

None beyond the parameter-name alignment — no new IR shape; the masked spelling travels through
the existing `ir::Param` and emitter tables (PF-001).

### Integration Points

- `lower_call`'s local-callee branch (`local_callee` marks the binding used and stops type
  inference) is unchanged; it now receives a suffixed name for colliding parameters.
- The four name-based consumers (may-lend gate, temp-owner collector, move-out places,
  materialized types) skip the module lookup for masked names by construction.
- The parameter-name plumbing lands in `own.rs`, `codegen.rs`, `codegen/function.rs`, and
  `codegen/expr.rs`; the latter is shared with P2 (sequential phases; PF-001).
- The emission-integrity review diffs the base and head emissions over the corpus; the only
  expected hunks are the colliding-name spellings and the signature/local-table alignment.

## Code Examples

```clorraine
func give(borrow v: String): str { return v; }      // the module lender

func makeIt(borrow v: String): String { return stringOf("y"); }

func demo(give: func(borrow String): String): i32 {
    let s: String = stringOf("x");
    return i32(len(give(s)));   // the call binds and releases the owned result
}
```

Before the fix the call result is never bound (ASan leak); after the fix the result is bound and
released once.

## Error Handling

No new diagnostics — the fix removes a leak for accepted source.

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Func-typed parameter named like a module callee | IR name suffixed; gate skips the module lookup | AR-3 |
| Genuine module call in a function with a colliding binding | Unchanged (resolves by the written name) | AR-3 |
| Shadowed/sanitized local (existing behavior) | Unchanged | AR-3 |

> **Traceability:** every rule above back-references the register entry that resolved it. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification tests ST-17 – ST-20 ([07-testing-strategy.md](07-testing-strategy.md)): the leak
  repro under the host ASan oracle (`RDC_ASAN`), the func-typed-field receiver case (ST-18), the
  genuine module lender path (ST-19), and the module-call pin with a colliding sibling binding
  (ST-20).
- Implementation tests pin the suffixed IR/emitted names for colliding bindings, the updated
  pins of the two existing tests, and the unchanged emission for a genuine module call.
