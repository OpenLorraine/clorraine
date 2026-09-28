# Element Read Ownership: Element Loop Integrity

> **Document**: 03-01-element-read-ownership.md
> **Parent**: [Index](00-index.md)

## Overview

This component fixes DEF-16: an owned sub-object read (an array element, a field
of one, or a field reached through a borrow struct) passed as a non-`take`
argument must be a lend or view, not a second owner. The fix is one read predicate
plus one guard change in the ownership stage; the expression lowering, the C
rendering, and the bounds guards are already correct.

> **Decision per AR #6:** the narrow materialization guard; the element read path
> itself is not changed.

## Architecture

### Current Architecture

`materialize_owned_argument` (`compiler/src/own/lower.rs:659`) decides whether an
argument needs an owned scope local. Its guard list skips `take` arguments,
closures, duplicable values, views, `ValueKind::Place`, speculative passes, unknown
types, and type-parameter mentions. Anything else is bound to `__cl_ownedN` in the
function body scope and released by the scope cleanup. Call arguments
(`own/call.rs:175`) and template placeholders (`own/lower.rs:211`) both use it.

The stage lowers member reads over a name to `ValueKind::Place` (the qualified-name
path, `own/lower.rs:30-33`), but element reads always become `ValueKind::Index`
(`own/lower.rs:38-51`). An `Index` value is not a `Place`, so it materializes even
though the container owns the element.

### Proposed Changes

Add a read-only predicate beside `value_place` and use it in the materialization
guard:

```rust
/// True when the value reads storage an owned container is responsible for:
/// a place, or a field or element projection of one at any index.
///
/// A `for (borrow e in xs)` element is a `Place` rooted at its address
/// temporary, so it is covered. An ownerless temporary over a container (for
/// example a call returning an array) is not: no container owns its element.
fn projects_from_place(&self, value: &Value) -> bool {
    match &value.kind {
        ValueKind::Place(_) => true,
        ValueKind::Member { base, .. } | ValueKind::Index { base, .. } => {
            self.projects_from_place(base)
        }
        _ => false,
    }
}
```

In `materialize_owned_argument`, replace the `ValueKind::Place` match with the
predicate:

```rust
|| self.projects_from_place(&value)
```

> **Decision per AR #12:** the predicate is named `projects_from_place`.

`value_place` is intentionally not extended: it feeds `consume_place`, which
performs moves into aggregate literals, and element moves are rejected earlier
(`E10026`). Keeping the two helpers separate keeps the move path unchanged
(AR #6, R3).

## Implementation Details

### Behavior Matrix

| Argument form | Value kind | Materialized? | Owner / lease |
| --- | --- | --- | --- |
| `arr[0]`, `arr[i]` over a local | `Index(Place)` | No | The array owns; the callee lends |
| `ps[0].name` | `Member(Index(Place))` | No | The array element owns |
| `b.xs[0]` (`borrow b: Bag`) | `Index(Member(Place))` | No | The caller's struct owns |
| `e` from `for (borrow e in xs)` | `Place(DerefTemp)` | No | The array owns |
| `xs[0]` over `borrow xs: String[2]` | `Index(Place(Param))` | No | The caller's owner |
| `p.name` over a local | `Place(Local, Field)` | No (already) | The struct owns |
| `stringOf("hi")`, `bytes(stringOf("hi"))` | `Call` | Yes | The scope temporary owns and releases once |
| `[stringOf("a")][0]` | `Index(ArrayLit)` | Yes (unchanged) | No container; today's path stays |
| `mk()[0]` (array-returning call) | `Index(Call)` | Yes (unchanged) | Invalid C today; DEF-21 (AR #4) |
| `take(arr[0])` | `Index(Place)` | Skipped as `take`; sema rejects `E10026` | Move, not a lend (R3) |

### Integration Points

- `own/call.rs:175` — call arguments: unchanged call, new guard behavior.
- `own/lower.rs:211` — template placeholders: same guard; an owned placeholder no
  longer double-frees (ST-8).
- `codegen/calls.rs:119` `argument_text` — already widens an owned `String`
  argument to `cl_string_view(...)`; with no temporary, the view is taken directly
  from the element expression.
- Guard emission is unchanged: on the assignment path `emit_assign`
  (`codegen/function.rs:577`) and on the return path `emit_return`
  (`codegen/cleanup.rs:264`) emit the value's guards (including the `E11211`
  bounds check) before the statement, so a runtime-index read inside a call
  argument is still bounded (ST-2, ST-13).

### Emitted C Before and After

```c
/* before: two owners */
if (__cl_owned0) { FreeString(__cl_owned0); }
__cl_owned0 = NULL;
if ((0 < 0) || ((usize)(0) >= (usize)(2))) cl_trap("E11211", "index out of range");
__cl_owned0 = arr[0];
__cl_ret = cl_tests_cos_read1_vlen(cl_string_view(__cl_owned0));
/* ... scope cleanup frees arr[0] and __cl_owned0 -> double free */

/* after: the array is the only owner */
if ((0 < 0) || ((usize)(0) >= (usize)(2))) cl_trap("E11211", "index out of range");
__cl_ret = cl_tests_cos_read1_vlen(cl_string_view(arr[0]));
/* scope cleanup frees arr[0] once */
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| `take(arr[0])`, `let a: String = arr[0]` | Unchanged `E10026` from the existing partial-move rule; not touched by this component | AR #2 |
| `vlen(arr[i])` with `i` out of range | Unchanged runtime `E11211` trap from the emitted guard (ST-13) | AR #13 |
| Ownerless temporary argument | Unchanged materialization and single release | AR #6 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- Specification cases ST-1 – ST-13 in `07-testing-strategy.md` (run oracle for the
  values and the trap; compile-fail oracle for `E10026`).
- Implementation cases in `element_loop_integrity_impl_test.rs`: emitted-C shape
  (no `__cl_owned` binding for an element argument; the owned-temporary path still
  binds and releases), the template placeholder shape, and determinism.
