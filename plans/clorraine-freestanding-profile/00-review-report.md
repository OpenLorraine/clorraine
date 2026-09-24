# Review Report: Freestanding profile

> **Document**: 00-review-report.md
> **Parent**: [Index](00-index.md)
> **Scope mode**: strict
> **Phase reviewed**: 2 (profile and runtime) and 3 (driver, specification, goldens)
> **Reviewed range**: `f517d32..1edbba2`
> **Reviewer**: independent correctness reviewer (subagent)
> **Last Updated**: 2026-09-24 08:40

## Findings and Rulings

| # | Severity | Finding | Ruling |
|---|----------|---------|--------|
| RV-01 | 🟠 MAJOR | The `static str -> cstring` widening was type-level, but codegen emitted a `char *` only for a literal call argument; `let p: cstring = "abc";` and `return "abc"` compiled to `char * = ClStr`, which fails to compile | User accepted: restrict the widening to a direct, NUL-free `static str` **literal call argument**; every other `static str -> cstring` site keeps `E10200` |
| RV-02 | 🟠 MAJOR | A template placeholder reached the heap formatter without an import (`println($"n={n}")`), emitting `cl_text_*`, `cl_string_concat`, `cl_string_ptr`, `FreeString`, and a raw `NULL` that the freestanding shim does not define — a link failure instead of a diagnostic | User accepted: reject a template placeholder under freestanding with compile-time `E11200` |
| RV-03 | 🟡 MINOR | `cl_panic` printed `panic: : message` because `cl_report` always inserted `": "` | Applied: `cl_report` omits the separator when the code is empty |
| RV-04 | 🟡 MINOR | A literal with an embedded NUL (`"a\0b"`) widened to `cstring` and silently truncated | Applied with RV-01: an embedded-NUL literal is no longer accepted as a `cstring` |

## Verification of the Rulings

- RV-01: `compiler/src/types/mod.rs` no longer widens `static str` to `cstring`;
  `sema/call.rs::is_cstring_literal_argument` allows only a NUL-free literal in
  argument position; `codegen/calls.rs::argument_text` emits the C string
  literal. The stage suite covers both the acceptance and the rejection.
- RV-02: `sema/check_template` reports `E11200` when the profile is
  `freestanding` and the template has a placeholder, so the driver stops before
  codegen. The freestanding suite covers the case.
- RV-03/RV-04: the shim and the argument gate are asserted by the suites.

No finding was waived and no optional remediation was accepted as scope.
