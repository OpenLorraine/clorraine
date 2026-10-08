# Element-Store Drop Spelling: Slice and Temporary Integrity

> **Document**: 03-04-element-store-drop-spelling.md
> **Parent**: [Index](00-index.md)
> **Defects**: DEF-75
> **AR**: AR-8, AR-10

## Overview

The owned branch of the call-indexed element store releases the replaced element with a drop
expression whose base is the dereferenced pointer (`*({pointer_c})`). When the element is a struct
with owning fields, `emit_struct_drop` appends `.field` to that base
(`compiler/src/codegen/drop.rs:86`), producing `*(&(out[i])).a` — parsed as `*((&(out[i])).a)` —
and the strict host build rejects it. The same precedence hazard exists in the fixed-array drop
recursion's index append.

## Architecture

### Current Architecture

- `Statement::StorePointer`'s owned branch
  (`compiler/src/codegen/function.rs:496-512`) evaluates the value, stores it in a fresh local,
  releases the replaced element, and moves the local into the element. The release call is
  `emit_drop_expr(&format!("*({pointer_c})"), &pointer_ty)` (`:509`).
- `emit_struct_drop` recurses with `format!("{c_name}.{field}")` (`compiler/src/codegen/drop.rs:86`);
  `emit_fixed_array_drop` recurses with an index suffix. A base that contains a unary `*` or an
  address-of loses its grouping when a suffix is appended.
- The sibling array-target branch already parents the dereference explicitly
  (`(*({pointer_c}))`, `compiler/src/codegen/function.rs:473`), and the handle case (no suffix)
  currently emits `*(&(rows[i]))`, which is valid and pinned by
  `compiler/tests/call_value_integrity_impl_test.rs:892-919`.

### Proposed Changes

Parenthesize a suffix base **only when it is not a primary/postfix expression**. The only such base
on the filed path is the dereferenced store target `*({pointer_c})` from the owned `StorePointer`
branch (`compiler/src/codegen/function.rs:509`); with grouping, the struct target reads
`(*(&(out[i]))).a` and nested suffixes compose correctly (`((*(&(out[i]))).a).b`). Identifier,
member, index, and call bases keep today's byte-identical spelling, so the existing pins and the
drop specification tests stay untouched (preflight PF-005).

The same conditional grouping applies to the expression-form builders `fixed_array_drop_expr` and
`struct_drop_expr` (`compiler/src/codegen/drop.rs:178-222`), whose callers pass temporaries today
but which share the suffix hazard (preflight PF-015).

## Implementation Details

### Integration Points

- `emit_drop_expr` callers that pass an already-parenthesized target are unaffected: the extra
  grouping is harmless (`((x)).a`).
- The fixed-array store branch (`compiler/src/codegen/function.rs:468-475`) is not on this path;
  only the owned drop path changes.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Struct element with owning fields | The dereferenced base gets grouping only when needed; the strict host build accepts the unit (preflight PF-005) | AR-8 |
| Handle element (no suffix) | Unchanged spelling; existing pins stay green | AR-8 |
| Nested struct/array suffixes | The conditional grouping composes; primary bases keep the old spelling | AR-10 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity
> Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification test ST-12 (`07-testing-strategy.md`): both filed programs build under the strict
  host compiler and store/run the correct value.
- Implementation tests: the emitted `(*(&(out[i]))).a` shape; the untouched handle spelling; the
  untouched fixed-array target spellings; **byte-identical existing spellings** for
  identifier/member/index bases; the expression-form grouping; byte-identical re-emission for
  programs without this shape.
