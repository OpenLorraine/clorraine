# Closure Integrity Implementation Plan

> **Feature**: Fix the six closure/callable defects (DEF-92, DEF-93, DEF-94, DEF-104, DEF-109, DEF-110) so capture lists, captured owners, and callable values behave per the frozen rules.
> **Status**: Planning Complete
> **Created**: 2026-10-10 10:35
> **Implements**: DEF-92, DEF-93, DEF-94, DEF-104, DEF-109, DEF-110
> **CodeOps Artifact Schema**: 1

## Overview

The v1 lambda surface has four major and two minor holes, all verified at the plan baseline with
emitted-C and AddressSanitizer probes: a nested lambda that relays a capture the enclosing lambda
does not list (DEF-92), a capture list that names a module-level binding (DEF-93), a dotted
capture write that reports `E10024` instead of the write rule (DEF-94), a lambda that returns its
by-value non-duplicable capture and double-frees (DEF-104), a called func-typed capture whose name
matches a module lender and leaks its owned result (DEF-109), and a call of an unbounded type
parameter that emits invalid C (DEF-110).

Every fix conforms the implementation to rules the frozen specification already states (L-2, L-3,
L-4, the `E10405` listing rule, G-1/G-6); the only specification change is the two user-authorized
diagnostic rows `E10406` (a module-level capture entry) and `E10307` (a call of an unbounded type
parameter) with their goldens (AR-2, AR-3, AR-12). The fixes reuse established mechanisms — the
capture-entry validation, the place root, the per-instance duty audit, and the callee-name
masking; no new harness, dependency, or generalized mechanism is introduced.

`02-current-state.md` records the verified reproduction of each defect; `03-01` – `03-03` own the
designs; `07-testing-strategy.md` owns the ST-cases; the `00-ambiguity-register.md` entries own
the decisions this plan cites as `AR ##`.

## Minimum-Sufficient Baseline

**Original goal:** fix the six closure/callable defects so accepted source compiles, builds, and
releases storage correctly, without changing the language surface beyond the two authorized
diagnostic rows.
**Smallest viable design:** reuse what each site already has — the capture-entry validation and
the scope floors for DEF-92/93; the existing `place_root_name` for DEF-94; the existing E10401
rule plus the established per-instance duty pattern for DEF-104; the existing masking path for
DEF-109; the existing callee typing for DEF-110.
**Excluded machinery:** None considered — the rejected alternatives (implicit capture relay,
environment-owned captures, capture support for module names) were changes to existing rules, not
new support surfaces (AR-2, AR-3, AR-5, AR-9).
**Approved complexity:** None.

## Document Index

| #    | Document | Description |
| ---- | -------- | ----------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md) | This document — overview and navigation |
| 01  | [Requirements](01-requirements.md) | Requirements, scope, acceptance criteria |
| 02  | [Current State](02-current-state.md) | Baseline analysis and verified repros |
| 03-01 | [Capture listing and validity](03-01-capture-listing-and-validity.md) | P1: DEF-92, DEF-93, DEF-94 |
| 03-02 | [Captured-owner integrity](03-02-captured-owner-integrity.md) | P2: DEF-104, DEF-109 |
| 03-03 | [Callable bounds](03-03-callable-bounds.md) | P3: DEF-110 |
| 05  | [Findings Ledger](05-findings.md) | Per-phase review rulings (appended in execution) |
| 07  | [Testing Strategy](07-testing-strategy.md) | ST-cases and verification |
| 99  | [Execution Plan](99-execution-plan.md) | Phases, sessions, task checklist |

## Quick Reference

### Usage Examples

```clorraine
// DEF-92: relayed captures are listed at each level
let n: i32 = 2;
let m: i32 = 3;
([n, m] () => ([m] () => m)())();     // accepted

// DEF-93: module names need no capture
const base: i32 = 7;
() => base;                            // accepted; [base] () => base is E10406

// DEF-94: dotted capture writes report the write rule
([p] () => { p.x = 2; })();            // E10402 — add [borrow p] to write through

// DEF-104: a returned owning capture is a move
func outer<T>(x: T): T { return ([x] () => x)(); }   // E10401 at an owning instance

// DEF-109: a captured func-typed call resolves to the capture
let run: func(borrow String): String = run;
([run] () => run(stringOf("ab")))();   // accepted, result released once

// DEF-110: callability needs a bound
func h<T: func(i32): i32>(x: T, n: i32): i32 { return x(n); }   // accepted
func g<T>(x: T): i32 { return x(5); }                            // E10307
```

### Key Decisions

| Decision | Outcome | AR Ref |
| -------- | ------- | ------ |
| Batch membership | six defects (DEF-92, 93, 94, 104, 109, 110) | AR-1 |
| Module-level capture entries | rejected with new `E10406` | AR-2, AR-8 |
| Unbounded type-parameter call | rejected with new `E10307` | AR-3 |
| Relay report | reuse `E10405` on the entry | AR-4, AR-9 |
| Captured-owner return | rejected per `E10401` (direct and per instance) | AR-5, AR-11 |
| Capture name mask | masked IR name; source name kept for diagnostics | AR-7, AR-10 |
| Diagnostics landing set | registry row + golden + driver registration | AR-12 |
| Verification | `make check` per phase, host ASan probes for P2, `make verify` final | AR-13 |
| Execution mode | `--auto-commit` + `--auto-design` (user instruction) | — |

## Specialist Agents

**None** — the capability-gap check ran against this repository's evidence and found no gap a
standing specialist would close.

_Detection evidence: the plan's three surfaces map onto the installed project agents — P1's
capture-rule and diagnostic changes onto the correctness and semantics reviewers, P2's
ownership/spelling changes onto the correctness reviewer and the memory-safety auditor, and P3's
checker rejection onto the correctness and semantics reviewers (`AGENTS.md` dispatch matrix,
`.opencode/agents/`). The three findings ledgers present at check time
(`plans/borrow-store-integrity/05-findings.md`, `plans/store-emission-integrity/05-findings.md`,
`plans/borrow-generic-integrity/05-findings.md`) show closure findings fixed in-batch or filed as
backlog defects; no area recurs beyond the reviewers' scope. The check ran on reduced evidence
(25 of 28 plans carry no findings ledger) and still found no new domain invariant, DSL, or review
checklist beyond the project briefs; candidate creation remains the user's call._

## Related Files

- New: `compiler/tests/closure_integrity_spec_test.rs`,
  `compiler/tests/closure_integrity_impl_test.rs`, `plans/closure-integrity/05-findings.md` (the
  per-phase review ledger, appended during execution),
  `spec/tests/compile-fail/E10406_module_capture.clo`,
  `spec/tests/compile-fail/E10307_unbounded_call.clo` (authorized, AR-2/AR-3)
- Modified: `compiler/src/sema/expr.rs`, `compiler/src/sema/mod.rs`,
  `compiler/src/sema/call.rs`, `compiler/src/own/lower.rs`, `compiler/src/own.rs`,
  `compiler/src/own/instance_check.rs`, `compiler/tests/compile_fail_spec_test.rs`,
  `spec/14-diagnostics.md` (the two authorized rows), `plans/00-roadmap.md` (row links)
- Referenced by the design (no change expected): `compiler/src/own/call.rs`,
  `compiler/src/codegen/expr.rs`, `compiler/src/codegen/program.rs`,
  `compiler/src/codegen/function.rs`
