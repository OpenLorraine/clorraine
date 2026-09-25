# Spec and Goldens: Clorraine v1 language completion

> **Document**: 03-04-spec-goldens.md
> **Parent**: [Index](00-index.md)

## Overview

Implementing generics, lambdas, and volatile access changes the frozen
specification's stated status for several constructs and retires one staged
diagnostic. This component is the single owner of the authorized `spec/` and
golden-data edits (AR-3, AR-32). No other document restates them.

## Authorized Modification Set

| Artifact | Edit | AR Ref |
| -------- | ---- | ------ |
| `spec/06-functions.md` | §6.1 prose: lambda semantics are implemented, not staged; correct the `let bump = ...` example to an inline call; L-7 retired; the diagnostics table drops `E90002` | AR-10, AR-11, AR-12, AR-21 |
| `spec/grammar.ebnf.md` | §8 note: remove "semantics are staged (L1, L4)"; state that a capturing closure is inline-only and fills a callable bound | AR-11, AR-32 |
| `spec/14-diagnostics.md` | `E90002` marked reserved with no trigger; `E10303`/`E10304`/`E10306` and `E10400`-`E10404` lose any "staged" wording | AR-21, AR-25 |
| `spec/tests/compile-fail/E10303_callable_bound_violated.clo` | Asserted (removed from the excluded list) | AR-25 |
| `spec/tests/compile-fail/E10304_callable_value_stored.clo` | Asserted (removed from the excluded list) | AR-25 |
| `spec/tests/compile-fail/E10306_generic_ephemeral_arg.clo` | Remove the `// stage:` annotation; assert `E10306` | AR-16 |
| `spec/tests/compile-fail/E10400_capturing_lambda_as_func.clo` | Asserted (removed from the excluded list) | AR-10 |
| `spec/tests/compile-fail/E10401..E10404` | Remove the `// stage:` annotations; assert each | AR-13, AR-14, AR-22 |
| `spec/tests/compile-fail/E11200_custom_module.clo` | New golden: importing `std.amiga.custom` under `host` is `E11200` | AR-7 |
| `spec/tests/compile-fail/E90002_lambda_capture_staged.clo` | Removed; `E90002` has no trigger | AR-21 |
| `spec/tests/staged/README.md` | Drop the `E90002` reference; note `E90000` still backs the `Array<move-only>` case | AR-21, AR-34 |
| `spec/tests/compile-fail/README.md` | Add `E90002` to the unreachable-until-a-trigger list | AR-21 |
| `spec/check.sh` | Add `E90002` to the excluded-code list | AR-21 |
| `compiler/tests/compile_fail_spec_test.rs` | Remove `E10303`, `E10304`, `E10400` from `EXCLUDED_CODES`; the un-skipped files join `ASSERTED_CODES`; `E90002` is not asserted | AR-21, AR-25 |
| `AGENTS.md` | Test-naming map and any staged-surface note | AR-32 |
| `plans/00-roadmap.md` | PLAN-04 stage and link (set by the roadmap skill) | AR-32 |

`spec/17-codegen.md` §8 already specifies the intended closure emission
(capture-less → function pointer; capturing → two-word callable) and needs no
edit.

`spec/decisions.md` P11, `spec/17-codegen.md` §7, and `spec/grammar.ebnf.md` §11
describe a hash for *over-long* instantiation keys. This plan reuses the existing
canonical-key scheme, so those sites need no edit; the over-long-key hash is a
pre-existing, separate concern outside this plan.

## Golden Reconciliation Rules

- A file listed in `ASSERTED_CODES` must have a golden that exits nonzero with
  the exact first-line code, and the message must match the registry template.
- A code that remains registered but has no trigger is excluded in `spec/check.sh`
  with a reason, matching the existing `E90000`/`E90003`/`E90004` treatment.
- The retired `E90002` stays registered forever (N2) but is never emitted; its
  golden is removed and the compile-fail suite stops expecting it.
- `E90000` keeps exactly one trigger after this plan: a non-duplicable `Array`
  element type (AR-34); the file `spec/tests/staged/E90000_generic_staged.clo`
  and the staged README are updated to say so.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A spec edit breaks `./spec/check.sh` | Fix the edit; the check is the spec's test and must stay green | AR-3 |
| A golden still carries a `// stage:` annotation | Remove it in the same task that asserts the code | AR-25 |
| `E90002` remains referenced after retirement | Grep the registry, Ch 06, the staged README, and the suite | AR-21 |

> **Traceability:** every edit cites the register entry that authorized it; see
> `00-ambiguity-register.md`.

## Testing Requirements

- `./spec/check.sh` passes after every edit (headers, links, registry
  cross-check, and compile-fail coverage).
- The compile-fail suite reports no unexpected skip and asserts every
  un-excluded code.
