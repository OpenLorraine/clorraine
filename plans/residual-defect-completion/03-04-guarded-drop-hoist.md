# Guarded Drop Hoist: Residual Defect Completion

> **Document**: 03-04-guarded-drop-hoist.md
> **Parent**: [Index](00-index.md)

## Overview

This component fixes DEF-51: a conditionally evaluated materialized operand whose type needs an
element loop to drop (a growable `Array<T>` with a drop-needing `T`, or a struct containing one)
skips the previous-value drop on re-store, so each re-materialization leaks. Guards are C
comma-expression operands and C99 has no statement expressions, so the drop looked impossible to
express; the approved mechanism is the smaller statement hoist (AR-8), not a helper facility.

## Architecture

### Current Architecture

- `emit_guards` walks a value and emits its guards; a `Materialize` node pushes the
  expression-form drop of the previous value only when `drop_expr_text` returns `Some`
  (`compiler/src/codegen/guards.rs:636-684`, `:680-684`).
- `drop_expr_text` has no expression form for a growable array whose element needs a drop
  (`compiler/src/codegen/drop.rs:160-164`), and the `None` propagates through fixed arrays and
  structs (`:177-193`, `:196-219`). The caller therefore skips the drop.
- The statement-form path already exists: `emit_reassigned_temp_drop` (`guards.rs:215-226`) uses
  `emit_drop`, which recurses element loops via `emit_growable_array_drop`
  (`compiler/src/codegen/drop.rs:112-129`). It runs for always-evaluated materializes
  (`guards.rs:104-110`).
- Every guarded loop has a per-iteration statement slot: the test is moved into the loop body and
  `emit_guards(condition)` runs per iteration (`compiler/src/ir/guards.rs:71-74`,
  `compiler/src/codegen/control.rs:346-352`). Temps are declared zero-initialized
  (`compiler/src/codegen/function.rs:251-258`).

### Proposed Changes

Add a recursive walk in `emit_guards` over the conditionally evaluated operand — the right side of
`&&`/`||` and both `?:` arms — that finds every `Materialize` whose drop has no expression form
and emits the existing statement-form `emit_reassigned_temp_drop` for it. Nothing else changes:

- Types whose drop already has an expression form keep the expression drop and their emitted
  shape is unchanged.
- The always-evaluated path (`guards.rs:104-110`) is untouched.
- No helper functions, no registration, no name mangling.

Dropping at the top of the construct is safe: the temporary is read only at its own `Materialize`
node, an early drop is null-safe on the zero-initialized temp, and the post-construct release
still runs with the temp zeroed, so each stored value is released exactly once — even when the
operand short-circuits and the temp still holds the previous iteration's value.

## Implementation Details

### Changed functions

- `compiler/src/codegen/guards.rs` — `emit_guards` gains the conditional-subtree walk; the walk
  calls `emit_reassigned_temp_drop` for a `Materialize` where
  `self.drop_expr_text(&destination, &inner_ty).is_none()`. The walk applies to the `&&`/`||`
  right operand and `?:` arms (the subtrees `emit_guards` deliberately does not walk today).

The walk must not double-emit: a `Materialize` that already receives its expression drop in
`guard_expressions` keeps only that drop.

### Integration Points

- `guard_expressions` stays `&self`; the hoist lives in `emit_guards` (`&mut self`), whose call
  sites are exactly at each evaluation of the enclosing construct.
- The emitted statement order is: hoisted drop(s), then the guarded construct; the drop runs
  before the operand's conditionally evaluated side effects. Nothing in `spec/` fixes that
  internal ordering; each stored value is dropped once.

## Code Examples

```clorraine
func grow(): Array<String>[2] {
    let a: Array<String> = arrayNew<String>();
    a.push(stringOf("ab"));
    let b: Array<String> = arrayNew<String>();
    b.push(stringOf("cd"));
    return [a, b];
}

func main(): i32 {
    let i: i32 = 0;
    while (i < 2 && grow()[0].count > 0) { i = i + 1; }
    return i;
}
```

After the fix the loop body drops the previous `__cl_tmp0[0]` element loop before re-storing;
the program runs and the close-out probe is LSan-clean.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| The operand short-circuits this iteration | The hoisted drop still releases the previous stored value once; the temp is zeroed, so the post-loop release no-ops | AR-8 |
| The type has an expression-form drop | No hoist; the emitted shape is unchanged | AR-8 |
| The operand is always evaluated | The existing `emit_reassigned_temp_drop` path is untouched | AR-8 |
| A fixed array or struct containing a growable drop-needing element | The statement `emit_drop` recurses the same shape and the hoist covers it | AR-8 |
| The implementation test pinning the old skip | Rewritten to pin the fixed shape (AR-10); no specification case pinned the skip | AR-10 |

> **Traceability:** Every error-handling strategy references the Ambiguity Register entry. See
> `00-ambiguity-register.md`.

## Testing Requirements

- Specification cases ST-15 – ST-17 (07-testing-strategy.md): the `while` repro, the `do`/`for`
  spellings, and a struct containing a growable drop-needing element.
- Implementation tests: emitted statement-form drop before the re-store; the existing skip pin
  rewritten; ASan/LSan-clean runs; byte-identical re-emission.
