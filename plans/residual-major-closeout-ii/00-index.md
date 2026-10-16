# Residual Major Closeout II (Batch L) Implementation Plan

> **Feature**: Close the 23 remaining open 🟠 major defect rows (batch L), severity-first:
> `DEF-315`, `DEF-322`, `DEF-323`, `DEF-329`, `DEF-335`, `DEF-336`, `DEF-337`, `DEF-338`,
> `DEF-339`, `DEF-340`, `DEF-341`, `DEF-342`, `DEF-344`, `DEF-345`, `DEF-346`, `DEF-347`,
> `DEF-348`, `DEF-350`, `DEF-351`, `DEF-353`, `DEF-356`, `DEF-357`, `DEF-359`. The rows are the
> fresh follow-ons of the batches I/J/K with recorded reproductions, base/head evidence, and fix
> directions; they span the nullable-view arm machinery, the generic walk and instance surfaces,
> the ownership transfer judgments, the store/literal emission shapes, the call-conversion and
> `ffi` typing surfaces, the concrete member path, and the module/global identity machinery.
> **Status**: Planning
> **Created**: 2026-10-16
> **Implements**: the 23 rows above — 21 red at the planning head, `DEF-341` and `DEF-350` probed
> as incidentally fixed by the batch-K corrections and carried as pinned controls (AR-3, AR-8)
> **CodeOps Artifact Schema**: 1

## Overview

This is batch L of the standing defect mandate: all remaining open 🟠 majors, severity-first
(AR-1). Batches I/J/K fixed the criticals and the earlier majors; these 23 are the last
un-parked majors, filed with reproductions by the batch-J and batch-K phase reviews. They are
worked as one batch grouped by machinery (eight phases, AR-4) so each phase's review surface
stays coherent, and each phase reuses the origin suites as its regression oracle. Excluded on
purpose (pre-decided by the dispatch, AR-1/AR-2): the six older ruling-parked rows (`DEF-167`,
`DEF-169`, `DEF-171`, `DEF-190`, `DEF-191`, `DEF-263` — `DEF-167`/`DEF-171` are majors and stay
parked for their user rulings) and the minors, which form the batch after this one.

Every fix is conformance to the frozen specification: the fix makes an accepted program build
and run with the language's semantics, stops an invalid-C acceptance, or makes a runtime
ownership failure impossible (a rejection or a corrected emission). No new language surface, no
new diagnostic code, no specification or registry text change (AR-12); a row that turns out to
need one is parked with the evidence.

The planning baseline is head `8829be6` (the batch-K close-out commit). Every row was probed
against the compiler built from that tree with the recorded probe sources (AR-3). The probe
overturned two rows: `DEF-341` (the invalid-C acceptance is closed — both the generic and the
concrete spelling now report `E10401`) and `DEF-350` (the batch-K phase-4 F1 correction typed
the view/slice `ffi` result as the ABI pointer; the view destinations now reject with `E10200`
and the length/index/cstring uses build — the residual `.ptr`/`.len`-on-a-raw-pointer spellings
belong to the separately filed minor `DEF-352`). Both become pinned controls rather than red
cases.

## Minimum-Sufficient Baseline

**Original goal:** eliminate the rows' observable failures — null dereferences, dropped stores
and writes, invalid C under the strict build, undefined generic instances, and leaks or double
frees on the accepted spellings — so every accepted spelling builds strictly and runs with the
language's semantics, and every invalid spelling reports the frozen diagnostic.

**Smallest viable design:** extend the machinery each row's fix direction names — the
per-arm/place null-guarded view widening (`codegen/expr.rs`, `codegen/calls.rs`,
`codegen/function.rs`, `codegen/program.rs`), the monomorphization walk's projected-argument
resolver (`own/generics.rs`), the per-instance substitution and element surfaces
(`own/instance_check.rs`, `codegen/drop.rs`), the ownership transfer judgments (`own/expr.rs`),
the conditional-base store's frozen place rejection and the guarded-literal route (`sema/ops.rs`, `own/lower.rs`, the store emitters), the
per-instance conversion duties (`own/call_check.rs`, `sema/call.rs`), the member dispatch and
conversion (`sema/expr.rs`, `codegen/function.rs`), and the func-global identity and const-folder
sites (`codegen/expr.rs`, `own/call.rs`, `load/`, `consts/`). No new pass, no new abstraction, no
new dependency (AR-14).

