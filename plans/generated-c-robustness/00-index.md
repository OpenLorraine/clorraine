# Generated-C Robustness Bundle

> **Feature**: Close #16, #30, and #31: reserve the `__cl_itmp` init-local
> prefix, release an owned temporary argument to `text(...)`, and let the
> `std.fmt.text` module path win over a module-level `std` value.
> **Status**: Implementation complete; `make verify` blocked at the FS-UAE legs
> **Created**: 2026-11-12 14:15
> **Implements**: R1–R6
> **CodeOps Artifact Schema**: 1

**Original goal:** deliver the confirmed Bundle 1 without changing the frozen
spec, stdlib, or ABI.

## Overview

Three pre-existing generated-C defects found by the #29 reviews:

1. `__cl_itmp<n>` init locals can shadow a user `ffi` name (#16).
2. `text(stringOf("x"))` / `text($"{s}")` leaks the owned temporary argument
   in a function body (#30).
3. A module-level or imported `std` value makes `std.fmt.text(7)` emit the
   literal `std.fmt.text(7)` instead of the module path/intrinsic (#31).

The fixes are small and independent: one symbol-reservation scan, one
synthetic borrow mode, and one precedence-gate removal grounded in MOD-13/MOD-16,
plus an initializer ordering fix so the module path wins there too.

## Minimum-Sufficient Baseline

**Original goal:** make each reported program build and run correctly.
**Smallest viable design:** mirror the existing `__cl_ctmpl` reservation in
`next_init_local`; give the intrinsic call a synthetic `Mode::Borrow`; remove
the module-level value-head gate from sema, own, and the initializer walk, and
decide the intrinsic before the initializer's callable fallback.
No new diagnostic, IR node, runtime helper, language surface, or spec edit.

## Document Index

| # | Document | Description |
|---|----------|-------------|
| AR | [Ambiguity Register](00-ambiguity-register.md) | Decisions and provenance |
| 00 | [Index](00-index.md) | This document |
| 01 | [Requirements](01-requirements.md) | Requirements and acceptance |
| 02 | [Current State](02-current-state.md) | Reproductions and root causes |
| 03 | [Implementation Design](03-implementation.md) | The three fixes |
| 07 | [Testing Strategy](07-testing-strategy.md) | Spec and impl cases |
| 99 | [Execution Plan](99-execution-plan.md) | Phases and tasks |
| 05 | [Findings](05-findings.md) | Evidence ledger (created at review) |

## Key Decisions

| Decision | Outcome |
|----------|---------|
| `std` value vs module path | Module path wins; only a local shadows (MOD-13/MOD-16) |
| Initializer order | Decide the intrinsic before `init_callable_value` |
| Text argument ownership | Borrow-materialize the owned body temporary; named places stay copies; the initializer already releases |
| Init local naming | Reserve `__cl_itmp` with `is_taken_symbol` |
| #29 IT-13 | Superseded: update the module-level value-head pin |
| Spec/ABI | No change |

## Related Files

Modified:
- `compiler/src/codegen/program.rs` — `next_init_local`, initializer text order and precedence
- `compiler/src/own/call.rs` — synthetic text borrow mode
- `compiler/src/own/expr.rs` — qualified intrinsic gate
- `compiler/src/sema/call.rs` — qualified intrinsic gate
- `compiler/tests/template_placeholder_residuals_iii_impl_test.rs` — IT-13 pin update
- `compiler/tests/template_placeholder_residuals_iii_spec_test.rs` — ST-15 comment update
- `compiler/tests/generated_c_robustness_{spec,impl}_test.rs` — new suites
- `plans/template-placeholder-residuals-iii/{01,03,05,07,99}` — supersession note
- `plans/00-roadmap.md` — PLAN-35 row
