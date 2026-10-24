# Strict Build and Type Identity: Critical/Major DEF Closeout

> **Document**: 03-E-strict-build.md
> **Parent**: [Index](00-index.md)

## Overview

Batch E closes the three strict-build/identity rows. Two are parser/analysis
acceptance gaps where the frozen spec already defines the correct response; one
is a type-table identity collision.

## Architecture

### Current Architecture

- `compiler/src/parse/` resolves the generic-call shape; `compiler/src/sema/call.rs`
  checks explicit type arguments and reports `E10300` for the generic paths, but
  a non-generic free/qualified/dot/func-typed callee and a non-callable
  local/parameter callee currently ignore the written type arguments.
- `compiler/src/driver.rs::Options::parse` owns the `clc` option surface for the
  parse-only/stage-select mode.
- `compiler/src/sema/expr.rs::check_intrinsic` types `embed` (currently falling
  through to an unknown type) and has no constant-context signal; `consts/mod.rs`
  folds constants and reports E10224; `compiler/tests/compile_fail_spec_test.rs`
  excludes E10213/E10214.
- `compiler/src/types/table.rs::build` keys declared types by simple name and has
  no diagnostics sink; the `DEF-416` collision must be reported by a
  cross-module check in `sema` (with a pure `TypeTable` query), not from the
  table constructor. It is a collision report, not a qualified key layer.

### Proposed Changes

| DEF | Change | Existing helper |
|---|---|---|
| 167 | Keep `a < b > (c)` as the frozen generic call; add a parse-only/stage-select mode for the stage test; report one `E10300` on **every** non-generic callee, including a non-callable local/parameter value. The filed stage shape is the local i32 `a`; the free-function form is the `DEF-418` twin | `sema/call.rs` type-argument check + `driver.rs` mode |
| 171 | Enforce DI-4: `embed` outside a constant context reports `E10213`; do not lower or emit; re-enable the E10213 golden. Add a constant-context flag so a module `const` keeps only `E10224`. A module-level `let embed` reports `E10213` under R-6 (`spec/04` §16; the `spec/13` §1 sentence is a recorded frozen inconsistency) | `sema/expr.rs` intrinsic check + `sema/items.rs` context flag + compile-fail harness |
| 416 | Report one `E11101` at the second declaration for a cross-module same-simple-name type from a `sema` cross-module check (pure `TypeTable` collision query); keep the simple table and do not emit C | `sema/mod.rs` + `types/table.rs` query |

No parser grammar change is planned. `DEF-167` is a type-stage rejection, not a
reparse; the parse-only mode preserves the stage-spec parse assertion, and the
same local-i32 source must still be rejected by a full compile. `DEF-171` uses
the existing diagnostic registry and golden; the positive constant-context
`embed` path is not implemented because data inclusion is staged out. A
module-level `let embed` reports `E10213` under R-6: the plan follows
`spec/04` §16 and records the `spec/13` §1 sentence as a frozen inconsistency;
it changes no spec chapter. `DEF-416` is a conservative collision report: the
spec's module-namespace allowance remains a recorded limitation (AR-20), not a
new type-identity layer.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| Explicit type args on a non-generic callee | One `E10300` wrong number of type arguments for every non-generic callee form, including the non-callable local/parameter value; no C; the parse-only stage test stays accepted | AR-14, AR-19 |
| `embed` outside a constant context | `E10213`; no C; the golden is re-enabled; a module `const` keeps only `E10224` (staged); a module-level `let` reports `E10213` under R-6 | AR-15, AR-23 |
| Same simple type name in two modules | One `E11101` at the second declaration from the `sema` cross-module check; no C; keep the simple table | AR-20 |

## Testing Requirements

- `DEF-167`: the stage-spec parse stays accepted through the parse-only mode;
  the **same local-i32 source** in a full compile rejects with one `E10300` and
  writes no C. Also cover the free, qualified, dot, and func-typed callee
  forms; `DEF-418` is the free-function twin. Name `driver.rs` as the owner of
  the parse-only option.
- `DEF-171`: the `E10213` golden is re-enabled (remove E10213 from
  `EXCLUDED_CODES`, add it to `ASSERTED_CODES`; keep E10214 excluded); no
  `memcpy(&(data), &(0), ...)`. A non-`embed` constant control builds; a
  module `const embed` reports only `E10224`; a function-body/local `let` or
  `const` `embed` and a module-level `let embed` report only `E10213` (R-6).
- `DEF-416`: two modules declare `Thing` with different fields; the second
  declaration reports one `E11101` and no C is written. `qinitstructname` is
  retargeted from its current success assertion to the collision rejection.
  A same-module bare `Thing` control is a separate one-module program and
  resolves normally.
- Existing stage/compile-fail suites stay green; the stage test uses the
  parse-only mode; `generic-mixed`/`generic-mixed-local` and the other pinned
  counts are retargeted in their owning phase; any minor twin closed
  incidentally (for example `DEF-418`) is recorded, not expanded into new work.
