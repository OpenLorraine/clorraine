# Phase 4: Null-Shaped Values

> **Document**: 03-04-null-shaped-values.md
> **Parent**: [Index](00-index.md)

## Overview

Two defects around the null shape — the type of a local inferred from `null`
(`TyKind::Unknown` with `nullable: true`, `sema/expr.rs:51`): a store of a non-null-shaped
value into such a local (DEF-149) and a use of it where a concrete value is required, including
the iterable position (DEF-151; the DEF-145 null-shaped sub-case closes here).

## Current Arc

`implicit_conversion` (`types/mod.rs:595-607`) short-circuits to `true` whenever either side is
unknown, so the null shape converts to anything: `x = s` (a `str`) is accepted and emits
`void * = ClStr`; `return x;` from an `i32` function emits `return void *`; `for (v in x)`
emits a `< 0` comparison on an unsigned bound. All three were probed at head and fail the
strict build. The frozen rule (`spec/03-ownership.md:196`) already names the diagnostic:
"Using `null` where a non-nullable type is required is E10012", and the registry row exists
(`'null' is not allowed here` / "declare the type `T?` or avoid the empty state").

## Proposed Changes

### 1. A shared null-shape predicate — `compiler/src/types/mod.rs`

Add a small predicate (`is_null_shaped(ty)`: `TyKind::Unknown` with `nullable: true`), with
unit tests; Phase 2 already uses it for the iterable case.

### 2. The rejection sites — `compiler/src/sema/`

Report `E10012` at the site when a null-shaped value is used where a concrete non-nullable
type is required, and when a store targets a null-shaped local with a non-null-shaped value:

| Site | Rule |
|------|------|
| a store into a null-shaped local (DEF-149) | only a null-shaped value (or `null`) may be stored; anything else reports `E10012` |
| a return of a null-shaped value from a concrete-typed function (DEF-151) | `E10012` |
| an argument position with a concrete parameter type | `E10012` |
| an iterable expression (the null-shaped sub-case) | `E10012` |

The accepted positions are pinned unchanged: `x == null` / `x != null`, `let x = null;`,
copies between null-shaped locals, `null` at a nullable-typed target (`let f: File? = null;`),
and `null` at an out-slot parameter.

The rejection uses the conversion site's span; one report per site (the conversion check runs
once per position), and no cascade follows (the expression's type stays the null shape so
later checks behave as today).

## Tests

| Case | Shape | Expectation |
|------|-------|-------------|
| `nullassignview` | `x = s` (DEF-149) | one `E10012`; the message renders the registry text |
| `nullreturn` | the `i32`-return repro (DEF-151) | one `E10012` |
| `nullarg` | a null-shaped value at a concrete parameter | one `E10012` |
| `nulliter` | `for (v in x)` with a null-shaped `x` | one `E10012` |
| `nullcompare` | `x == null`, `x != null` guards | accepted; byte-identical emission |
| `nullnullable` | `let f: File? = null;` and a narrowed use | accepted; byte-identical emission |
| `nullcopy` | a copy between two null-shaped locals | accepted |

## Acceptance

- Each rejected shape reports exactly one `E10012`; each accepted shape is unchanged and
  byte-identical.
- The corpus differential shows no delta beyond the new rejections (which emit no C).
