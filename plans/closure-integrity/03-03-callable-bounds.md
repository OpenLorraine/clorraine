# Callable Bounds: Closure Integrity

> **Document**: 03-03-callable-bounds.md
> **Parent**: [Index](00-index.md)

## Overview

Phase 3 rejects a call of a value whose type is a type parameter without a callable bound
(DEF-110). The grammar already spells callable bounds on type parameters (`<T: func(i32): i32>`,
`spec/grammar.ebnf.md:521-522`), and the frozen rules make a callable value callable only through
such a bound (G-1/G-6, `spec/06-functions.md:248-272`); the current checker silently accepts the
unbounded call and the emitter spells `x(5)` on a non-callable value.

## Architecture

### Current Architecture

`check_call`'s callee typing (`compiler/src/sema/call.rs:91-129`) continues the callable path only
when the callee's type is `TyKind::Param(name)` **and** `name` is in `callable_bound_params`
(`sema/mod.rs:220-227`): a bare `T` falls through the match with no diagnostic and the callee
resolves to `Ty::unknown()` (`:124-129`). The ownership stage then treats the callee as a binding
call (`own/call.rs:159-165`), and codegen spells the source name with the argument list
(`codegen/expr.rs:575-588`), emitting `x(5)`.

### Proposed Changes

In the callee typing, a `TyKind::Param(name)` that is not in `callable_bound_params` reports the
new `E10307` (`type parameter '{name}' is not callable`, help ``declare a callable bound:
`T: func(...)` ``) at the call, and the call continues with `Ty::unknown()` so later checks do not
cascade. A parameter with a callable bound keeps its current path (including the E10303 argument
check at `sema/call.rs:310-357`).

### Value positions

The judgment covers every call position the checker types (expression, statement, argument,
return); the callee's type is the only input, so one site covers all of them.

## Implementation Details

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `sema/call.rs` | the callee-typing match gains the unbounded-parameter arm |
| `sema/mod.rs` | `callable_bound_params` is read, not changed |
| `spec/14-diagnostics.md` | the authorized `E10307` row (AR-3) |
| `spec/tests/compile-fail/` | the `E10307` golden (AR-12) |
| `compiler/tests/compile_fail_spec_test.rs` | the asserted-code registration (AR-12) |
| the emitter | unchanged; the rejected program writes no translation unit |

## Code Examples

```clorraine
func g<T>(x: T): i32 { return x(5); }                  // E10307: type parameter 'T' is not callable

func h<T: func(i32): i32>(x: T, n: i32): i32 {
    return x(n);                                        // accepted: the bound declares callability
}
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Call of an unbounded type-parameter value | one `E10307` at the call, naming the type parameter | AR-3 |
| Call of a callable-bound parameter | accepted; the argument check stays `E10303` | AR-3 |
| Non-callable concrete values | unchanged (the existing call diagnostics) | AR-3 |

> **Traceability:** every error-handling strategy back-references its register entry. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification cases ST-14 – ST-16 ([07-testing-strategy.md](07-testing-strategy.md)).
- The `E10307` golden runs through `./spec/check.sh` and the compile-fail driver.
- Implementation pins: one report per site, the diagnostic span and name, the unchanged bounded
  path, and the no-translation-unit rule.
