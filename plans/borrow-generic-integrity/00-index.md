# Borrow/Generic Integrity Implementation Plan

> **Feature**: Fix the six open borrow/generic defects (DEF-77, DEF-78, DEF-79, DEF-81, DEF-82, DEF-83) to the frozen specification's rules.
> **Status**: Planning Complete
> **Created**: 2026-10-09 20:23
> **Implements**: DEF-77, DEF-78, DEF-79, DEF-81, DEF-82, DEF-83
> **CodeOps Artifact Schema**: 1

## Overview

The batch closes six pre-existing defects filed by the `def-backlog-closeout` reviews, all in the
borrow/out-slot and generic-instance machinery. Each fix implements an existing rule — it adds no
language surface and changes no frozen chapter (AR-10):

- a whole-binding store into a lend parameter frees the caller's storage (DEF-78);
- an out-slot self-assignment empties the caller's slot (DEF-82);
- a spurious `W10001` fires on a nullable out-slot (DEF-83);
- struct instance keys ignore nullability, so two instances share one C symbol (DEF-79);
- a nullable view cannot be compared with `null`, so it cannot be narrowed (DEF-81);
- a func-typed parameter named like a module lender makes the may-lend gate leak the owned
  result (DEF-77).

`02-current-state.md` records the verified reproduction of each defect at the plan baseline;
`03-01` through `03-04` own the designs; `07-testing-strategy.md` owns the ST-cases; the
`00-ambiguity-register.md` entries own the decisions this plan cites as `AR ##`.

## Minimum-Sufficient Baseline

**Original goal:** fix the six defects so accepted source compiles, runs, and releases storage
correctly, without reopening the frozen specification.
**Smallest viable design:** reuse the machinery each site already uses — the established
per-instance duty pattern (`ir::Requires*` records plus the `own/instance_check.rs` audits), the
existing diagnostics (`E10034`, `E10020`, `W10001`), the existing callee-name masking guard, and
the existing type-key function. Every change lands in an existing function or an existing audit,
except the one approved internal addition: the `ir::RequiresStorableSlot` duty record and its
audit alongside the existing audits (AR-5; PF-012).
**Excluded machinery:** None considered beyond those patterns — no new diagnostic codes, no
generalized store-policy layer, no new verification harness (the host ASan probes and the strict
build checks reuse the repo's existing tooling).
**Approved complexity:** None.

## Document Index

| #   | Document                                            | Description                                 |
| --- | --------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)      | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                  | Requirements, scope, acceptance criteria    |
| 02  | [Current State](02-current-state.md)                | Baseline analysis and verified repros       |
| 03-01 | [Borrow-store rules](03-01-borrow-store-rules.md) | P1: DEF-78, DEF-82, DEF-83                  |
| 03-02 | [Instance keys](03-02-instance-keys.md)           | P2: DEF-79                                  |
| 03-03 | [Nullable view comparison](03-03-nullable-view-comparison.md) | P3: DEF-81                  |
| 03-04 | [May-lend callee identity](03-04-may-lend-callee-identity.md) | P4: DEF-77                  |
| 07  | [Testing Strategy](07-testing-strategy.md)          | ST-cases and verification                   |
| 99  | [Execution Plan](99-execution-plan.md)              | Phases, sessions, task checklist            |

## Quick Reference

### Usage Examples

```clorraine
// DEF-78: a store into a lend parameter is now rejected (was: double free)
func fill(take v: String, borrow slot: String): void {
    slot = v;                  // E10034
}

// DEF-82: out-slot self-assignment is rejected (was: the caller's slot emptied)
func self(borrow slot: String?): void {
    slot = slot;               // E10020
}

// DEF-83: no W10001 for a nullable out-slot (was: spurious warning)
func set(borrow x: raw u8?): void { x = null; }

// DEF-79: the two instances get distinct C types (was: redefinition error)
struct Wrap<T> { v: T; }
let a: Wrap<String> = Wrap<String> { v: stringOf("x") };
let b: Wrap<String?> = Wrap<String?> { v: stringOf("y") };

// DEF-81: a nullable view narrows (was: E10222)
if (xs != null) { return i32(xs[0]); }

// DEF-77: a func-typed parameter named like a module lender binds its result (was: leak)
func demo(give: func(borrow String): String): i32 {
    return i32(len(give(s)));
}
```

### Key Decisions

| Decision | Outcome | AR Ref |
| -------- | ------- | ------ |
| DEF-78 rule and the corrected scope | Declaration-site rejection of owning stores into lend params + a per-instance duty; nullable out-slots and duplicable lends stay legal | AR-2, AR-13 |
| DEF-82 out-slot self-assignment | Reject with `E10020`, same as locals — whole-binding and exact projected self-assign | AR-8, PF-005 |
| DEF-83 warning exemption | Every nullable borrow (out-slot) parameter is exempt | AR-9 |
| DEF-79 key surface | Nullability folds into `type_key`; the text-conversion spelling is preserved; top-level function-instance symbols stay identical (nested nullable arguments gain `Q`) | AR-4, AR-6, PF-003 |
| DEF-81 comparison rule | View-vs-`null` only, either order; slice-view `==` slice-view stays `E10222` | AR-7, PF-010 |
| DEF-77 callee identity | Extend the callee-name masking guard to every binding kind, with the parameter-name plumbing through the IR and emitter | AR-3, PF-001 |
| Frozen spec | `spec/` stays byte-identical | AR-10 |
| Verification | `make check` per phase, host-ASan probes for P1 and P4 (`RDC_ASAN`), strict build check for P2, `make verify` final | AR-12, PF-007 |

## Specialist Agents

**None** — the capability-gap check found no gap a standing specialist would close.

_Detection evidence: every phase surface maps onto an installed project agent — P1's store rules
onto the correctness and memory-safety reviewers, P2's symbol/emission change onto the
emission-integrity auditor, P3's operator rule onto the semantics reviewer, and P4's borrow gate
plus parameter-name/emission change onto the correctness, memory-safety, and emission-integrity
reviewers (`.opencode/agents/`, dispatch matrix in `AGENTS.md`); the on-demand `analyze-agents`
check ran again on 2026-10-09 21:09 — now including this plan — with the same outcome (`None`,
reduced evidence: no findings ledgers) and is recorded in `codeops/specialist-check.json`. No new
domain invariant, DSL, or review checklist was identified beyond those briefs._

## Related Files

- New: `compiler/tests/borrow_generic_integrity_spec_test.rs`,
  `compiler/tests/borrow_generic_integrity_impl_test.rs`
- Modified: `compiler/src/own/expr.rs`, `compiler/src/own/instance_check.rs`,
  `compiler/src/own/call_check.rs`, `compiler/src/own/call.rs`,
  `compiler/src/ir/mono.rs`, `compiler/src/ir.rs`, `compiler/src/own.rs`,
  `compiler/src/types/keys.rs`, `compiler/src/codegen/types.rs`,
  `compiler/src/codegen.rs`, `compiler/src/codegen/function.rs`,
  `compiler/src/codegen/expr.rs`, `compiler/src/sema/ops.rs`
- Conditional: `compiler/src/own/place.rs` — only if the dedupe needs the shared
  release/store helper (AR-13)
- Shared by P2 and P4: `compiler/src/codegen/expr.rs` (P4 carries the masked parameter
  spelling through the call/place lookups; PF-001)
