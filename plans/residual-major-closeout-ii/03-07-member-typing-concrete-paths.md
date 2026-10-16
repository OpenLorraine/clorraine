# Member Typing on Concrete Paths: Residual Major Closeout II

> **Document**: 03-07-member-typing-concrete-paths.md
> **Parent**: [Index](00-index.md)

> **Superseded (AR-22):** the `raw E` member read is the value form of its enum — every read reports
> the frozen `E10212` (existing and unknown members alike); the resolve-through-the-pointee route below
> is not conformant. The register records the ruling.

## Overview

Three batch-K phase-3 residue rows are member-surface gaps on the *concrete* path: an unknown
member on a fixed array or a view silently falls through `Ty::unknown()` and the strict build
fails; a view `.ptr` read at a mutable raw destination emits a const-qualified pointer that the
strict build rejects; and a `raw E` member read routes through the raw-field arm, so the
concrete and instance paths disagree. The fixes type the members and judge the conversions
inside the frozen member table and conversion rules.

## Design

### 1. The unknown member on a concrete base (`DEF-346`)

`let p = a.ptr;` on `u8[4]` and `let x = s.foo;` on a `str` are accepted and fail the strict
build (`request for member 'ptr' …` / `'ClStr' has no member named 'foo'`). The member dispatch
(`sema/expr.rs:748`, the `_` fallthrough of the dispatch at :692) falls every unhandled base to
`Ty::unknown()`; `own/expr.rs` mirrors it.
Ch 04 §8 requires `E10212` for an unknown member.

Report `E10212` for the unknown-member fallthrough on every concrete base (fixed arrays, views,
`cstring`, and the sibling fallbacks the phase-2 review's P-4 extended: the non-view `.ptr`
forms). The typed members (`.ptr`/`.len` on views and slices) and the scoped/qualified forms are
the controls; the instance audit mirrors the rule.

### 2. The view `.ptr` at a mutable raw destination (`DEF-347`)

`let p: raw u8 = s.ptr;`, a `raw u8` return, and `Holder { c: s.ptr }` are accepted and fail the
strict build (`discarded-qualifiers`/`pointer-sign`; `ClStr.ptr` is `const u8 *`). The sanctioned
spelling is the explicit `raw u8(s.ptr)` inside `unsafe` (`ctl347` builds and runs 0). Batch K
phase 3 scoped the `.ptr` member conversion to the store slots and the returns for `cstring`.

Emit the sanctioned member cast (`(u8 *)(s.ptr)`) at a mutable raw destination (store, return,
struct field) — the same member-conversion shape the `cstring` slots spell — keeping the
`unsafe` gate. The AR-7 fallback: if the semantics review rules the const-drop conversion
illegal, the destination reports the frozen `E10200` instead. The cast spelling and the
`cstring` slots are the controls; the immutable-view destination stays read-only.

### 3. The `raw E` member read (`DEF-348`)

`i32(p.A)` on a `raw E` is accepted and fails the strict build (`request for member 'A' …`); the
concrete path and the instance path disagree. Ch 04 §8's enum member form is `Type.MEMBER`.

Route the raw-enum member read through the enum rule: the member resolves the enum constant
through the pointee type (or the spelling reports the frozen `E10212` where the enum has no such
member), so the concrete and instantiated paths agree. The scoped `E.A`, the qualified
`A.B.f`, and the value-form enum rejection (batch K phase 3) are the controls.

## Integration Points

- `compiler/src/sema/expr.rs:748` (the member fallthrough; the dispatch at :692), the enum/raw
  member arms (:711-730),
  `compiler/src/own/expr.rs` (`:784` the mirror), `compiler/src/own/instance_check.rs`
  (`projection_resolves`).
- `compiler/src/codegen/function.rs` the pointer-member conversion (the store-slot/return
  scoping), the emitted cast shape.
- Controls: the `checker_conformance` `memenum`/`viewmembers` cases, the batch-K phase-3 pins,
  `ctl347`'s sanctioned cast.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| An unknown member on a base the frozen table defines | The typed member/rule; the fallthrough report is the unknown-member code | — |
| A `.ptr` read at an immutable destination | The existing read-only treatment (no cast) | — |
| A `cstring` slot/return | The existing member conversion | — |
| The const-drop conversion ruled illegal | The frozen `E10200` fallback | AR-7 |

## Testing Requirements

- Spec: the unknown members on the fixed array and the view (one `E10212` each, the span), the
  typed `.ptr`/`.len` controls, the mutable raw destinations (build and run; the sanctioned
  explicit-cast control), the immutable destinations unchanged, and the raw-enum member (the
  concrete/instance agreement).
- Impl: the rejection spans, the emitted cast at each destination, the enum member shape, the
  controls byte-identical; a re-emission determinism pin.
