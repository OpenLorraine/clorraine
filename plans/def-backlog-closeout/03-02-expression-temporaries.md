# Expression Temporaries: Def Backlog Closeout

> **Document**: 03-02-expression-temporaries.md
> **Parent**: [Index](00-index.md)
> **Defects**: DEF-67
> **AR**: AR-6

## Overview

An owned call result consumed as a borrow argument inside a larger expression is emitted inline
with no binding and no release. The fix routes intrinsic-call arguments and dot-`len` receivers
through the owned-argument binding the named-call path already uses, keeping the guarded-operand
deferral. This is the smallest change that closes the filed leak without materializing every owned
value position.

## Architecture

### Current Architecture

- `lower_value`, `ExprKind::IntrinsicCall` (`compiler/src/own/lower.rs:244-258`): lowers each
  `IntrinsicArg::Expr` with plain `lower_value`; no owned binding, no release.
- `lower_intrinsic_call`, dot `len` (`compiler/src/own/call.rs:273-285`): lowers the receiver with
  `lower_dot_receiver` inline; no binding.
- `materialize_owned_argument` (`compiler/src/own/lower.rs:1336-1432`): the named-call path's
  "callee borrows, caller owns" binding; binds to a body-scope `__cl_ownedN` local whose scope
  cleanup releases it.
- `materialize_guard_operand` (`:646-679`): the deferred `Materialize` shape for a guarded operand,
  registering the release on the evaluated path.
- Release drain: statement end drains `pending_releases` (`compiler/src/own.rs:1065-1073`);
  a return carries `pending_release_copies()` on its terminator
  (`compiler/src/own/statements.rs:867-993`).
- Loop conditions already route through `materialize_owned_calls`
  (`compiler/src/own/loops.rs:664-767`), which is why the loop-condition form is clean.

### Proposed Changes

1. In `lower_value`'s intrinsic arm (`compiler/src/own/lower.rs:244-258`), route each
   `IntrinsicArg::Expr` through `materialize_owned_argument` when the argument is owned and the
   intrinsic consumes it as a borrow; under `defer_move_outs > 0` use the guard-deferred shape so
   short-circuit evaluation stays lazy.
2. In `lower_intrinsic_call`'s dot-`len` receiver (`compiler/src/own/call.rs:273-285`), do the same
   for an owned receiver.
3. Do not bind when the value is adopted or moved (take/store positions) — the binding applies only
   in borrow-argument position, so no double release can appear.

The existing `storeAt` body shapes asserted by the prior impl test
(`def_backlog_completion_impl_test.rs:391-454`) stay unchanged; a call-result argument now gains
its binding and release, so whole-program byte-identity holds only for programs that pass a place.

## Implementation Details

### Integration Points

- The statement path binds through the existing owned-argument local; the guard path reuses the
  deferred `Materialize` release; no IR or emitter change is expected.
- `check_return`'s existing release copies (`compiler/src/own/statements.rs:974-983`) cover the
  return form once the release is registered.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Owned call result passed to a borrow-taking intrinsic (dot `len`, `slice` arg) | Bind via the owned-argument path; release at statement end/return | AR-6 |
| Owned call result inside a guarded operand (`&&`, `||`, `?:`) | Guard-deferred `Materialize`; release only on the evaluated path | AR-6 |
| Value is adopted by a store/take | No binding; keep the existing path | AR-6 |

> **Traceability:** Every strategy references the Ambiguity Register entry (AR #) that resolved it.
> See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: ST-8 – ST-11 (`07-testing-strategy.md`).
- Implementation tests: release shape for the intrinsic argument, the dot receiver, and the guard
  deferral.
- Host-ASan probes: the DEF-67 program and its dot/guarded variants.
