# Evaluation and Emission Integrity Implementation Plan

> **Feature**: Close 24 defects across module-initializer emission, strict-build emission shapes, checker conformance, write-mutability rules, and ownership releases, plus the DEF-103 documented-limitation closure.
> **Status**: Planning Complete
> **Created**: 2026-10-10 12:06
> **Implements**: DEF-95, DEF-96, DEF-97, DEF-98, DEF-99, DEF-103 (closure), DEF-107, DEF-108, DEF-111, DEF-112, DEF-117, DEF-118, DEF-119, DEF-120, DEF-121, DEF-122, DEF-124, DEF-125, DEF-126, DEF-127, DEF-128, DEF-129, DEF-76, DEF-80, DEF-90
> **CodeOps Artifact Schema**: 1

## Overview

The batch closes the evaluation/emission majors and the ruled items from the store-emission,
defer-integrity, borrow-generic-integrity, slice-temporary-integrity, and closure-integrity
reviews, plus the minor sweep and the five fresh rows filed by batch A. Five layers: (1) module
initializers that emit destination-less fixed-array calls, double-evaluate guarded operands, or
accept backing-less slice literals; (2) emitted C shapes failing the strict build (structural
`len` bindings, struct compound literals, the freestanding `bool`, the extern view coercion,
generated-name collisions, slice typedef order, the bounded-field call); (3) checker judgments the
frozen rules already decide (view equality, `str` loops, bare-parameter member calls) plus the
null inference and the generic projection fold; (4) the two user rulings (value-position write
checks, `cstring` element writes); (5) five ownership/release holes (view-bound temporary,
defer-preempted return, value-call temporary, block-bodied capture return, conditional capture
moves). DEF-103 closes as the documented, fail-closed key-marker limitation with no code change.

The only spec edits are the approved diagnostics: the new `E10043` row with its golden, the
`E10042` trigger wording, and the `E10212` trigger (AR-B6, AR-B11). `02-current-state.md` carries
the `file:line` mapping per gap (three read-only recon passes); `03-01` – `03-05` own the designs;
`07-testing-strategy.md` owns the 41 ST-cases; the `00-ambiguity-register.md` entries own the
decisions cited as `AR-B##`.

## Minimum-Sufficient Baseline

**Original goal:** accepted programs build under the strict flags and behave per the frozen rules;
the reviewed memory holes stop leaking or double-freeing; the ruled write checks land.
**Smallest viable design:** every fix takes the smallest recorded lever from the recon fact
sheets — existing contracts are mirrored (the function path's hidden destination, the
materialized-operand contract, the supersede gate, the owner machinery, the duty audits), and the
checker rejections reuse the existing diagnostics except the one approved `E10043` row.
**Excluded machinery:** None considered — the larger alternatives (static backing storage,
escaping key scheme, `str`-loop support, env-owned captures) were user-declined with their own
triggers recorded.
**Approved complexity:** None.

## Document Index

| #    | Document | Description |
| ---- | -------- | ----------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md) | This document — overview and navigation |
| 01  | [Requirements](01-requirements.md) | Requirements, scope, acceptance criteria |
| 02  | [Current State](02-current-state.md) | Baseline analysis and `file:line` mapping |
| 03-01 | [Module-initializer integrity](03-01-module-initializer-integrity.md) | P1: DEF-117, 121, 122 |
| 03-02 | [Strict-build emission shapes](03-02-strict-build-emission.md) | P2: DEF-76, 97, 80, 90, 98, 112, 120, 128 |
| 03-03 | [Value typing and checker conformance](03-03-value-typing-conformance.md) | P3: DEF-99, 111, 108, 107, 118, 129 |
| 03-04 | [Write-mutability rules](03-04-write-mutability-rules.md) | P4: DEF-119, 124 |
| 03-05 | [Ownership lifetimes and releases](03-05-ownership-lifetimes.md) | P5: DEF-95, 96, 125, 126, 127 |
| 05  | [Findings Ledger](05-findings.md) | Per-phase review rulings (appended in execution) |
| 07  | [Testing Strategy](07-testing-strategy.md) | ST-cases and verification |
| 99  | [Execution Plan](99-execution-plan.md) | Phases, sessions, task checklist |

