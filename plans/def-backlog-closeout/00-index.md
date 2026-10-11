# Def Backlog Closeout Implementation Plan

> **Feature**: Close the seven remaining filed defects DEF-63 – DEF-69 in the Clorraine compiler (a
> guarded member-root take leaking the call result's siblings, an aggregate-literal consume losing
> the root call and index, a nullable `borrow` out-slot lowered by value, an element read through a
> slice view over a statement temporary, an ownerless call result consumed as a borrow inside a
> larger expression, a do-while deferred return leaking the condition temporary, and an unlisted
> lambda capture emitting undeclared C)
> **Status**: Complete — executed and verified 2026-10-04 … 2026-10-12 (44/44 tasks; `make verify` all legs passed; close-out review lenses clean; CFN-1 filed as DEF-255)
> **Created**: 2026-10-03 22:53
> **Implements**: DEF-63, DEF-64, DEF-65, DEF-66, DEF-67, DEF-68, DEF-69
> **CodeOps Artifact Schema**: 1

## Overview

The def-backlog-completion plan closed seven defects and filed six more; its close-out and the
gate work around it added a seventh. This plan closes the remaining filed backlog: seven defects
that share three machinery threads and two runtime shapes.

Four defects are move-out/ownership gaps: a guarded take whose call-rooted projection never gets a
place (DEF-63), an aggregate-literal consume that discards the root call and the runtime index
(DEF-64), a nullable `borrow` out-slot that is passed by value so the callee's write never reaches
the caller (DEF-65), and an element read through a slice view over a temporary (DEF-66). Two are
missing releases: an ownerless call result used as a borrow argument inside a larger expression
(DEF-67) and a do-while condition temporary that a `defer`-registered return never releases
(DEF-68). One is a front-end gap: a lambda reading an enclosing binding that is not in its capture
list compiles and emits undeclared C (DEF-69), fixed by a new diagnostic.

Six of the seven fixes reuse existing machinery — the deferred `Materialize` carrier, the
store/move-out preparation, the owned-argument binding, the pointer-parameter predicate, and the
return-release append. Only DEF-69 changes the frozen specification: one approved registry row
(`E10405`) with its compile-fail golden. DEF-66 is treated as a reproduction gate first, because a
later base-settlement fix may already cover its generic paths.

## Minimum-Sufficient Baseline

**Original goal:** Make every filed DEF-63 – DEF-69 reproduction behave correctly: no leak, no
double free, no use-after-free, no invalid C, and no silently wrong slot — with DEF-69 rejected by
the approved `E10405`.
**Smallest viable design:** bind a guarded call-rooted take through the existing deferred
`Materialize` temp so the current place derivation and guard zeroing apply; prepare the
aggregate-literal consume path like the existing store path; key the pointer lowering on
`mode == Borrow && ty.nullable` in the shared declaration predicate and on the substituted
parameter type at the call site, with an exact-typed place required for an out-slot argument; route
intrinsic arguments and dot-`len` receivers through the existing owned-argument binding; extend the
do-while condition-release walk to the registered defer blocks; and add one scope-floor check in
the lambda checker with the approved diagnostic. No new layer, dependency, harness, or crate.
**Excluded machinery:** expression-wide materialization of every owned call result (AR-6); a
separate guard-root registry (AR-8); converting all `borrow` parameters to pointers (AR-3); capture
inference (AR-4); a new harness or runner beyond the repository's existing probe-runner pattern
(AR-15).
**Approved complexity:** None — every fix reuses existing machinery except the approved
specification row for `E10405` (AR-4, AR-13).

## Document Index

| #   | Document | Description |
| --- | -------- | ----------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md) | This document — overview and navigation |
| 01  | [Requirements](01-requirements.md) | Feature requirements and scope |
| 02  | [Current State](02-current-state.md) | Analysis of the current implementation and the filed reproductions |
| 03-01 | [Take and Move-Out Integrity](03-01-take-moveout-integrity.md) | DEF-63, DEF-64, DEF-66 technical specification |
| 03-02 | [Expression Temporaries](03-02-expression-temporaries.md) | DEF-67 technical specification |
| 03-03 | [Borrow Out-Slot ABI](03-03-borrow-outslot-abi.md) | DEF-65 technical specification |
| 03-04 | [Defer Condition Release](03-04-defer-condition-release.md) | DEF-68 technical specification |
| 03-05 | [Capture Diagnostic](03-05-capture-diagnostic.md) | DEF-69 technical specification |
| 07  | [Testing Strategy](07-testing-strategy.md) | Specification cases and verification |
| 99  | [Execution Plan](99-execution-plan.md) | Phases, sessions, and task checklist |

## Quick Reference

### Key Decisions

| Decision | Outcome | AR |
| -------- | ------- | -- |
| Plan scope | All seven defects, one plan | AR-1 |
| DEF-66 | Reproduce first; regression pins or fix | AR-2 |
| DEF-65 width | Nullable out-slots only | AR-3 |
| DEF-69 | Reject unlisted reads with E10405 | AR-4, AR-13, AR-14 |
| Nested captures | List at each level | AR-5 |
| DEF-67 | Bind intrinsic args and dot-`len` receivers | AR-6 |
| DEF-68 | Collect defer blocks in the condition-release walk | AR-7 |
| Workflow | Task-time ASan probes, blast-radius packets, probe runner | AR-15 |

### Usage Examples

The plan's target shapes; all seven currently misbehave and must be clean after execution.

```clorraine
flag && sink(mkv().a);                      // DEF-63: siblings of mkv() leak
Box<T> { v: mk()[i] }                       // DEF-64: root call lost, wrong slot
fill(take v: T, borrow slot: String?);     // DEF-65: non-null slot double-frees
let s: T = slice(mk(), 0, 2)[i];            // DEF-66: element read after release
return i32(len(stringOf("abc")));           // DEF-67: result never released
do { defer { return i; } } while (guarded); // DEF-68: condition temp leaks
[x] () => n;                                // DEF-69: undeclared 'n' in C
```

## Related Files

- `compiler/src/own/lower.rs`, `compiler/src/own/call.rs`, `compiler/src/own/loops.rs` — DEF-63/64/66/67/68 fixes.
- `compiler/src/codegen/types.rs`, `compiler/src/codegen/calls.rs` — DEF-65 fix.
- `compiler/src/sema/expr.rs`, `compiler/src/sema/mod.rs` — DEF-69 fix.
- `spec/14-diagnostics.md`, `spec/tests/compile-fail/E10405_unlisted_capture.clo` — the approved specification change.
- `compiler/tests/def_backlog_closeout_{spec,impl}_test.rs` — the new suites.
- `build/def-backlog-closeout-probes/run-def-backlog-closeout.sh` — the close-out probe runner (gitignored).
