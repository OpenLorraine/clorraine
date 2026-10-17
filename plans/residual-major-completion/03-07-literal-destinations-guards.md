# Literal Destinations and Guards: Residual Major Completion

> **Document**: 03-07-literal-destinations-guards.md
> **Parent**: [Index](00-index.md)

## Overview

Two rows are literal-lowering gaps: a call-valued view-array field drops its hidden destination
at the read/unguarded positions (invalid C), and an owning nested conditional under a `&&` guard
fills its arm carrier before the guard, so the skipped path still runs a call. Both fixes extend
the existing literal/guard lowering; both are acceptances that must build and run with the
language's evaluation order.

## Design

### 1. The hidden view-array destination at every literal position (`DEF-378`)

`Inner { row: n1() }.row[0][0]` with `n1(): str[2]` is accepted and the strict build fails
(`too few arguments to function 'cl_probe_..._n1'; expected 1, have 0`): the literal keeps the
brace-list form, but a fixed view array is returned through a hidden destination parameter
(`static void cl_n1(ClStr __cl_ret[2])`). The guarded positions use
`materialize_guard_literal`; the **unguarded codegen value/read path** is decided by
`struct_literal_needs_stores` (`codegen/function.rs:1890`), which is not consulted for the
literal's value/projection-base position. The lowerer predicate `literal_field_needs_stores`
(`own/lower.rs:4563`) already returns true for a call-valued fixed-array field (the field is not a
brace list), so the fix is the read-path call site, not a new predicate. Frozen: Ch 04 §3
(`spec/04-expressions.md:53`) fixes function-call and array-literal evaluation order; Ch 04 §15
makes the literal elements typed by the expected type; Ch 17 §5 fixes the move shape. The hidden
destination is an internal emission mechanism; the frozen requirement is that the accepted
literal builds and evaluates each call once with the literal's value.

Fix: consult `struct_literal_needs_stores` (or route the literal through the lowerer's existing
store form) at the unguarded value/read position, so the destination is bound and the call is
emitted into it. The existing element-wise store path binds the destination, calls the function
into it, and copies the field; the change is the read-path call site, not a new ABI. Expected:
`readpos`, `gridel`, and `guardrowcond` build strictly and run 0; each call runs once; the concrete
non-call view-array literal control stays byte-identical.

### 2. The owning nested conditional under a guard (`DEF-379`)

