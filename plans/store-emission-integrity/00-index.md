# Store-Emission Integrity Implementation Plan

> **Feature**: Fix the four open major defects (DEF-113, DEF-114, DEF-115, DEF-116) so accepted source compiles, builds, and releases storage correctly.
> **Status**: Done — plan and roadmap rows complete (2026-11-09 closeout).
> **Created**: 2026-10-10 08:02
> **Implements**: DEF-113, DEF-114, DEF-115, DEF-116
> **CodeOps Artifact Schema**: 1

## Overview

The batch closes the four majors filed by the `borrow-store-integrity` preflight and Phase 3
reviews, all verified at the plan baseline with emitted-C and AddressSanitizer probes: a
conditional-result store through a non-nullable lend root that frees the caller's value
(DEF-113), an element write through an immutable `str`/`static str` view that no conforming C
compiler accepts (DEF-114), a typedef emission order that defines slice typedefs before their
element types and struct definitions before enum typedefs (DEF-115), and module initializers that
emit array assignments and unlowered intrinsic calls (DEF-116). Each fix conforms the
implementation to rules the frozen specification already states (BR-5, the Ch 02 §6 mutability
table, the Ch 17 emission contract).

The only specification change is the user-approved `E10042` registry row and its compile-fail
golden (AR-2, AR-12). The DEF-113 judgment reuses the existing per-instance duty machinery; the
DEF-115 fix is an ordering change over existing emission functions; the DEF-116 fix reuses the
established element-store semantics, the intrinsic renderer, and the existing guard arm. No new
harness, dependency, or generalized mechanism is introduced.

`02-current-state.md` records the verified reproduction of each defect; `03-01` – `03-04` own the
designs; `07-testing-strategy.md` owns the ST-cases; the `00-ambiguity-register.md` entries own
the decisions this plan cites as `AR ##`.

## Minimum-Sufficient Baseline

**Original goal:** fix the four defects so accepted source compiles, builds, and behaves correctly,
without reopening the frozen specification beyond the user-authorized E10042 row and golden.
**Smallest viable design:** reuse what each site already has — the suppression predicate and the
existing `RequiresStorableSlot` duty for DEF-113; the ownership layer's write-mutability checks
(`E10023`/`E10024`) and a registry row for DEF-114; the existing emission phase functions,
reordered, for DEF-115; the function-path element-store semantics, the existing intrinsic
renderer, and init-function locals for DEF-116.
**Excluded machinery:** None considered — the rejected alternatives (a new duty record for
DEF-113, forward typedefs inside `emit_slice_typedefs`, a compound-literal or `memcpy` form for
DEF-116, a sema-layer check for DEF-114) were changes to existing paths, not new support
surfaces (AR-4, AR-6, AR-8, AR-9).
**Approved complexity:** None.

## Document Index

| #   | Document                                                    | Description                                        |
| --- | ----------------------------------------------------------- | -------------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)               | Zero-Ambiguity Gate decisions (audit trail)        |
| 00  | [Index](00-index.md)                                         | This document — overview and navigation            |
| 01  | [Requirements](01-requirements.md)                           | Requirements, scope, acceptance criteria           |
| 02  | [Current State](02-current-state.md)                         | Baseline analysis and verified repros              |
| 03-01 | [Conditional-store judgment](03-01-conditional-store-judgment.md) | P1: DEF-113                                |
| 03-02 | [Immutable-view writes](03-02-immutable-view-writes.md)  | P2: DEF-114 (+ the E10042 spec addition)           |
| 03-03 | [Typedef emission order](03-03-typedef-order.md)            | P3: DEF-115                                        |
| 03-04 | [Module initializers](03-04-module-initializers.md)         | P4: DEF-116                                        |
| 05  | [Findings Ledger](05-findings.md)                            | Per-phase review rulings (appended in execution)   |
| 07  | [Testing Strategy](07-testing-strategy.md)                   | ST-cases and verification                          |
| 99  | [Execution Plan](99-execution-plan.md)                       | Phases, sessions, task checklist                   |

