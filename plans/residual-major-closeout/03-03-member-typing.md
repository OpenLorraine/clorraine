# Member Typing: Enum Value Form and View Members: Residual Major Closeout

> **Document**: 03-03-member-typing.md
> **Parent**: [Index](00-index.md)

## Overview

Two batch G member-surface rows: the value-form enum member read (`DEF-233`) and the untyped
view members `.ptr`/`.len` (`DEF-238`). Both are member-dispatch gaps: the checker accepts a
spelling whose C form is invalid because the member has no checked surface. `DEF-233` resolves
per AR-6 (reject with the registered `E10212` on both the concrete and the instance paths);
`DEF-238` types the members so the conversions judge. The machinery is the member dispatch
(`sema/expr.rs::member_type`), the instance audit (`own/instance_check.rs::projection_resolves`),
and the pointer-member conversion (`codegen/function.rs`). No new diagnostic code, no message
text change (Ch 14 owns the text).

## Design

### 1. The value-form enum member read (`DEF-233`)

`let e: E = E.A; let n = i32(e.A);` is accepted and the strict build fails
(`request for member 'A' in something not a structure or union`); the instance path
`read<T>(x: T) { i32(x.A) }` at `T = E` mirrors the acceptance. Ch 04 §8 spells the enum member
form `Type.MEMBER`; a value receiver has no member surface (AR-6).

Reject with `E10212` at both paths:
- The concrete path: the enum arm of `member_type` (`sema/expr.rs:569-579`) currently checks
  membership and returns the enum type; it reports the registered unknown-member diagnostic for
  a value receiver, while the scoped form (handled in `check_qualified`'s
  `is_enum(first)` branch, `:485-498`) and the qualified `A.B.f` forms stay accepted. The
  report names the enum type and the help line teaches the scoped spelling.
- The instance path: `projection_resolves` (`own/instance_check.rs:1000-1011`) drops the enum
  arm, so the opaque projection's substituted enum reports the same diagnostic at the audit's
  span (`:946-991`), one report per site, matching the concrete span behavior.

The concrete and instance paths report together (the row: "apply it to both the concrete and
the instance paths"); the instance body's own C emission disappears because the program is
rejected before codegen.

### 2. The view members `.ptr` and `.len` (`DEF-238`)

`func f(s: str): usize { return s.ptr; }` is accepted (the member falls through
`member_type`'s `_ => Ty::unknown()`, `sema/expr.rs:598`) and emits `return s.ptr;` where the
return type is a scalar — the strict build fails `-Werror=int-conversion`. The store conversion
compensates for a `cstring` destination only, by recognizing the member's *name*
(`codegen/function.rs:2046-2078`, `names_pointer_member` `:2647-2659`), because the member
carries no checked type.

Type the members in the dispatch:
- `.ptr` carries the view's pointer — the frozen ABI's pointer member (Ch 17: `str` is
  `{ const u8 *ptr; usize len }`); in the type system the view's raw pointer to its element
  (`raw u8` for the byte views, the element pointer for slices). The conversions then judge per
  the frozen table: an integer destination (`usize`) reports the frozen `E10200`, so the red
  spelling stops failing the C build.
- `.len` is `usize` (a fixed array's structural `.len` keeps its compile-time fold).
- The `cstring` destination keeps its pointer-member conversion (the pinned `(char *)(s.ptr)`
  shape): the conversion keys on the typed member instead of the name heuristic, so
  `toCString`'s body and the pinned `checker_conformance` cases stay accepted. The
  `own/expr.rs::member_type` mirror (`:784`) gains the view arms so ownership projections and
  the audit agree.

The stdlib's explicit `raw u8(s.ptr)` casts stay valid; the `toCString` body keeps its
conversion through the typed pointer-member path.

## Integration Points

- `sema/expr.rs:553-599` (`member_type`), `:485-498` (the scoped enum path) — the enum and
  view member typing.
- `own/expr.rs:784-792` (`member_type`) — the ownership mirror.
- `own/instance_check.rs:946-1011` (the audit and `projection_resolves`) — the instance
  reports.
- `codegen/function.rs:2046-2078` (`store_text`) and `:2647-2659`
  (`names_pointer_member`) — the pointer-member conversion keyed on the typed member.
- `codegen/expr.rs:35-58` (the member renderer) — the `.ptr`/`.len` C spellings (already the
  view struct's fields; unchanged).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A value-form enum member read | One `E10212` per site on the concrete and instance paths; the scoped form accepted | AR-6 |
| An enum member name that does not exist on the scoped form | The existing `E10212` (unchanged) | — |
| A view pointer member at an integer destination | The frozen `E10200` conversion report | — |
| A view pointer member at a `cstring` destination | The existing pointer-member conversion (typed) | — |
| A `.len` on a fixed array | The structural fold (unchanged) | — |
| A `.ptr`/`.len` on a non-view base | The existing unknown-member/`E10212` path | — |

## Testing Requirements

- Spec: the concrete and instance value-form enum reads (rejected once with `E10212`; the
  scoped `E.A` and `Type.MEMBER` forms accepted controls), the `.ptr` into `usize` (rejected
  with the frozen conversion code), the `.len` twin (accepted, runs), and the `toCString`
  acceptance (the stdlib body and the pinned caller cases stay green).
- Impl: the enum rejection span on both paths, the typed member's conversion emission
  (`(char *)(s.ptr)` or its typed successor), the scalar/struct member controls byte-identical,
  and a re-emission determinism pin.
