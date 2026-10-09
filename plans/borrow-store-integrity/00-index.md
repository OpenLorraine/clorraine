# Borrow-Store Integrity Implementation Plan

> **Feature**: Fix the five open write-path defects (DEF-100, DEF-101, DEF-102, DEF-105, DEF-106) to the frozen specification's rules.
> **Status**: Planning Complete
> **Created**: 2026-10-10 00:34
> **Implements**: DEF-100, DEF-101, DEF-102, DEF-105, DEF-106
> **CodeOps Artifact Schema**: 1

## Overview

The batch closes five pre-existing defects filed by the `borrow-generic-integrity` reviews, all on
the write or move path *through a borrowed place*: a whole-binding self-assign through a
parameter-typed lend root (DEF-100), a direct slot return that never empties the caller's slot
(DEF-101), a runtime-indexed element self-assign on a nullable out-slot (DEF-102), an element
store through a possibly-null view (DEF-105), and a constant-index element store without its bounds
trap (DEF-106). Each fix implements an existing rule — it adds no language surface and changes no
frozen chapter (AR-10).

DEF-102 changes no semantics: the filed "unguarded" premise does not reproduce at the plan
baseline; the plan pins the conservative comparison that already rejects every same-root
runtime-indexed element pair (AR-4, user ruling). DEF-101 follows the user's move-out ruling: the
return transfers the slot's value and empties the caller's slot, matching the accepted
`let local = v` store (AR-3).

`02-current-state.md` records the verified reproduction of each defect at the plan baseline;
`03-01` through `03-03` own the designs; `07-testing-strategy.md` owns the ST-cases; the
`00-ambiguity-register.md` entries own the decisions this plan cites as `AR ##`.

## Minimum-Sufficient Baseline

**Original goal:** fix the five defects so accepted source compiles, runs, and releases storage
correctly, without reopening the frozen specification.
**Smallest viable design:** reuse the machinery each site already uses — the established
per-instance duty pattern (`ir::Requires*` records plus the `own/instance_check.rs` audits) for
DEF-100; the read path's null check (`check_nonnull_expr`) and the existing narrowing state for
DEF-105; the existing bounds-guard helpers (`bounds_guard_condition` / `emit_bounds_guard`) behind
one small guard-carrying statement for DEF-106; and the existing source-emptying step
(`emit_arm_source_moves` / `emit_move_zero_place`) in the return branch for DEF-101.
**Excluded machinery:** None considered — the rejected alternatives (an index-identity comparison
for DEF-102, routing constant-index stores through `StorePointer` for DEF-106, a runtime trap for
DEF-105) were changes to existing paths, not new support surfaces (AR-4, AR-5, AR-6).
**Approved complexity:** None.

## Document Index

| #   | Document                                            | Description                                 |
| --- | --------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)      | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                  | Requirements, scope, acceptance criteria    |
| 02  | [Current State](02-current-state.md)                | Baseline analysis and verified repros       |
| 03-01 | [Self-assign rules](03-01-self-assign-rules.md)   | P1: DEF-100, DEF-102                        |
| 03-02 | [Out-slot return](03-02-out-slot-return.md)       | P2: DEF-101                                 |
| 03-03 | [View-store guards](03-03-view-store-guards.md)   | P3: DEF-105, DEF-106                        |
| 07  | [Testing Strategy](07-testing-strategy.md)          | ST-cases and verification                   |
| 99  | [Execution Plan](99-execution-plan.md)              | Phases, sessions, task checklist            |

## Quick Reference

### Usage Examples

```clorraine
// DEF-100: the parameter-typed self-assign is judged per instance
func copySelf<T>(borrow slot: T): void {
    slot = slot;               // instance String -> E10034, String? -> E10020, i32 -> accepted
}

// DEF-102: the runtime-indexed element self-assign stays rejected (conservative)
if (xs != null) { xs[i] = xs[i]; }   // E10020; xs[1] = xs[0] stays accepted

// DEF-101: the direct slot return transfers and empties
func cid(borrow v: String?): String? {
    return v;                  // caller's slot is null afterwards; ASan-clean
}

// DEF-105: a store through a possibly-null view needs the narrowing (like a read)
func f(borrow xs: u8[]?): i32 {
    xs[0] = 1;                 // E10010 unless checked
    if (xs != null) { xs[0] = 1; }   // accepted
    return 0;
}

// DEF-106: a constant-index slice store traps like the read path
func g(borrow xs: i32[]?): i32 {
    if (xs != null) { xs[0] = 1; }   // traps E11211 when the view is empty
    return 0;
}
```

### Key Decisions

| Decision | Outcome | AR Ref |
| -------- | ------- | ------ |
| Batch membership | DEF-100, DEF-101, DEF-102, DEF-105, DEF-106 (user choice) | AR-1 |
| DEF-100 mechanism | New `RequiresSelfAssign` duty following the `Requires*` pattern | AR-2 |
| DEF-100 diagnostics | Owning → `E10034`; nullable → `E10020`; duplicable/view → accepted | AR-7 |
| DEF-101 direction | Move out: the return stores the value and empties the caller's slot | AR-3 |
| DEF-102 resolution | Keep the conservative runtime-index guard; pin and document it | AR-4 |
| DEF-105 rule | Compile-time `E10010`, mirroring the read path | AR-5 |
| DEF-106 emission | The read path's `E11211` trap before the store, carried by a `Guards` statement | AR-6 |
| Frozen spec | `spec/` stays byte-identical | AR-10 |
| Verification | `make check` per phase, ASan probes for the memory phases (`RDC_ASAN`), `make verify` final | AR-11 |

## Specialist Agents

**None** — the capability-gap check found no gap a standing specialist would close.

_Detection evidence: every phase surface maps onto an installed project agent — P1's self-assign
rules onto the correctness, memory-safety, and semantics reviewers; P2's return emission and P3's
IR/emission change onto the correctness and memory-safety reviewers plus the emission-integrity
auditor (`.opencode/agents/`, dispatch matrix in `AGENTS.md`); the on-demand `analyze-agents` check
ran on 2026-10-09 21:09 with the same outcome (`None`, reduced evidence: no findings ledgers), and
this plan's surfaces introduce no new domain invariant, DSL, or review checklist beyond those
briefs._

## Related Files

- New: `compiler/tests/borrow_store_integrity_spec_test.rs`,
  `compiler/tests/borrow_store_integrity_impl_test.rs`
- Modified: `compiler/src/ir.rs`, `compiler/src/ir/mono.rs`, `compiler/src/own.rs`,
  `compiler/src/own/expr.rs`, `compiler/src/own/instance_check.rs`,
  `compiler/src/codegen/cleanup.rs`, `compiler/src/codegen/function.rs`
- Referenced by the design (no change expected): `compiler/src/own/place.rs`,
  `compiler/src/codegen/guards.rs`, `compiler/src/ir/place.rs`
