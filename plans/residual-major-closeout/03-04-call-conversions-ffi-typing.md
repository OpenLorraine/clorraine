# Call Conversions and `ffi` Result Typing: Residual Major Closeout

> **Document**: 03-04-call-conversions-ffi-typing.md
> **Parent**: [Index](00-index.md)

## Overview

Two batch G Phase 3 rows in the call-typing surface: a parameter-typed argument whose
conversion is deferred to an audit that does not exist (`DEF-242`, major-class), and an `ffi`
result whose declared type is discarded (`DEF-243`, major-class). The fix shapes are the
existing per-instance duty pattern (`own/instance_check.rs`, the iterable/storable/out-slot
duties) and the written `ffi` signature the checker already resolves for parameters
(`sema/call.rs:692`). No new diagnostic code: the duty reports the frozen `E10200`, and the
`ffi` result type restores the ordinary conversion judgments.

## Design

### 1. The parameter-typed conversion duty (`DEF-242`)

`call<str>(v) { unsafe { Open(x, 0); } }` is accepted (the `contains_param` guard defers the
conversion, `sema/call.rs:741`/`:765`) and the emitted C passes the `ClStr` view where the
`char *` parameter is declared; the ordinary `cstring` twin, the `return x` position at
`make<i32>()`, and the `let y: T = x` declaration position are the same hole (probed red,
AR-3). The legitimate instances (`T = cstring`, `T = str`, `T = String`) must stay accepted.

Record a conversion duty at each site whose written value may resolve through a parameter and
whose destination is concrete:
- At the argument sites (`check_ffi_arguments` and the direct-call check), where the current
  guard skips the conversion while the type mentions a parameter.
- At the value positions the row names: a `return` whose value type mentions a parameter and
  whose function's return type is concrete, and a `let`/`const` declaration whose declared type
  is concrete and whose value type mentions a parameter.

The duty carries the declared source type, the destination type, the function, and the span;
the instance audit substitutes the instance's arguments and judges the concrete conversion with
the ordinary rules, reporting the frozen `E10200` once per site where the conversion is not
implicit. A site whose substitution converts legally (the `cstring`/`str`/`String` instances)
reports nothing. The null-shaped source (`let x = null; return x;`) reports the frozen
null judgment at the instance (the same E10012 family the argument rule uses), not a raw
`void *` conversion.

This is the iterable duty's shape (`own/instance_check.rs:897`): record once at the
declaration, substitute per instance, report the first offending instance, keep the accepted
instances. Stop condition (AR-8): if the duty cannot be expressed without rejecting the legal
instances, the phase stops and parks the row with the evidence.

### 2. The `ffi` result's declared type (`DEF-243`)

The sema types every `ffi` call result as `Ty::unknown()` (`sema/call.rs:76-83`), so
`Uses(Count())` with `func Count(): i32` and `func Uses(s: cstring)` is accepted and fails the
strict build (`passing argument 1 ... makes pointer from integer`).

Carry the declared result type into the call's type: the `ffi_signature` (`sema/call.rs:683`)
already resolves the written signature; resolve its return type through the same path the
parameters take (`ffi_parameter_types`, `:692`) and return it as the call's checked type. The
ordinary conversion judgments then apply at the result's uses: `Count()` at a `cstring`
parameter reports the frozen conversion code instead of a C build failure, a valid integer
destination accepts, and the emitter's ABI prototype is unchanged (it already renders the
declared return type). A declaration with no return (`: void`) keeps the void type.

## Integration Points

- `sema/call.rs:76-83` — the `ffi` result type.
- `sema/call.rs:692-765` (`ffi_parameter_types`, `check_ffi_arguments`) and the direct-call
  check — the duty recording sites.
- `sema/mod.rs:185-187` (`ffi_params`) — the resolved-signature cache the return type joins.
- `own/instance_check.rs:57` (the duty audits), `:897` (the iterable duty) — the duty pattern.
- `own/call_check.rs` — the ownership-side argument checks that read the resolved types.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A parameter-typed argument whose instance cannot convert | `E10200` once per site at the first offending instance; legal instances stay accepted | AR-8 |
| A null-shaped source at a concrete destination | The frozen null judgment at the instance | — |
| A duty whose instance still mentions a parameter | The existing deferral (judged at its own instance) | — |
| An `ffi` call with an undeclared return type | The existing declaration validation | — |
| A void `ffi` result used as a value | The existing void-use rule | — |

## Testing Requirements

- Spec: the `ffi` argument spelling and the ordinary `cstring` twin (rejected once with the
  frozen conversion code), the legal instance controls (`T = cstring`/`str`/`String`, accepted
  and building), the parameter-typed return and declaration positions (rejected once each),
  the null-shaped source (the frozen null code), and the `ffi` result at an integer and at a
  `cstring` destination (the conversions judge; the valid case runs).
- Impl: the duty records, the per-instance report counts and spans, the emitted `ffi` prototype
  unchanged, the accepted-instance emissions byte-identical, and a re-emission determinism pin.
