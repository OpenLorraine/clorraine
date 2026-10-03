# Capture Diagnostic: Def Backlog Closeout

> **Document**: 03-05-capture-diagnostic.md
> **Parent**: [Index](00-index.md)
> **Defects**: DEF-69
> **AR**: AR-4, AR-5, AR-13, AR-14

## Overview

A lambda that reads a binding from an enclosing function or lambda without listing it in the
capture list is accepted through resolve and sema, then lifted without the binding and emits
undeclared C. The fix rejects the read with the approved `E10405`, enforcing the spec's explicit
capture list (L-1/L-2). This is the plan's single frozen-specification change.

## Architecture

### Current Architecture

- `resolve_expr`'s Lambda arm (`compiler/src/resolve/mod.rs:317-338`) resolves capture names and
  declares captures plus parameters in a pushed scope while outer scopes stay live.
- `Checker::check_lambda` (`compiler/src/sema/expr.rs:105-219`) binds only listed captures and
  parameters (`:156-162`), pushes `lambda_captures` (`:164`); free enclosing names are typed by
  `name_type` (`compiler/src/sema/mod.rs:421-459`), which walks every scope outward.
- `capture_mode` (`compiler/src/sema/mod.rs:360-367`) searches only listed captures and returns
  `None` for an unlisted name; `reject_capture_write` (E10402), `reject_capture_move` (E10401), and
  the `take` rejection (E10403) all key on it, so none fires for an unlisted read.
- `lift_lambda`/`declare_captures` (`compiler/src/own/lower.rs:378-496`, `:523-573`) rebuilds the
  binding tables from parameters and recorded captures; an unlisted name becomes a bare path and
  the emitter renders `sanitize_identifier(name)` into the lifted C function
  (`compiler/src/codegen/expr.rs:263-289`), which C rejects.

### Proposed Changes

1. **Specification** (approved): add to `spec/14-diagnostics.md` after E10404 —
   `E10405 | Lambda reads an enclosing name without a capture | lambda reads '{name}' without
   listing it | add '{name}' to the capture list` — and add
   `spec/tests/compile-fail/E10405_unlisted_capture.clo`. Register `E10405` in
   `compiler/tests/compile_fail_spec_test.rs` `ASSERTED_CODES` (`:31-43`). No chapter edit: L-1/L-2
   already state the capture-list rule.
2. **Sema**: `Checker::check_lambda` records a scope floor at lambda entry. A name that resolves to
   a binding introduced at or below that floor — an enclosing function local/parameter or an outer
   lambda's capture — and that is not in this lambda's captures or parameters reports `E10405`.
   Module-level bindings (globals, functions) are above every floor and stay valid.
3. **Nested lambdas**: each lambda sets its own floor, so an inner lambda must list an outer
   lambda's capture to read it (AR-5). The existing capture-of-capture recording in
   `declare_captures` is unchanged; the listing is now enforced at the source.

The check lives in sema, not in lowering: `lift_lambda` cannot distinguish an unlisted outer
binding from a module function once the name is a path.

## Implementation Details

### New Types/Interfaces

- `E10405` diagnostic call site with the approved message and suggestion; the diagnostic type is
  the existing `Diag`/`code: &'static str` pattern (`compiler/src/diag.rs:20-34`).
- A scope-floor helper in the checker (name and placement chosen during implementation; the
  natural home is beside `name_type`, `compiler/src/sema/mod.rs:421-459`).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Read of an enclosing function local/param not in the capture list | `E10405` with the approved message | AR-4, AR-14 |
| Read of an outer lambda's capture not re-listed | `E10405`; list at each level | AR-5 |
| Listed capture read (`[x, n] () => n`) | Accepted, unchanged | AR-4 |
| Capture-less lambda reading a module global/function | Accepted (above every floor) | AR-4 |
| Write/move of a captured binding | Existing E10402/E10401 unchanged | AR-4 |
| `take` capture | Existing E10403 unchanged | AR-4 |

> **Traceability:** Every strategy references the Ambiguity Register entry (AR #) that resolved it.
> See `00-ambiguity-register.md`.

## Testing Requirements

- Specification tests: ST-20 – ST-24 (`07-testing-strategy.md`); the compile-fail golden is the
  registry-template test.
- Implementation tests: nested capture cases, global reads, and the unchanged E10401/E10402
  behavior.
- `spec/check.sh` validates the registry row and the golden.