`g && consume(ok ? Outer { inner: Inner { names: n1() }, tag: stringOf("x") } : Outer { ... })`
is accepted and runs 1 (expected 0): the emitted C fills the arm carrier `__cl_owned0` before
`if (g && consume(__cl_owned0) == 1)`, so the skipped `g == false` path still runs one `n1`/`n2`
call (calls = 3, expected 2). The eager carrier is `materialize_owned_argument`'s by-value binding
(`own/lower.rs:5109-5131`), which emits an `Assign` into `__cl_owned0` at statement entry even
when `defer_move_outs > 0`; the fill position is `codegen/guards.rs:106-117` (statement form) and
`:1016` (guarded expression form), and the drop/re-fill hoist is `guards.rs:273-307`.
`materialize_guard_literal` (`own/lower.rs:1697`) is the guarded-literal entry, not the by-value
argument carrier. Frozen: Ch 04 §3 (`:63`, the `&&`/`||` row: "Left operand first; the right
operand only when the result is not yet fixed"); Ch 04 §5 (`:108`: "only the chosen branch
evaluates").

Fix: defer the final by-value argument binding under `defer_move_outs` into the `Materialize`
channel instead of emitting the eager `Assign`, so the guarded codegen fills it on the evaluated
path; keep the call and its emptying on the same path. The phase's memory lens also analyzes the
guard drop/re-fill interaction (`guards.rs:273-307`) for a re-executed guard: the carrier's
previous value must not be released on the skipped path, and a re-fill on the evaluated path must
release the previous value exactly once. A loop fixture (`b379_loop`) re-executes the guard twice per use
(observed calls = 6, run 2; expected calls = 4, run 0), exercising the re-fill/re-drop/re-release
order. Expected: calls = 2 and run 0 in the single-pass fixture, calls = 4 and run 0 in the loop
fixture; ASan/LSan-clean for both selections and the loop; the impl pin asserts `__cl_owned0`'s
fill sits inside the guarded evaluated path and the re-fill releases the previous value once. The eager (unguarded) literal shapes keep their current order.

## Integration Points

- `compiler/src/own/lower.rs:4536` (`literal_fields_need_stores`), `:4563`
  (`literal_field_needs_stores`), `:4592` (`view_element_needs_binding`), the unguarded literal
  builder and the element-wise store path; `:5109-5131` (the by-value carrier),
  `materialize_owned_argument` (:4866), the `&&`/`||` right-operand lowering (:228-236).
- `compiler/src/codegen/function.rs:1890` (`struct_literal_needs_stores`, the unguarded
  value/read path) — the fix must consult it at the read/projection-base call site; `compiler/src/codegen/guards.rs:106-117` (the statement fill), `:1016` (the
  guarded expression fill), `:273-307` (the drop/re-fill hoists); `compiler/src/ir.rs` — the
  deferred argument carrier is `ValueKind::Materialize` (`ir.rs:298`: `move_outs: Vec<Place>` and `value_move_outs: Vec<Value>`).
- The batch-L phase-5 guarded view-literal machinery and the `DEF-323` cases.
- Controls: `probes/m378_readpos.clo`, `m378_gridel.clo`, `m378_guardrowcond.clo`,
  `probes/m379_guardnest.clo`, `preflight/b379_loop.clo`, `preflight/b379_eager.clo`; the batch-L
  literal/store origin pins.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A call-valued view-array field at a read/unguarded position | Bind the hidden destination; call once; copy the field | AR-22 |
| The same field in a guarded position | The existing guarded fill; no regression | AR-22 |
| A brace-list view-array literal with complete views | The current form (control); no unnecessary binding | AR-22 |
| An owning nested conditional under `&&` | Defer the by-value carrier into `Materialize`; fill it inside the evaluated guard path | AR-23 |
| The skipped guard path | Nothing runs (no call, no fill), and the carrier's previous value is not released | AR-23 |
| A re-executed guard | The evaluated re-fill releases the previous carrier value exactly once | AR-23 |
| A nested conditional not under a guard | The current eager order (control) | AR-23 |

## Testing Requirements

- **Spec cases** (`conditional_projection_residue_spec_test.rs`):
  - `should_materialize_a_hidden_view_destination_at_every_literal_position` (`DEF-378`): the
    read, grid-element, and guarded-row-condition shapes build and run 0; each call runs once;
    the unguarded codegen value/read path consults `struct_literal_needs_stores`; the concrete brace-list control is unchanged.
  - `should_keep_an_owning_nested_conditional_inside_its_guard` (`DEF-379`): both selections
    build and run 0; calls = 2 in the single-pass fixture and calls = 4 in the loop fixture
    (`b379_loop`); ASan/LSan-clean; a re-executed guard releases the previous carrier value
    exactly once; `b379_eager` is the eager-order control (run 0).
- **Impl pins** (`conditional_projection_residue_impl_test.rs`): the bound destination array, the
  call-with-destination form, the copy into the field, the call counter, the guarded fill's
  position (the `__cl_owned0` fill inside the evaluated path), the drop/re-fill order, and a
  re-emission determinism pin.
- **Verification**: `make check`; the focused suite plain and with `CMD="RDC_ASAN=1 cargo test
  --test conditional_projection_residue_spec_test --test
  conditional_projection_residue_impl_test"`; the strict builds; the hosted-amiga leg for
  `DEF-378`.
- **Risks/lenses**: all four (AR-27). The emission lens owns the destination form, the call
  count, and the base/head differential over `m378_*`/`m379_*` (AR-37); the memory lens owns the
  guard paths, the drop/re-fill analysis (`guards.rs:273-307`), and the leaks; the semantics lens
  owns the evaluation-order reading.
