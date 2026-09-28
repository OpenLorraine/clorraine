# Borrow Element Alias: Codegen Ownership Safety

> **Document**: 03-01-borrow-element-alias.md
> **Parent**: [Index](00-index.md)

## Overview

This document specifies the DEF-13 fix: a `for (borrow e in xs)` element
becomes a true alias of the iterable element for the whole iteration. The
binding keeps a compiler temporary holding the element address; reads and
writes go through the dereference, so the element is the single source of
truth and no copy-back step exists (AR #2, AR #7).

The specification requires this: element views write through to the owner
(`spec/16-borrow-rules.md` section 2, LOOP-2), and assignment drops the live
old value of the target before storing the new one (`spec/03-ownership.md` O3,
AR #6). Non-`borrow` element loops keep the per-iteration copy (AR #8).

## Architecture

### Current Architecture

`compiler/src/own/loops.rs` lowers each element loop to an index loop and
binds the element with a local copy:

- `bind_for_in_element` (`:294`) declares the binding as an ordinary local and
  emits `Assign` of the indexed element into it.
- `set_for_in_write` (`:338`) records `ForInWrite` on the innermost
  `ExitFrame`.
- `store_for_in_element` (`:365`) emits the write-back `StorePointer` on
  fallthrough, `break` (`compiler/src/own/statements.rs:523`), and `continue`
  (`:547`).

The emitter drops the cache's old value by type in
`compiler/src/codegen/function.rs` `emit_assign`, which frees the aliased
element at the top of the next iteration.

### Proposed Changes

Alias the borrow element through a typed compiler temporary:

1. Allocate `address = new_temp(Ty::raw(element))` once per loop.
2. Declare the element binding exactly as today, then override its place base
   with the new `PlaceBase::DerefTemp { temp: address, ty: element }`
   (AR #11). The existing `Binding.base` override already serves lambda
   captures.
3. Each iteration, emit `Assign` of `AddressOf(Index { iterable, index })` into
   the temporary. The address carries the bounds check as today.
4. Delete `ForInWrite`, `set_for_in_write`, `store_for_in_element`,
   `run_for_in_write`, `ExitFrame::for_in`, and the `break`/`continue` hooks.
5. Skip base-overridden local bindings when building `Function.locals`
   (`kind == BindingKind::Local && binding.base.is_none()`), so the alias never
   declares a spurious C local (R7). Captures are `BindingKind::Param` and are
   already excluded by the kind predicate.

Reads and writes of the element then render as `(*__cl_tmpN)`, and O3
replacement drops and stores through the dereference. No exit path needs extra
work: `return`, `break`, `continue`, fallthrough, and deferred cleanup all see
the element exactly as written.

## Implementation Details

### New Types

```rust
/// A place reached by dereferencing a compiler temporary that holds its
/// address.
///
/// A `for (borrow e in xs)` element is an alias of the iterable element: a
/// per-iteration temporary holds the element's address and every use of the
/// binding goes through it, so a write reaches the iterable immediately and
/// the assignment rules apply to the element itself.
DerefTemp {
    /// The temporary holding the element address.
    temp: TempId,
    /// The type of the value the address points at.
    ty: Ty,
},
```

`compiler/src/ir/place.rs`. The variant mirrors `PlaceBase::EnvField`, which
also carries its own type so consumers need no site lookup.

### New Functions and Methods

- `compiler/src/own/loops.rs` `bind_for_in_element`: for a `borrow` element,
  set the binding's `base` to `DerefTemp`, emit the per-iteration address
  assignment, and mark the binding used (preserving the side effect the
  deleted write-back had on the unused-local diagnostic); for a copied
  element, keep the current `Assign` into the local.
- `compiler/src/own/loops.rs` `check_for_in`: allocate the address temporary
  before both the speculative and the real binding pass, exactly where
  `index_temp` is allocated today, so both passes agree on its id.
- `compiler/src/codegen/expr.rs` `place_base`: render `(*__cl_tmp{temp})`.
- `compiler/src/codegen/expr.rs` `place_base_type`: return the carried `ty`.

### Integration Points

- `Binding.base` (`compiler/src/own/flow.rs:62`) already overrides the usual
  local or parameter spelling; `binding_place` returns it unchanged.
- `Function.locals` builds from `BindingKind::Local` bindings whose base is
  not overridden (`compiler/src/own.rs:303`, `compiler/src/own/lower.rs:382`);
  captures are `BindingKind::Param` and were never included.
- Compiler temporaries are declared, initialized, and marked deliberately used
  by `compiler/src/codegen/function.rs:248`.
- `Ty::raw(element)` renders `ClString *` and friends
  (`compiler/src/codegen/types.rs:26`, `:118`).

## Code Examples

### Example: emitted C after the fix

```c
/* for (borrow e in arr) { e = stringOf("c"); } */
for (; (__cl_tmp0 < 2); __cl_tmp0 = (__cl_tmp0 + 1u)) {
    __cl_tmp1 = &(arr[__cl_tmp0]);              /* fresh alias */
    if ((*__cl_tmp1)) { FreeString((*__cl_tmp1)); }   /* O3 replacement */
    (*__cl_tmp1) = cl_std_text_stringOf(cl_str("c"));
}
```

The scope cleanup still frees each array element once; the binding declares no
local of its own.

### Example: field write through the alias

```clorraine
struct P { name: String; }
let ps: P[2] = [P { name: stringOf("aa") }, P { name: stringOf("bb") }];
for (borrow p in ps) {
    p.name = stringOf("zz");
}
```

`p.name = ...` lowers to a place with the `Field` projection over the deref
base, so the old name is dropped and the new one stored in the element.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Run-time index out of range during the alias refresh | The existing bounds guard is emitted with the address assignment, unchanged | AR #2 |
| A `take`/move of the alias | The checker does not reject a whole move of the borrow element binding; the element is stolen. Observed and filed as a separate defect (`DEF-18`), unchanged by this plan | AR #1 |
| A non-`borrow` element loop over a non-duplicable type | Unchanged: E10040 requires `borrow` | AR #8 |

> **Traceability:** Every error-handling strategy and design choice must
> reference the Ambiguity Register entry (AR #) that resolved it. See
> `00-ambiguity-register.md`.

## Testing Requirements

- Specification cases ST-1 – ST-7 in `07-testing-strategy.md`.
- Implementation tests: a slice iterable (via `import { slice } from std.mem`),
  replacement read-back within the same iteration, a sanitized-name collision
  and a keyword-named shadow, a structural check that no spurious local or
  write-back remains, and a build-only freestanding compile of a
  borrow-element program. Nested element loops are out of scope (a separate
  step-selection defect is filed as `DEF-17`).
- Full behavioral regression: `make check`, then `make verify`.
