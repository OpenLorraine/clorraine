# Review Report: Generic Substitution Guards

> **Document**: 00-review-report.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-26 22:41

## Phase 1: Instantiation-time audit

- **Phase baseline tree**: `3442b3af1a470919cfdfbc3b187f5db3e419ab94`
- **Scope mode**: strict
- **Diff**: `compiler/src/own/instance_check.rs` (new), `compiler/src/own/generics.rs`,
  `compiler/src/own.rs`, `compiler/tests/ownership_drop_spec_test.rs`,
  `compiler/tests/ownership_drop_impl_test.rs`, `plans/generic-substitution-guards/99-execution-plan.md`
- **Reviewers**: correctness reviewer (independent generic subagent; the specialist
  agent is manual-only, so a generic subagent received the complete dispatch packet)
- **Lenses**: correctness (base); maintainability; standards; domain lens: compiler and language
- **Verdict**: ✅ no findings

The reviewer re-ran every suite in the pinned container and a clean
`cargo check -p clc --all-targets` (no warnings), then traced predicate parity,
the value-walk arm set against `generics::walk_value`, span attribution,
`substitute_instance` parameter ordering, the `struct_declarations` refactor, and
false-positive controls. It reported `no findings` explicitly.

A follow-up re-review was not required because no critical or major finding was
raised.
