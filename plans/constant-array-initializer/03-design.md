# Design: Constant Array Initialization (PLAN-25)

> **Document**: 03-design.md
> **Parent**: [Index](00-index.md)

## Overview

The design adds one local emission rule to the module-level global initializer
path. When a module-level `let` fixed array has a complete explicit list of
guard-free constant scalar elements, the emitter writes:

1. one block-scope `static const` template inside the same
   `cl_init_<module>` function, and
2. one `memcpy(<global>, <template>, sizeof(<global>));`

instead of one store per element. The global declaration, its writable storage,
its address, its type/size, and the position of the initialization remain
unchanged. All other shapes keep the current spelling.

The measured `lookup` probe (4,096-entry `i32` table) is the acceptance target:
`.text` 117,440 → 92,976 and raw instructions 17,025 → 12,842, with equal
output. See [Current State](02-current-state.md) for the measurement record.

## Architecture

### Current path

| Stage | Code |
|---|---|
| Global declarations | `emit_globals` `compiler/src/codegen/program.rs:1134-1160` |
| Init function order | `emit_init_functions` `program.rs:1163-1193`; `global_init_order` `compiler/src/codegen/init.rs:69-120` |
| One global's initializer | `emit_global_init` `program.rs:1244-1290` |
| Array-literal branch | `emit_global_array_fill` `program.rs:1842-1864` |
| Per-element stores | `emit_filled_array` `program.rs:1926-1952` |
| Constant evaluator | `constant_integer` `compiler/src/codegen/expr.rs:2036-2124` |
| Typed renderer | `store_text` `compiler/src/codegen/function.rs:3984` |
| Volatile flag | `GlobalInfo.volatile` `compiler/src/codegen.rs:49-63` |
| Profile | `Profile` `compiler/src/profile.rs:35-60` |

### Proposed path

```text
emit_global_init(module, name, value)
  → lower the initializer
  → if declared is a fixed array:
        if emit_constant_array_template(target, declared, lowered, volatile):
            return
        else if fills_element_wise(lowered):
            emit_global_array_fill(...)
  → existing scalar/other path
```

The helper returns `true` only when it emitted the template and `memcpy`; a
`false` result leaves every existing path untouched.

## Eligibility

The helper accepts a value only when **all** of the following hold. The
predicate is deliberately narrower than the general constant evaluator.

### Binding and type

- The binding is a module-level `let`. A module-level `const` array with an
  explicit list is rejected by the front end (`E10224`) and never reaches
  codegen.
- The declared type is `TyKind::Array` with exactly one extent; the extent is
  `Some(n)`, `n > 0`.
- The leaf element type is one of `i8`, `i16`, `i32`, `u8`, `u16`, `u32`,
  `usize`, `isize`, or `bool`. The check is an explicit list, not
  `is_integer_scalar` (which excludes `bool`).
- `n >= 16` and `n * size_of(element) >= 64` under the checked type table
  (`self.sema.table`; `usize`/`isize` are 32-bit typedefs on every profile),
  using `checked_mul`; an overflow or a smaller table falls back. Phase 1
  measures the small-table boundary and finalizes this lower bound before
  Phase 2 sizes the tests.
- The binding is not `volatile` (`GlobalInfo.volatile == false`).
- The profile is `Host`, `HostedAmiga`, or `HostedAmiga13`. `Freestanding`
  falls back because its runtime `memcpy` is a byte loop and its performance
  is not measured.

### Value shape

- The lowered value is a direct `ValueKind::ArrayLit { elements, fill }` with
  `fill == None` and `elements.len() == n`. A `Materialize` wrapper, a place, a
  call, or a conditional falls back.
- Every element satisfies `guard_free_constant`:

```text
guard_free_constant(value):
  Int | Bool | Char                                  → true
  Cast { value: inner, .. } where both the cast target
    and inner.ty are non-float scalars               → guard_free_constant(inner)
  Unary { op: Neg | BitNot | Not, operand }          → guard_free_constant(operand)
  everything else                                    → false
```

  This excludes every `Binary` (so `MIN / -1`, `MIN % -1`, shifts, and logical
  short-circuit forms), every float-to-integer cast, every conditional, every
  `Materialize`, every call, every place, and every intrinsic. The current
  emitter attaches runtime guards or unevaluated operands to those forms; the
  template path never sees them. A cast between non-float scalars includes a
  `bool` target; the value is still checked by `constant_integer`.
- `constant_integer(element)` returns `Some(value)` for every element. It is
  only the width-correct eligibility/value gate; the template does **not**
  re-render from the folded integer.
- The rendered element text is always `store_text(element, &element_ty)`, so
  negative values, narrow casts, and unsigned maxima keep the exact spelling
  the current stores use.

## Emission

### Template name

Use an explicit per-init-function counter, not `unique_local` (which returns
`__cl_ctmpl` first and only appends `_1` on collision). Add an
`init_templates: usize` field to the emitter, reset it in
`emit_init_functions`, and generate `__cl_ctmpl_<n>` (first
`__cl_ctmpl_0`, then `__cl_ctmpl_1`). The template is block-scope, so the same
name may repeat in a different init function; do not reserve it in the global
`used_names`. The reserved `__cl_` prefix and the distinct `__cl_itmp`/global
spellings make a collision with a user symbol or init local impossible. A
debug assertion checks the candidate is not already declared in the current
init function. The exact spelling is pinned as
`__cl_ctmpl_0`/`__cl_ctmpl_1` in the tests.