## Quick Reference

### Usage Examples

```clorraine
// P1: init positions build; the guard runs once; the literal slice rejects
let a: i32[2] = mk();                 // hidden-destination emission
let v: u8 = s[idx()];                 // idx() once
let x: i32[] = [1, 2];                // E10200

// P2: strict builds
func g<T>(x: T): i32 { return i32(len(mkArr(x))); }        // no unused local
func empty(): Pair { return; }                              // cl_zero_ClPair()
func use<T: func(i32): i32>(h: Holder<T>): i32 { return h.call(5); }  // instance field call

// P3: frozen judgments
if (a == b) { }                       // E10222 on str pairs
for (borrow c in s) { }               // E10043 on a str loop
func g<T>(x: T): i32 { return x.foo(5); }  // E10212

// P4: rulings
let y: i32 = i32(xs[0]++);            // E10023/E10024 on const/read-only
c[0] = 65;                            // E10042 on cstring

// P5: memory-correct
let s: str = (Box { v: mk().a }).v;   // scope-owned backing
func outer<T>(x: T): T { return ([x] () => c ? x : x)(); }  // E10401
```

### Key Decisions

| Decision | Outcome | AR Ref |
| -------- | ------- | ------ |
| Batch scope | 24 fix rows + DEF-103 closure | AR-B1 |
| DEF-95 | binding-scoped owner (shape stays accepted) | AR-B2 |
| DEF-103 | documented limitation, row closes | AR-B3 |
| DEF-108 | nullable inference kept | AR-B4 |
| DEF-122 | rejected at the checker | AR-B5 |
| Diagnostics | new `E10043`; `E10042`/`E10212` wording | AR-B6, AR-B11 |
| Conformance | view equality, `str` loops, bare-parameter members | AR-B7 – AR-B9 |
| Write checks | value updates routed through the statement checks | AR-B10 |
| Execution mode | `--auto-commit` + `--auto-design` (user instruction) | — |

## Specialist Agents

**None** — the standing specialist check (2026-10-10, `codeops/specialist-check.json`) is current
and this batch introduces no new domain.

_Detection evidence: every phase's surface maps onto the installed project reviewers through the
`AGENTS.md` dispatch matrix — P1/P2 (codegen, evaluation order, symbols) onto the correctness
reviewer, the memory-safety auditor, and the emission-integrity auditor; P3/P4 (checker rules)
onto the correctness and semantics reviewers; P5 (ownership, captures, releases) onto the
correctness reviewer, the memory-safety auditor, and the semantics reviewer (AR-B16). The five
findings ledgers present at check time show closure findings fixed in-batch or filed as backlog
rows; no area recurs beyond the reviewers' scope. Candidate creation remains the user's call._

## Related Files

- New: `compiler/tests/eval_emission_integrity_spec_test.rs`,
  `compiler/tests/eval_emission_integrity_impl_test.rs`, `plans/eval-emission-integrity/05-findings.md`
  (appended during execution), `spec/tests/compile-fail/E10043_str_loop.clo`
- Modified: `compiler/src/sema/{ops,stmt,call,expr,mod}.rs`,
  `compiler/src/own/{lower,call,call_check,place,statements,loops,expr,instance_check}.rs`,
  `compiler/src/codegen/{program,expr,calls,function,cleanup,types,guards,control}.rs`,
  `compiler/tests/compile_fail_spec_test.rs`, `spec/14-diagnostics.md` (the approved edits),
  `plans/00-roadmap.md` (row links and the DEF-103 closure)
- Referenced by the design (touch expected, no surface change): `compiler/runtime/shim_freestanding.c`,
  the store-emission order pins, the closure batch's block-body pin (updated deliberately, AR-B12)
