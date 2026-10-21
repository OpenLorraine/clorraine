# Design — Global Borrow and Move Conformance (Batch U)

## Phase 1 — `DEF-533`: BR-9 applies uniformly

- Remove the `receiver` exemption from `check_argument_place`; every non-
  `const` global target reports `E10038` for `borrow`, `const`, and unmarked
  parameters.
- Remove the module-qualified and unmarked scalar/view exemptions from the
  initializer walk. Resolve the callee with the body's scope order and report
  through one shared `report_initializer_global_borrow`.
- Check a view builder's source in both the intrinsic and `Callee::Stdlib`
  forms.
- Retarget the conflicting oracles and emission pins to `const`, local, or
  call-result sources. Add rejection spec cases for the module-qualified
  receiver, the view builder, and the unmarked scalar/view arguments.

## Phase 2 — `DEF-537`: func-typed initializer calls

Resolve the callee's `TyKind::Func` signature when the callee is a func-typed
global or a func-typed field of the receiver. Judge each written argument
against the signature's parameter types and unmarked lend semantics; a
`take` parameter stays accepted. A `const` global and a non-place argument
stay accepted.

## Phase 3 — `DEF-425`: generic element move through a borrow

Record the element move-out through a borrowed parameter in the per-instance
duty and judge it at the instance: the concrete `borrow` twin reports
`E10034` at the element read; the `take` twin reports `E10026`. Retarget the
pinned acceptance cases to the rejection. The implementation uses the
`RequiresNotLend` projection path (`project_place_path` +
`check_not_lend_requirements`).

## Verification

- Phase 1: the new rejection spec cases, the retargeted oracles, the focused
  suites, `make check`, `make verify`.
- Phase 2: the func-global/field argument rejection and the accepted controls.
- Phase 3: the generic/concrete/take case matrix and the duplicable control.
