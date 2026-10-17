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
(`static void cl_n1(ClStr __cl_ret[2])`). The literal builder routes a view-array field to the
element-wise store only when an element needs a binding
(`own/lower.rs:4536` `literal_fields_need_stores`, `:4563` `literal_field_needs_stores`), and
`view_element_needs_binding` (`:4592`) returns false for a call whose type is already a view, so
the call keeps the value form. The guarded positions were fixed in batch L; the read, unguarded,
nested, and grid-element positions still fail. Frozen: Ch 04 §3 (`spec/04-expressions.md:53`)
fixes function-call and array-literal evaluation order; Ch 04 §15 makes the literal elements
typed by the expected type; Ch 17 §5 fixes the move shape. The hidden destination is an internal
emission mechanism; the frozen requirement is that the accepted literal builds and evaluates each
call once with the literal's value.

Fix: treat a call-valued view-array element as needing the binding store whenever the call's
return uses the hidden destination (a fixed view-array return), at **every** literal position —
the read, unguarded, nested, and guarded builders. The existing element-wise store path binds the
destination, calls the function into it, and copies the field; the change is the predicate and
the read-position route, not a new ABI. Expected: `readpos`, `gridel`, and `guardrowcond` build
strictly and run 0; each call runs once; the concrete non-call view-array literal control stays
byte-identical.

### 2. The owning nested conditional under a guard (`DEF-379`)

`g && consume(ok ? Outer { inner: Inner { names: n1() }, tag: stringOf("x") } : Outer { ... })`
is accepted and runs 1 (expected 0): the emitted C fills the arm carrier `__cl_owned0` before
`if (g && consume(__cl_owned0) == 1)`, so the skipped `g == false` path still runs one `n1`/`n2`
call (calls = 3, expected 2). The owning literal needs stores, so `materialize_guard_literal`
(`own/lower.rs:1697`) materializes at statement entry. Frozen: Ch 04 §3 (`:53`, the `&&`/`||`
row: "Left operand first; the right operand only when the result is not yet fixed"); Ch 04 §5
(`:108`: "only the chosen branch evaluates").

Fix: keep the guarded literal's fill in the guard's evaluated path: the carrier is filled at the
position where the `&&` right operand evaluates, not at statement entry, using the existing
guarded-literal/deferred-move-out carrier. On `g == false` the fill and the call run nothing; on
`g == true` the fill runs once and the call reads the filled carrier. Expected: calls = 2, run 0;
ASan/LSan-clean; both selections covered. The eager (unguarded) literal shapes keep their current
order.

## Integration Points

- `compiler/src/own/lower.rs:4536` (`literal_fields_need_stores`), `:4563`
  (`literal_field_needs_stores`), `:4592` (`view_element_needs_binding`), the literal builder and
  the element-wise store path; `:1697` (`materialize_guard_literal`), the `&&`/`||` right-operand
  lowering (`:228-236`) and the guard statement carrier.
- The batch-L phase-5 guarded view-literal machinery and the `DEF-323` cases.
- Controls: `probes/m378_readpos.clo`, `m378_gridel.clo`, `m378_guardrowcond.clo`,
  `probes/m379_guardnest.clo`; the batch-L literal/store origin pins.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A call-valued view-array field at a read/unguarded position | Bind the hidden destination; call once; copy the field | AR-22 |
| The same field in a guarded position | The existing guarded fill; no regression | AR-22 |
| A brace-list view-array literal with complete views | The current form (control); no unnecessary binding | AR-22 |
| An owning nested conditional under `&&` | Fill the carrier inside the evaluated guard path | AR-23 |
| The skipped guard path | Nothing runs (no call, no fill) | AR-23 |
| A nested conditional not under a guard | The current eager order (control) | AR-23 |

## Testing Requirements

- **Spec cases** (`conditional_projection_residue_spec_test.rs`):
  - `should_materialize_a_hidden_view_destination_at_every_literal_position` (`DEF-378`): the
    read, grid-element, and guarded-row-condition shapes build and run 0; each call runs once;
    the concrete brace-list control is unchanged.
  - `should_keep_an_owning_nested_conditional_inside_its_guard` (`DEF-379`): both selections
    build and run 0; calls = 2; ASan/LSan-clean.
- **Impl pins** (`conditional_projection_residue_impl_test.rs`): the bound destination array, the
  call-with-destination form, the copy into the field, the call counter, the guarded fill's
  position, and a re-emission determinism pin.
- **Verification**: `make check`; the focused suite plain and under `RDC_ASAN=1`; the strict
  builds; the hosted-amiga leg for `DEF-378`.
- **Risks/lenses**: all four (AR-27). The emission lens owns the destination form and the call
  count; the memory lens owns the guard paths and the leaks; the semantics lens owns the
  evaluation-order reading.