## Quick Reference

### Usage Examples

```clorraine
// DEF-113: the conditional store is judged like the direct store
func st(borrow slot: String, pick: bool): void {
    let other: String = stringOf("bbbb");
    slot = pick ? slot : other;   // E10034 once; was accepted and freed the caller's value
}

// DEF-114: immutable views reject element writes (new E10042)
func bad(borrow s: str): i32 {
    s[0] = 65;                    // E10042; use an owned String or a borrow-derived u8[] view
    return 0;
}

// DEF-115: slices of user types and structs with enum fields build
struct P { x: i32; y: i32; }
func first(ps: P[]): i32 { return ps[0].x; }   // ClP is defined before ClSlice_P

// DEF-116: module-level arrays and slice views initialize element-wise
let backing: i32[2] = [1, 2];
let view: i32[] = slice(backing, 0, 2);        // fills backing[0]/backing[1]; view.ptr/.len set
```

### Key Decisions

| Decision | Outcome | AR Ref |
| -------- | ------- | ------ |
| Batch membership | DEF-113, DEF-114, DEF-115, DEF-116 (user choice) | AR-1 |
| DEF-113 mechanism | `lend_moves_judged` excludes the exact-target self arm (mirrors the marking path) | AR-4 |
| DEF-113 reports | Concrete owning → one `E10034` on the target; per-instance duty for parameter-typed; nullable accepted | AR-5 |
| DEF-114 diagnostic | New `E10042` + registry row + golden (user ruling) | AR-2, AR-7, AR-12 |
| DEF-114 coverage | Plain/compound/update forms over params, locals, fields, globals | AR-6 |
| DEF-115 order | Handles, enums, struct forwards, slice typedefs, struct definitions | AR-3, AR-8 |
| DEF-116 emission | Element-wise fills (literal/place/conditional), intrinsic lowering, operand-once | AR-9, AR-10, AR-11, AR-18 |
| Verification | `make check` per phase, host ASan probes for memory cases, `make verify` final | AR-13 |
| Execution mode | `--auto-commit` (user instruction) | AR-16 |

## Specialist Agents

**None** — the capability-gap check ran against this repository's evidence and found no gap a
standing specialist would close.

_Detection evidence: the plan's four surfaces map onto the installed project agents — P1's
lend-store rule and P2's language-rule change onto the correctness, memory-safety, and semantics
reviewers; P3's and P4's emission changes onto the correctness and memory-safety reviewers plus
the emission-integrity auditor (`.opencode/agents/`, dispatch matrix in `AGENTS.md`). The two
findings ledgers present at check time (`plans/borrow-generic-integrity/05-findings.md`,
`plans/borrow-store-integrity/05-findings.md`) show findings are fixed in-batch or filed as
backlog defects; no area recurs beyond the reviewers' scope. The check ran on reduced evidence
(24 of 26 plans carry no findings ledger) and still found no new domain invariant, DSL, or review
checklist beyond the project briefs; candidate creation remains the user's call_

## Related Files

- New: `compiler/tests/store_emission_integrity_spec_test.rs`,
  `compiler/tests/store_emission_integrity_impl_test.rs`, `plans/store-emission-integrity/05-findings.md`
  (the per-phase review ledger, appended during execution),
  `spec/tests/compile-fail/E10042_write_immutable_view.clo` (authorized, AR-12)
- Modified: `compiler/src/own/expr.rs`, `compiler/src/own/place.rs`, `compiler/src/codegen.rs`,
  `compiler/src/codegen/program.rs`, `spec/14-diagnostics.md` (the authorized row only),
  `plans/00-roadmap.md` (row links)
- Referenced by the design (no change expected): `compiler/src/own/instance_check.rs`,
  `compiler/src/own/call.rs`, `compiler/src/codegen/calls.rs`, `compiler/src/codegen/guards.rs`,
  `compiler/src/stdlib.rs`
