# Def Backlog Completion Implementation Plan

> **Feature**: Complete the seven open defects DEF-56 – DEF-62 in the Clorraine compiler (a generic
> lend escaping as owned, a generic lend passed to a take position, a specialized capturing
> closure's callable-bound type spelling, a zero-argument closure call's trailing comma, an
> ownerless element read under a maybe-owning substitution, a runtime-index element move-out, and a
> do-while early return leaking the condition temporary)
> **Status**: Complete
> **Created**: 2026-10-03 08:09
> **Implements**: DEF-56, DEF-57, DEF-58, DEF-59, DEF-60, DEF-61, DEF-62
> **CodeOps Artifact Schema**: 1

## Overview

The residual-defect-completion plan closed six defects and filed seven more while reviewing and
probing its fixes. Every filed defect is a place where a value crosses a C boundary without the
treatment its type, ownership, or instance requires. Each was reproduced against the committed
compiler before this plan was written, and one filed remedy was corrected during discovery: the
guarded-take double free is actually the runtime-index element move-out (see AR-1 and
[02-current-state.md](02-current-state.md)). Preflight then split the close-out evidence between
five run probes and two rejection probes (PF-011) and filed one further distinct residual, the
aggregate-literal consume, as DEF-64 (PF-009).

The seven defects fall into three machinery threads. Generic lend duties (DEF-56, DEF-59, DEF-61,
DEF-60) all stem from checks that cannot judge a type parameter before substitution or from a
move-out that loses its element index. The guard/loop release thread (DEF-62) is a release that never
reaches an early return. The closure C spelling thread (DEF-57, DEF-58) is the emitter disagreeing
with the recorded instance type and one argument-list separator.

None of the seven needs a new language surface, a new diagnostic, or a new subsystem. The fixes
reuse the existing duplicability-duty audit, the statement-oriented store/move-out arms, the return
release pipeline, the instance-key renderer, and the closure call renderer. The frozen specification
under `spec/` is the oracle; this plan changes no specification text and adds no diagnostic code.

## Minimum-Sufficient Baseline

**Original goal:** Make the seven filed reproductions behave correctly: no leak, no double free, no
use-after-free, no invalid C, and no silently wrong element — with the two generic-lend escapes
rejected by the existing `E10034` and the five remaining reproductions running sanitizer-clean
(PF-011).
**Smallest viable design:** Record a post-substitution `E10034` duty where a maybe-owning lend is
moved (centralized in `mark_moved_if_owning`); carry the element index through the move-out and apply
the existing copy-and-zero store arm to statement-owned destination temps and the emitter's
no-ladder return branch; append the do-while condition's newly registered releases to the body's
return terminators; stamp the enclosing instance key on recorded closure type arguments (including it
in `type_key`); and omit the separator for an empty closure argument list. No new layer, dependency,
harness, or crate.
**Excluded machinery:** The guard-deferred take-root binding (AR-1) — it fixes a different, guarded
member-root leak and is filed as a new backlog defect. The `ClosureTy` enclosing-argument
representation (AR-3). The lower-layer return move-out (AR-5). A spec change or a new diagnostic
(AR-9).
**Approved complexity:** None — every fix reuses existing machinery.

## Document Index

| #   | Document | Description |
| --- | -------- | ----------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md) | This document — overview and navigation |
| 01  | [Requirements](01-requirements.md) | Feature requirements and scope |
| 02  | [Current State](02-current-state.md) | Analysis of the current implementation and the confirmed reproductions |
| 03-01 | [Generic Lend Duties](03-01-generic-lend-duties.md) | DEF-56 and DEF-61 technical specification |
| 03-02 | [Move-Out Integrity](03-02-move-out-integrity.md) | DEF-60 and DEF-59 technical specification |
| 03-03 | [Do-While Condition Release](03-03-do-while-condition-release.md) | DEF-62 technical specification |
| 03-04 | [Closure C Spelling](03-04-closure-c-spelling.md) | DEF-57 and DEF-58 technical specification |
| 07  | [Testing Strategy](07-testing-strategy.md) | Specification cases and verification |
| 99  | [Execution Plan](99-execution-plan.md) | Phases, sessions, and task checklist |

## Quick Reference

### Reproductions

Each defect has a minimal reproduction that fails on the committed compiler; the exact emitted
failure and root-cause sites are in [02-current-state.md](02-current-state.md) §Reproductions.

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Generic lend returned/stored/passed as owned | Reject at instantiation with the existing `E10034`, recorded as a post-substitution duty (AR-2) |
| Runtime-index element move-out | Carry the taken element's index through to the zeroed move-out slot (AR-1) |
| Ownerless element read store/return | Store through `PreparedStore::Move`; return through the emitter's no-ladder branch (AR-5) |
| Do-while early return | Append the condition's pending releases to the body's return terminators (AR-4) |
| Specialized closure argument type | Stamp the enclosing instance key when recording the argument and put it in `type_key` (AR-3) |
| Zero-argument closure call | Omit the separator when the argument list is empty (AR-3 family) |

## Related Files

| File | Role in this plan |
| ---- | ----------------- |
| `compiler/src/own/place.rs` | Central maybe-owning move site; the new lend duty (DEF-56, DEF-61) |
| `compiler/src/own/statements.rs`, `compiler/src/own/call_check.rs` | Maybe-owning return/take branches; store preparation (DEF-56, DEF-59, DEF-61) |
| `compiler/src/ir/mono.rs`, `compiler/src/own.rs`, `compiler/src/own/instance_check.rs` | Duty recording, collection, and post-substitution audit |
| `compiler/src/ir/place.rs`, `compiler/src/ir.rs` | The `ElementTemp` variant and the builder release-append mutator |
| `compiler/src/own/lower.rs`, `compiler/src/own/call.rs` | Element places, index materialization, and take move-out (DEF-60) |
| `compiler/src/codegen/expr.rs` | Element rendering; closure call renderer (DEF-58) |
| `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/function.rs` | Return emission and arm source moves (DEF-59) |
| `compiler/src/own/loops.rs` | Do-while lowering and the condition release append (DEF-62) |
| `compiler/src/own/generics.rs`, `compiler/src/types/` | Instance collection and the stage-neutral key renderer (DEF-57) |
| `compiler/src/codegen/types.rs`, `compiler/src/codegen.rs` | Closure spelling and `type_key` (DEF-57) |
| `compiler/tests/def_backlog_completion_{spec,impl}_test.rs` | The plan's specification and implementation suites |
