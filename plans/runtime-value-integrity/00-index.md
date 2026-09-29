# Runtime Value Integrity Implementation Plan

> **Feature**: Fix the seven filed runtime-value defects: a call-rooted element-loop iterable emits invalid C (`DEF-24`), an owning compiler temporary is never released (`DEF-23`), a fixed-array conditional value emits invalid C (`DEF-25`), a runtime-index callable-field receiver is never bounds-guarded (`DEF-26`), a func value with a fixed-array return carries the wrong C pointer type (`DEF-27`), a non-literal panic message fails the C compile (`DEF-28`), and a `take` argument in a guarded operand moves eagerly (`DEF-29`)
> **Status**: Complete
> **Created**: 2026-09-29
> **Implements**: DEF-23, DEF-24, DEF-25, DEF-26, DEF-27, DEF-28, DEF-29
> **CodeOps Artifact Schema**: 1

## Overview

Seven filed defects share one theme: a runtime value the compiler already accepts is
materialized, typed, or evaluated incorrectly. A call-rooted element-loop iterable lowers
`mk()[i]` while the callee uses the hidden out-parameter ABI; a materialized owning
temporary is never released although the specification requires a statement-end drop; a
fixed-array conditional value byte-copies a C ternary whose array operands have decayed to
pointers; a callable-field receiver with a runtime index is never bounds-guarded; a
`func(...)` value with a fixed-array return renders without the hidden out-param; a
`str`/`String` panic message renders a view/handle where the runtime takes `const char *`;
and a `take` argument inside a short-circuit operand emits its move-out eagerly.

The fixes reuse existing machinery rather than adding layers: the ownership stage already
materializes guard operands (`ValueKind::Materialize`), the codegen stage already writes
fixed-array values through one assignment path, and the guard emitter already defers
operands. The plan extends those paths — temporary release with its view-lifetime rule,
iterable materialization, branch-aware conditional stores, receiver guards, the out-param
pointer spelling, a pointer + length panic runtime, and deferred move-outs — against the
frozen specification. The release of an owning statement temporary is only sound when a
view derived from it cannot outlive its statement, so the plan adds statement-scoped
temporary loan provenance to the existing loan/liveness stage (the one approved complexity
surface; see below).

## Minimum-Sufficient Baseline

**Original goal:** every accepted program that materializes, stores, calls, or panics with
these runtime values compiles to valid C, runs correctly, and is released exactly once.

**Smallest viable design:** one owning-temporary release rule at statement end (spec D5/O2),
one iterable materialization in the element loop, one branch-aware fixed-array store for
conditionals, receiver guards at the three call sites that miss them, the existing
out-param ABI applied by the shared func-pointer declarator, `cl_panic` widened to pointer
+ length in the three shims, and the existing guard-operand materialization carrying
deferred move-outs. No new type layer, dependency, or harness.

**Excluded machinery:** No separate type layer, dependency, service, or harness. The only
support surface beyond the smallest release is the statement-scoped temporary loan
provenance described below; it is not a new subsystem but an extension of the existing
view provenance/loan records (`own/place.rs`, `own/liveness.rs`).

**Approved complexity:** Statement-scoped temporary ownership in the view/loan model
plus a statement-position release path (machinery approved 2026-09-29 through the gate
packet in `00-preflight-report.md`, PF-002). The release rule alone would free storage a
live derived view points into; the addition makes the release sound without a new
diagnostic (the existing `E10030` loan-lifetime rule applies) and without a new layer.

## Document Index

| #   | Document                                                       | Description                                 |
| --- | -------------------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)                 | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                           | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                             | Feature requirements and scope              |
| 02  | [Current State](02-current-state.md)                           | Analysis of current implementation          |
| 03-01 | [Materialization and Temporary Release](03-01-materialization-and-temps.md) | The iterable, temporary, and conditional fixes |
| 03-02 | [Callable Fields](03-02-callable-fields.md)                  | The receiver guard and func-pointer ABI     |
| 03-03 | [Runtime Boundary](03-03-runtime-boundary.md)                | The panic conversion and deferred move-outs |
| 07  | [Testing Strategy](07-testing-strategy.md)                     | Test cases and verification                 |
| 99  | [Execution Plan](99-execution-plan.md)                         | Phases, sessions, and task checklist        |

## Quick Reference

### Usage Examples

```clorraine
// Accepted: the call result is materialized for the loop and released after it.
func mk(): i32[2] { return [1, 2]; }

func main(): i32 {
    let sum: i32 = 0;
    for (x in mk()) {
        sum = sum + x;
    }
    return sum;                     // 3
}
```

```clorraine
// Accepted: the chosen arm evaluates; the other arm never runs.
let c: i32[2] = ok ? a : b;

// Accepted: a func value with a fixed-array return.
let g: func(i32): i32[2] = pair;
let a: i32[2] = g(3);

// Accepted: a str panic message prints and aborts.
panic(label());
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| `DEF-23` release | Every owning temporary, statement end, reverse creation order (AR-5) |
| `DEF-23` lifetime | A view derived from a statement temporary is a loan of that temporary; a use after its statement reports `E10030` (AR-18) |
| Release mechanism | A dedicated statement-position release (not the scope-collected `Drop`) drains after each statement and before terminators (AR-18) |
| Release set | Destination materializations only; take-call and literal-adoption temporaries are excluded (AR-18) |
| `DEF-24` iterable | Accept and materialize once before the loop (AR-6) |
| `DEF-25` conditional | Chosen-arm branch; copy for duplicable, move for owned arms (AR-7, AR-16) |
| `DEF-25` argument | A place-arm conditional argument stays a forwarding value; a call-arm argument is filed (DEF-32) |
| `DEF-26` receiver | Guard on the statement, fixed-array assign, and return paths (AR-8) |
| `DEF-27` func pointer | Hidden out-param ABI for the filed spellings (fields, locals, parameters, discarded calls) plus call destinations (AR-9); closure structs and lifted lambdas are out of scope and filed (DEF-30) |
| `DEF-28` panic | Runtime takes pointer + length; three shims change together (AR-10, AR-17) |
| `DEF-29` guarded take | Move-out travels on the materialized value (AR-11) |
| Verification | `make check` per task, `make verify` and leak-detection ASan probes at close (AR-14) |

## Related Files

- `compiler/src/own/lower.rs` — guard-operand materialization, call/take lowering, conditional arms
- `compiler/src/own/loops.rs` — the element-loop iterable
- `compiler/src/own/statements.rs`, `own.rs`, `ir.rs` — statement-end release bookkeeping
- `compiler/src/own/place.rs`, `own/liveness.rs` — view provenance and the loan-lifetime rule behind the temporary-view rejection
- `compiler/src/codegen/function.rs` — fixed-array stores, call emission, panic rendering, temp declarations, func-value destinations
- `compiler/src/codegen/guards.rs` — guarded emission and deferred move-outs
- `compiler/src/codegen/types.rs` — func-pointer declarators
- `compiler/src/codegen/cleanup.rs` — fixed-array returns, including the branch-aware return store
- `compiler/runtime/shim_host.c`, `shim_hosted_amiga.c`, `shim_freestanding.c` — the runtime
- `compiler/tests/runtime_value_integrity_spec_test.rs`, `compiler/tests/runtime_value_integrity_impl_test.rs` — new suites
- `compiler/src/own/tests.rs` — unit coverage
