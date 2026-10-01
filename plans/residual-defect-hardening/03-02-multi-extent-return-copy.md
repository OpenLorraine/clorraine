# Multi-Extent Return Copy: Residual Defect Hardening

> **Document**: 03-02-multi-extent-return-copy.md
> **Parent**: [Index](00-index.md)

## Overview

This component fixes DEF-47: returning a fixed-array place of more than one extent copies only the
first extent, emitting an array assignment that C rejects. The fix delegates the copy to the
existing fixed-array store core, which already handles multiple extents, owned sources, and the
`needs_drop` gate. Per AR #12 the hand-written loop is removed rather than extended.

## Architecture

### Current Architecture

`emit_fixed_array_return_arm` (`compiler/src/codegen/cleanup.rs:516-602`) handles a return value by
kind. The literal branch already builds the hidden destination as
`ArrayTarget::parameter("__cl_ret", bytes)` and delegates to `emit_array_assign_to`
(`:532-537`). The place branch (`:539-555`) instead loops over the first extent and emits
`__cl_ret[i] = source[i];`, then empties the source when the substituted source type needs a drop.

The statement store core `emit_fixed_array_assign_to` (`compiler/src/codegen/function.rs:970-1120`)
handles a whole-array value: for a plain place it memcpys `target.bytes` from the source address,
drops the previous target value only when the type needs a drop and the value does not read the
target, and empties the source through `emit_arm_source_moves` (defined `:1723-1752`, called at
`:1093` and `:1117`). The core is currently private to the codegen `function` module; the sibling
`cleanup` module cannot call it.

### Proposed Changes

1. Promote `emit_fixed_array_assign_to` to sibling visibility (`pub(super)`) so the return module
   can delegate, matching the already-visible `emit_array_assign_to`.
2. In the place branch, build the same destination the literal branch uses and call the core once
   with `reads_target = false`. Delete the per-extent loop and the manual `emit_move_zero_place`;
   the core performs the byte copy and the source emptying, gated on `needs_drop(ty)`.

The caller's destination is the hidden out-parameter. Every caller-side destination is a hoisted
local or a temporary initialized to the empty state (`codegen/function.rs:1018-1026`,
`:1785-1792`, `:498-506`, `:1227-1259`; `codegen/guards.rs:141-147`, `:561`), so the core's drop
of the target is a null-safe no-op and cannot free a live value. This is the behavior the literal
branch already relies on (`codegen/expr.rs:412-446` only renders the call-with-destination).

### Behavior After the Change

For `func copy(take a: String[2][2]): String[2][2] { return a; }` the arm emits one byte copy of
the whole array into `__cl_ret` and then empties `a`, so the caller owns the only reference and
the callee's scope cleanup releases nothing. For `i32[2][2]` no source emptying is emitted,
exactly as today. A conditional whose chosen arm is a multi-extent place keeps its branch form and
recurses into this arm.

## Implementation Details

### Signature and visibility

```
pub(super) fn emit_fixed_array_assign_to(
    &mut self,
    target: &ArrayTarget,
    ty: &Ty,
    value: &Value,
    reads_target: bool,
)
```

No signature change; only the visibility marker is added.

### Return arm

```
ValueKind::Place(place) => {
    let destination = ArrayTarget::parameter("__cl_ret", self.array_param_bytes("__cl_ret", &ty));
    self.emit_fixed_array_assign_to(&destination, &ty, value, false);
}
```

`emit_guards(value)` is no longer needed separately: the core emits the value's guards on the
copy path.

### Integration Points

- `compiler/src/codegen/function.rs:970` — visibility.
- `compiler/src/codegen/cleanup.rs:539-555` — the delegated arm.
- `compiler/src/codegen/function.rs:2029-2065` — `ArrayTarget` (used already by the literal
  branch).

## Code Examples

```clorraine
func copy(take a: String[2][2]): String[2][2] {
    return a;
}
```

Emitted arm (before: `__cl_ret[0] = a[0]; __cl_ret[1] = a[1];`):

```c
memcpy(__cl_ret, a, sizeof(a));
/* the core then empties a: a[0][0] = NULL; ... */
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Source needs a drop | The core empties the source after the copy, once | AR #12 |
| Source is duplicable | No source emptying; the copy is a plain byte copy | AR #12 |
| Destination reads the value | Not reachable for a return arm (the destination is the hidden out-parameter, never the source); `reads_target = false` is fixed | AR #12 |
| Conditional / call return arm | Unchanged: those arms keep their existing forms and recurse | AR #12 |

## Testing Requirements

- Spec cases ST-6 – ST-7 (`07-testing-strategy.md`).
- Impl tests for the emitted `memcpy` shape and the absence of a per-extent array assignment.
- The close-out ASan probe covers the owned multi-extent return (no leak, no double free).