**Excluded machinery:** a new diagnostics family (every rejection reuses the frozen registry); a
generalized layout engine for the const folder (the extent fold defers to the C type size, AR-13);
eager evaluation of the deferred guard spellings (the fixes keep laziness); a spec edit for the
out-of-surface `len`/`.ptr` spellings (those remain the minors' scope).

**Approved complexity:** None — every fix extends existing machinery (AR-14).

## Document Index

| #    | Document                                                                 | Description                                     |
| ---- | ------------------------------------------------------------------------ | ----------------------------------------------- |
| AR   | [Ambiguity Register](00-ambiguity-register.md)                           | Zero-Ambiguity Gate decisions (audit trail)     |
| 00   | [Index](00-index.md)                                                     | This document — overview and navigation         |
| 01   | [Requirements](01-requirements.md)                                       | Scope, the 23 rows, and acceptance criteria     |
| 02   | [Current State](02-current-state.md)                                     | The post-I/J/K machinery and each gap's site    |
| 03-01 | [Nullable-view arms and initializer view stores](03-01-nullable-view-arms.md) | P1 design (`DEF-335`, `336`, `337`) |
| 03-02 | [The generic walk's projected-argument resolver](03-02-generic-walk-resolver.md) | P2 design (`DEF-338`, `340`, `342`) |
| 03-03 | [Instance substitution and element surfaces](03-03-element-instance-substitution.md) | P3 design (`DEF-339`, `345`, `353`, `341` control) |
| 03-04 | [Ownership transfer judgments](03-04-ownership-transfer-judgments.md)    | P4 design (`DEF-315`, `329`)                    |
| 03-05 | [Store emission: conditional bases and guarded view literals](03-05-store-emission-conditional-literals.md) | P5 design (`DEF-322`, `323`) |
| 03-06 | [Call conversions and `ffi` typing](03-06-call-conversions-ffi.md)       | P6 design (`DEF-344`, `351`, `350` control)     |
| 03-07 | [Member typing on concrete paths](03-07-member-typing-concrete-paths.md) | P7 design (`DEF-346`, `347`, `348`)             |
| 03-08 | [Module/global identity and the const folder](03-08-module-global-identity-and-folds.md) | P8 design (`DEF-356`, `357`, `359`) |
| 05   | [Findings Ledger](05-findings.md)                                        | Phase-review verdicts, fix rounds, and filed rows |
| 07   | [Testing Strategy](07-testing-strategy.md)                               | ST cases and verification                       |
| 99   | [Execution Plan](99-execution-plan.md)                                   | Phases, sessions, and task checklist            |

## Quick Reference

### What each phase closes

| Phase | Rows | Surface |
|-------|------|---------|
| 1 | `DEF-335`, `336`, `337` | Nullable-view arms and initializer view stores: the null-guarded per-arm widening at the conditional, argument, and initializer positions |
| 2 | `DEF-338`, `340`, `342` | The walk's projected-argument resolver: the local-headed qualified receiver, the temp-rooted projection, the conditional and non-local place bases share one resolver |
| 3 | `DEF-339`, `345`, `353` (+ `DEF-341` control) | Instance substitution and element surfaces: the element store's overwritten-owner drop, the substituted element in conversion/member positions, the duplicable out-slot store; the capture pair is a pinned control |
| 4 | `DEF-315`, `329` | Ownership transfer judgments: the view-like copy's element loans and the owned raw deref read's move/duplicate judgment |
| 5 | `DEF-322`, `323` | Store emission: the frozen `E10221` rejection for the element store through a conditional base, and the guarded conditional of call-filled view-array literals |
| 6 | `DEF-344`, `351` (+ `DEF-350` control) | Call conversions and `ffi` typing: the null-shaped inferred argument and the inverse conversion routes at instances; the `ffi` view/slice result is a pinned control |
| 7 | `DEF-346`, `347`, `348` | Member typing on concrete paths: the fixed-array `.ptr`/unknown view members, the const pointer at mutable destinations, the `raw E` member read |
| 8 | `DEF-356`, `357`, `359` | Module/global identity and the const folder: the aliased import and the qualified value-head path, the ABI-layout-aware extent fold, the scope-aware checker typing |

### Key Decisions

| Decision | Outcome | AR |
|----------|---------|----|
| Batch scope | The 23 remaining un-parked majors, severity-first — pre-decided by the dispatch and recorded, not re-litigated | AR-1 |
| Parked-rows boundary | The six older ruling-parked rows and the minors stay out | AR-2 |
| Probe status | 21 confirmed red; `DEF-341` and `DEF-350` verified incidentally fixed and reclassified as pinned controls (AR-8) | AR-3 |
| Phase grouping | Eight phases by machinery, executed 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 | AR-4 |
| Handling mode | Red → green spec cases per row; the two reclassified rows are pinned controls; no row parked | AR-5 |
| `DEF-322` | Report the frozen `E10221` place rejection for the element store (the scalar member twin's rule; the per-arm route is parked, preflight PF-001) | AR-6 |
| `DEF-347` | Cast the view pointer at a mutable raw destination (the sanctioned member-conversion shape); the conversion duty is the fallback | AR-7 |
| `DEF-357` | Defer the extent-position fold to the C type size (`sizeof(ClS)`), as the value position already does | AR-13 |
| Test placement | Additions to the origin suites; a new `residual_major_closeout_ii_{spec,impl}_test.rs` pair only if a case cannot host naturally | AR-9 |
| Lenses | P1 correctness + memory-safety + emission-integrity; P2 correctness + semantics + memory-safety + emission-integrity; P3 all four; P4 correctness + memory-safety + semantics; P5 correctness + memory-safety + emission-integrity; P6 correctness + semantics + memory-safety + emission-integrity; P7 correctness + semantics + emission-integrity; P8 correctness + semantics + emission-integrity | AR-10 |

## Specialist Agents

| Role | Kind | Use | AR Ref |
| ---- | ---- | --- | ------ |
| `correctness-reviewer` | reviewer (project brief) | Every phase | AR-10 |
| `memory-safety-auditor` | reviewer (read-only) | P1, P2, P3, P4, P5, P6, and any ownership/drop diff | AR-10 |
| `emission-integrity-auditor` | reviewer (read-only) | P1, P2, P3, P5, P6, P7, P8 | AR-10 |
| `semantics-reviewer` | reviewer (project brief) | P2, P3, P4, P6, P7, P8 | AR-10 |

_Detection evidence: the four agents already exist under `.opencode/agents/` (committed project
briefs) and authored the base evidence for these rows; no new specialist type is needed._

## Related Files

- `compiler/src/codegen/expr.rs` — `conditional_arm` (:610), the callee/global resolution (:426)
- `compiler/src/codegen/calls.rs` — the argument widening (`:150`)
- `compiler/src/codegen/function.rs` — `store_text` (:2068), `widen_text_view` (:2150), the pointer-member conversion
- `compiler/src/codegen/program.rs` — the initializer store arms (:1052-1077), the init-owner pass
- `compiler/src/own/generics.rs` — the monomorphization walk (`record_call` :407, `resolve_ir_call_args` :1031, `resolved_ir_value_ty` :1063)
- `compiler/src/own/instance_check.rs` — the per-instance duties (:78 the capture move), the audits
- `compiler/src/own/lower.rs` — `materialize_guard_literal` (:1680), `literal_fields_need_stores` (:4488), the store lowering
- `compiler/src/own/expr.rs` — the loan/move judgments, the raw deref read; `compiler/src/own/statements.rs`, `compiler/src/own/place.rs` — the copy transfer and the element loans (`move_element_loans`)
- `compiler/src/sema/expr.rs`, `compiler/src/sema/call.rs`, `compiler/src/own/call_check.rs` — the member dispatch, the call typing, the null rule (:397)
- `compiler/src/own/call.rs` — `func_global_callee_path` (:454), `function_in_scope` (:486)
- `compiler/src/consts/mod.rs` — `fold_intrinsic` (:495, the `sizeof`/`offsetof` folds)
- `compiler/src/codegen/drop.rs` — `drop_expr_text` (:141), `emits_drop` (:237)
- Origin suites: `compiler/tests/module_initializer_integrity_*_test.rs`, `checker_conformance_*_test.rs`, `conditional_projection_residue_*_test.rs`, `callable_release_integrity_*_test.rs`
- Recorded probe sources under the gitignored `build/` tree: `build/memory-audit/k1/` (P1), `build/k2-audit/`, `build/K2S-review/`, `build/emission-audit/k2/probes/` (P2/P3), `build/memory-audit/final-audit/`, `build/memory-audit/batchJ-p4-final/` (P4), `build/memory-audit/j4/` (P5), `build/memory-audit/k4p4/`, `build/K4S-sem/`, `build/emission-audit/k4/` (P6), `build/memory-audit/p5/`, `build/k18/` (P8)
