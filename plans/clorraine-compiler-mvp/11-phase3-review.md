# Phase 3 Review: Name Resolution and Modules

> **Document**: 11-phase3-review.md
> **Parent**: [Index](00-index.md)
> **Date**: 2026-09-19
> **Scope mode**: strict

## What was reviewed

The complete Phase 3 diff from the phase baseline tree
(`fe1717c242353ac381e2e302265551580371054f`) through task 3.3.3: the new
`load` module (module table, merged modules, import bindings, the identifier
scan), the `stdlib` export table, the new `resolve` module, the parser's
misplaced-module record, the driver wiring, and the new spec and implementation
tests. Reviewer: an independent phase-review subagent carrying the
correctness, maintainability, and standards lenses, running `make check` and
exercising the built binary on adversarial inputs.

## Findings and rulings

| ID | Severity | Finding | Ruling (user, 2026-09-19) |
|---|---|---|---|
| RV-102 | 🔴 CRITICAL | The stdlib table omitted `std.mem.slice`, so the corpus `file-copier` program would fail with a false `E11102` | Fix: add `slice` to the table and pin the whole corpus import set against it |
| RV-103 | 🟠 MAJOR | Names inside an `ffi` block were never module declarations, so `handle T freedBy F` and direct ffi calls reported a false `E10219` | Fix: register each ffi signature name as an unexported declaration and include them in the duplicate check |
| RV-104 | 🟡 MINOR | The unused-import scan skipped ffi signatures, and it is scope-blind | Fix: scan ffi signatures and handle `freedBy` names; document the deliberate conservative scope-blindness |
| RV-105 | 🟡 MINOR | A module could not reach its own private declaration by full name (`app.helper()` inside `app`) | Fix: own-module qualified access accepts any declaration; only other modules require `export` |
| RV-106 | 🟡 MINOR | The `E11104`/`E11105` help text differed from the Ch 14 registry | Fix: use the registered help strings |
| RV-107 | 🟡 MINOR | No test pinned alias imports or the stdlib export surface | Fix: add an alias-import test and a corpus-import surface test |
| RV-108 | 🟡 MINOR | `resolve()` returns no resolved symbol graph | Defer (user ruling): no consumer exists yet; the type layer will add annotations when it needs them. The deferral is recorded in the execution plan at task 3.2.2 |
| RV-109 | 🟡 MINOR | `E11104` could cascade when the declaration holding `main` was dropped by an earlier error | Fix: skip the entry-point check when any error was already reported |

## Fix evidence

- `make check` green: spec check OK (27 files, 101 compile-fail tests), 76
  unit tests, 2 compile-fail driver tests, 8 stage driver tests.
- New tests: ffi signature and deleter resolution, ffi names in the duplicate
  check, own-module qualified access, alias imports, an import used only in an
  ffi signature, entry-point poisoning suppression, and the corpus-import
  surface check (which is what would have caught RV-102).
- Size standard: `resolve.rs` was split into `resolve/mod.rs` plus
  `resolve/tests.rs`; every source file is again under the threshold.

## Re-review of the fix diff

The single permitted re-review confirmed RV-102, RV-103 (bare-name contract),
RV-104, RV-105, RV-106, RV-107, and RV-109 resolved, with RV-108's deferral on
record, and reported two new minor findings:

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| RV-110 | 🟡 MINOR | A qualified deleter such as `handle File freedBy app.Close;` still reported a false `E10219`, although the grammar allows a qualified name and RV-105 makes the self-module form legal | Fixed: dotted names in type and deleter position share one resolver that routes multi-segment names through qualified resolution; a `qualified_deleter_resolves_in_its_own_module` test pins it |
| RV-111 | 🟡 MINOR | The entry-point suppression is broader than the root cause: it skips on any prior error, so a mistyped `main` is not reported when an unrelated error exists | Accepted as the authorized RV-109 behavior; no spec or golden requires the narrower scope, and the tradeoff is recorded here |

A double blank line before the `resolve` test module was also cleaned up. The
re-review's remaining nit (the test-only corpus-import parser would misread a
trailing comment on an import line) has no current corpus case and is left as
is.