### Template text

```c
    static const <element-declarator> __cl_ctmpl_0[<n>] = {
        <v0>, <v1>, <v2>, <v3>, <v4>, <v5>, <v6>, <v7>,
        ...
    };
    memcpy(<target>, __cl_ctmpl_0, sizeof(<target>));
```

- The declarator is built with the existing `declarator(element, name,
  &self.sema.table)` and prefixed with `static const `.
- The initializer is written eight values per line. This keeps the template
  under C99’s guaranteed 4095-character logical-line limit and makes the diff
  reviewable.
- A trailing comma on each row is valid C99.
- The `memcpy` size is `sizeof(<target>)`; the template is extent-sized, so
  `sizeof(template) == sizeof(target)` when the element types match. The
  extent-sized declarator is also a deliberate guard against a future short
  list: the C compiler would zero-fill the tail rather than over-read.
- The template is block-scope, so it has static storage duration, is
  initialized once at program load, and is not visible outside the init
  function.

### Position and effects

The helper runs in `emit_global_init` before `emit_global_array_fill`. The
`#line` directive is already emitted, and the `memcpy` appears exactly where
the first store would have appeared. No guard, owner, drop, or init-local is
created for a constant scalar array. The global declaration in `emit_globals`
is unchanged, so the target remains writable and keeps its empty state until
the `memcpy`.

### Volatile and profile checks

`emit_global_init` already looks up the declared type by module/name. Extend
that lookup to read `GlobalInfo.volatile` and pass it to the helper. The helper
also checks `self.profile`. This keeps the volatile semantics out of
`emit_global_array_fill`, whose signature has no flag.

## Error Handling and Fallbacks

| Case | Handling | AR Ref |
|---|---|---|
| Short/empty explicit list | Fall back to `emit_global_array_fill`; the current behavior leaves the omitted tail at the zero state | AR-04 |
| Fill form or prefix | Fall back to the existing fill compaction | AR-04 |
| Multi-extent array | Fall back to the existing per-row path | AR-04 |
| Volatile binding | Fall back; never pass a volatile target to `memcpy` | AR-04 |
| Float, struct, handle, owned, view element | Fall back; the predicate rejects it | AR-04 |
| Guard-bearing element | Fall back; the guard-free predicate rejects it | AR-04 |
| Module `const` explicit list | Front end rejects `E10224`; no codegen path | AR-04 |
| Zero extent | Front end rejects `E10226`; no codegen path | AR-04 |
| Slice or handle/owned array | Front end rejects `E10200`/`E11106`; no codegen path | AR-04 |
| Freestanding multi-byte table | Fall back to the stores | AR-04 |
| Template name collision | Per-init-function `__cl_ctmpl_<n>` counter, reserved prefix, debug assertion | AR-03 |
| Large table line | Eight values per line | AR-03 |
| Overflow in the threshold product | `checked_mul` fails; fall back | AR-04 |

No new diagnostic code is introduced. The front-end diagnostics are existing
behavior and are only used as test cases.

## Complexity and Limits

- The change is one predicate, one helper, and one call site in the existing
  program emitter. It adds no pass, IR node, crate, build script, harness
  feature, or configuration.
- The template duplicates the table’s bytes in read-only storage while the
  writable global remains in `.bss`/`.data`. The total binary is still smaller
  than the current store run for the measured table, and the scored `.text`
  and instructions improve. This is recorded as an accepted limit.
- The lower bound is provisional; Phase 1 measures the small-table boundary
  and finalizes it. The 4,096-entry benchmark is far above it.
- Function-local arrays, fills, short lists, and freestanding multi-byte
  tables are explicit residual/out-of-scope items, not silent omissions.

## Testing Requirements

The specification and emission cases live in
[Testing Strategy](07-testing-strategy.md). The implementation must:

- keep the host behaviour suite green; `hosted-amiga` execution is covered by
  the benchmark/corpus gates; `hosted-amiga-13` is compile-checked by
  `bench-shape` (the A500 runtime gates do not use an eligible table), and
  `freestanding` keeps the old stores;
- pin the template, values, `memcpy`, target declaration, position, wrapping,
  `__cl_ctmpl_<n>` name uniqueness, profile gate, and every exclusion in the
  emission suite;
- prove the red phase against the pre-change compiler;
- run ASan/UBSan on the behaviour suite;
- pass `make bench-shape` on hosted-amiga (m68k GCC), hosted-amiga-13
  (nix13), vbcc, and freestanding.

## Traceability

| Decision | AR |
|---|---|
| Measured shape and benchmark | AR-01 |
| v1.1.0 base and isolated comparison | AR-02 |
| Template + `memcpy` mechanism | AR-03 |
| Eligibility, threshold, profiles, exclusions | AR-04 |
| Order, storage, and mutability | AR-05 |
| Tests and red-first | AR-06 |
| Requirement values | AR-07 |
| No harness/linker/shim change | AR-08 |
| Documentation and roadmap | AR-09 |
| Review routing | AR-10 |
