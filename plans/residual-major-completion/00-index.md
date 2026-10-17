# Residual Major Completion (Batch M) Implementation Plan

> **Feature**: Close the last 20 un-parked defect rows of the major backlog, severity-first:
> `DEF-367` (🔴 critical) and the 19 🟠 majors `DEF-360`, `DEF-361`, `DEF-363`, `DEF-364`,
> `DEF-365`, `DEF-368`, `DEF-371`, `DEF-372`, `DEF-373`, `DEF-374`, `DEF-375`, `DEF-376`,
> `DEF-377`, `DEF-378`, `DEF-379`, `DEF-384`, `DEF-385`, `DEF-386`, `DEF-390`. The rows span
> the call identity and written-argument checks, the substitution edges of the conditional and
> element surfaces, the concrete context-typing positions, the dropped store targets, the
> element/store loan transfers, the raw-deref and guarded-take channels, and the literal
> destination and guard paths.
> **Status**: Planning
> **Created**: 2026-10-17
> **Implements**: the 20 rows above — 18 probed red at the planning head `86d6f51`;
> `DEF-361` and `DEF-374` probed green and carried as pinned controls (AR-3, AR-5, AR-31,
> AR-32)
> **CodeOps Artifact Schema**: 1

## Overview

This is batch M, the close of the major backlog: the last 20 rows after batch L
(`residual-major-closeout-ii`). Nineteen are 🟠 majors and one is the last open 🔴 critical
(`DEF-367`, a silently dropped store). They are worked as one batch grouped by machinery into
seven phases (AR-4), each phase reusing the origin suites as its regression oracle, and each
phase re-probed against the compiler built from head `86d6f51` (AR-5). Two rows turned out to
be already green at the head and become pinned controls rather than red cases:
`DEF-361` (the direct-name shadow family is closed by the batch-L phase-2 body-scope
corrections) and `DEF-374` (the borrow-element conditional take now reports the concrete twin's
two `E10026`).

Every fix is conformance to the frozen specification: the fix makes an accepted program build
and run with the language's semantics, stops an invalid-C acceptance, or makes a runtime
ownership failure impossible. No new language surface, no new diagnostic code, no specification
or registry edit (AR-29); a row that turns out to need one is parked with the evidence. The two
old ruling-parked majors (`DEF-167`, `DEF-171`) and the remaining minors stay out (AR-1); the
dispatch's "79 minors" is the remaining backlog count after this batch (2 parked majors + 77
minors), not 79 minor rows.

The planning baseline is head `86d6f51` (the batch-L close-out follow-up commit). Every row was
probed against the compiler built from that tree with a minimal reproduction reconstructed from
the row note (AR-5, and per-row evidence in `02-current-state.md`). The probe overturned two
rows: `DEF-361` and `DEF-374` are green and become controls. It also refined `DEF-384`: the
nullable-handle instance (`Array<String?>`) already keys and runs correctly, so only the invalid
nullable scalar (`Array<i32?>`) is red; the row's ABI-keying half needs no change (AR-14).

## Minimum-Sufficient Baseline

**Original goal:** eliminate the rows' observable failures — silently dropped stores and writes,
invalid C under the strict build, wrong callee identities, missed loan reports with
heap-use-after-free or double-free, and a skipped guard path that still evaluates a call — so
every accepted spelling builds strictly and runs with the language's semantics, and every
invalid spelling reports the frozen diagnostic.

**Smallest viable design:** extend the machinery each row's fix direction names — the dot-call
written-argument check (`sema/call.rs`), the load-stage import binding check (`load/mod.rs`), the
emitter's qualified-callee identity (`own/call.rs`, `codegen.rs`), the per-instance conditional
and nullable judgments (`sema/ops.rs`, `own/instance_check.rs`), the element binding substitution
(`types/mod.rs`), the per-instance member conversion (`own/instance_check.rs`, `codegen/`), the
context typing of literals (`sema/ops.rs`, `sema/items.rs`, `types/table.rs`), the runtime
indexed-element store (`own/expr.rs`), the assignment place recursion (`sema/ops.rs`), the
element-loan transfers (`own/place.rs`, `own/statements.rs`, `own/expr.rs`), the deref
materialization and the guarded move-out channel (`own/statements.rs`, `own/lower.rs`), and the
literal destination and guard-path lowering (`own/lower.rs`). No new pass, no new abstraction, no
new dependency (AR-30).

**Excluded machinery:** a new diagnostics family (every rejection reuses the frozen registry); a
new store route the frozen place list does not define (the conditional-base store stays rejected,
AR-7); a new ABI key scheme (the nullable spelling already keys, AR-14); a spec edit for the two
parked rows or the minors.

**Approved complexity:** None — every fix extends existing machinery (AR-30).

## Document Index

| #    | Document                                                                 | Description                                     |
| ---- | ------------------------------------------------------------------------ | ----------------------------------------------- |
| AR   | [Ambiguity Register](00-ambiguity-register.md)                           | Zero-Ambiguity Gate decisions (audit trail)     |
| 00   | [Index](00-index.md)                                                     | This document — overview and navigation         |
| 01   | [Requirements](01-requirements.md)                                       | Scope, the 20 rows, and acceptance criteria     |
| 02   | [Current State](02-current-state.md)                                     | The post-L machinery and each row's probe gap   |
| 03-01 | [Call identity and written-argument checks](03-01-call-identity-written-arguments.md) | P1 design (`DEF-360`, `361` control, `DEF-363`, `DEF-390`) |
| 03-02 | [Substitution edges I: conditional and element keys](03-02-substitution-conditional-element.md) | P2 design (`DEF-364`, `365`, `368`) |
| 03-03 | [Substitution edges II: context typing](03-03-substitution-context-typing.md) | P3 design (`DEF-371`, `384`, `385`, `386`) |
| 03-04 | [Dropped store targets](03-04-dropped-store-targets.md) | P4 design (`DEF-367`, `377`) |
| 03-05 | [Element and store loans](03-05-element-store-loans.md) | P5 design (`DEF-372`, `373`, `374` control) |
| 03-06 | [Deref and take channels](03-06-deref-take-channels.md) | P6 design (`DEF-375`, `376`) |
| 03-07 | [Literal destinations and guards](03-07-literal-destinations-guards.md) | P7 design (`DEF-378`, `379`) |
| 05   | [Findings Ledger](05-findings.md)                                        | Phase-review verdicts, fix rounds, and filed rows |
| 07   | [Testing Strategy](07-testing-strategy.md)                               | ST cases and verification                       |
| 99   | [Execution Plan](99-execution-plan.md)                                   | Phases, sessions, and task checklist            |

## Quick Reference

### What each phase closes

| Phase | Rows | Surface |
|-------|------|---------|
| 1 | `DEF-360`, `361` control, `DEF-363`, `DEF-390` | Call identity and written-argument checks: the dot-call written argument, the declaration/import collision, the qualified callee's own identity; the direct-name shadow is a pinned control |
| 2 | `DEF-364`, `365`, `368` | Substitution edges I: the opaque conditional's per-instance judgments, the element-of-element key, the substituted element member at a view destination |
| 3 | `DEF-371`, `384`, `385`, `386` | Substitution edges II: the literal at a generic concrete parameter, the nullable type argument, the qualified type path, the negated literal at concrete contexts |
| 4 | `DEF-367`, `377` | Dropped store targets: the runtime-indexed element field store (critical) and the conditional-base member store's frozen place rejection |
| 5 | `DEF-372`, `373`, `374` control | Element and store loans: the element-read store's loan, the conditional/array-literal/single-view copy's loans; the borrow-element conditional take is a pinned control |
| 6 | `DEF-375`, `376` | Deref and take channels: the deref operand's single materialization and the guarded non-place take |
| 7 | `DEF-378`, `379` | Literal destinations and guards: the hidden view-array destination at every literal position, and the owning nested conditional's arm store inside its guard path |

### Key Decisions

| Decision | Outcome | AR |
|----------|---------|----|
| Batch scope | The 20 remaining un-parked major/critical rows, severity-first — pre-decided by the dispatch and recorded, not re-litigated | AR-1 |
| Parked-rows boundary | `DEF-167`, `DEF-171`, and the remaining minors stay out | AR-1 |
| Probe status | 18 confirmed red; `DEF-361` and `DEF-374` verified green and reclassified as pinned controls | AR-3, AR-5 |
| Phase grouping | Seven phases by machinery, executed 1 → 7 | AR-4 |
| Handling mode | Red → green spec cases per row; the two reclassified rows are pinned controls; no row parked | AR-3, AR-5 |
| `DEF-367` | Emit the store through the indexed element address (the frozen place list includes a field of an element); rejection is non-conformant | AR-6 |
| `DEF-377` | Report the frozen `E10221` place rejection (a conditional selection is not a place) | AR-7 |
| `DEF-384` | Reject the invalid nullable scalar type argument with `E10011`; the nullable-handle instance already keys and stays a control | AR-14 |
| Test placement | Additions to the origin suites; no new suite pair | AR-26 |
| Lenses | P1 correctness + semantics + emission; P2 all four; P3 correctness + semantics + emission; P4 all four; P5 correctness + memory + semantics; P6 all four; P7 all four | AR-27 |

## Specialist Agents

| Role | Kind | Use | AR Ref |
| ---- | ---- | --- | ------ |
| `correctness-reviewer` | reviewer (project brief) | Every phase | AR-27 |
| `semantics-reviewer` | reviewer (project brief) | P1, P2, P3, P5, P6, P7 | AR-27 |
| `memory-safety-auditor` | reviewer (read-only) | P2, P4, P5, P6, P7, and any ownership/loan/drop diff | AR-27 |
| `emission-integrity-auditor` | reviewer (read-only) | Every phase (P1–P7) | AR-27 |

_Detection evidence: the four agents already exist under `.opencode/agents/` (committed project
briefs) and authored the base evidence for these rows; no new specialist type is needed._

## Exclusions

- **Parked majors**: `DEF-167` (a call-shaped comparison emits a call) and `DEF-171` (an accepted
  `embed` outside a constant context) stay parked for their user rulings (AR-1). They are the two
  remaining un-parked-major candidates and are deliberately not planned here.
- **The minors**: the remaining backlog after this batch is 77 minor rows (the dispatch's "79"
  counts the full remaining backlog: 2 parked majors + 77 minors). They form the batch after this
  one (AR-1).
- **The two controls**: `DEF-361` and `DEF-374` are implemented as pinned control cases only; no
  fix is planned (AR-31, AR-32).
- Any new diagnostic code, registry row, specification chapter, or golden (AR-29); a row that
  turns out to need one is parked mid-phase with the evidence.
- A store route for a conditional selection (`DEF-377`'s target class) — the frozen place list is
  closed (AR-7).

## Related Files

- `compiler/src/sema/call.rs` — `check_dot_call` (:465), `dot_call` (:570), the written-argument check
- `compiler/src/load/mod.rs` — `resolve_imports` (:386), the prior-import duplicate check (:413)
- `compiler/src/own/call.rs` — `lower_call` (:279), the qualified callee arm (:255),
  `global_head_segments`; `compiler/src/own/call_check.rs` — `callee_name` (:291)
- `compiler/src/codegen.rs` — `resolve_function` (:701), the own-first fallback
- `compiler/src/sema/ops.rs` — `check_conditional` (:318), `check_assign` (:352), `is_place` (:402),
  `check_unary` (:18)
- `compiler/src/own/instance_check.rs` — the per-instance conversion and move duties and audits
- `compiler/src/types/mod.rs` — `element_param` (:702), `instance_bindings` (:712), `index_element` (:745),
  `unify_numeric` (:839)
- `compiler/src/types/table.rs` — `resolve_ty_with` (:602), `resolve_named` (:659)
- `compiler/src/sema/items.rs` — `validate_type` (:131)
- `compiler/src/own/program.rs` — `report_illegal_nullable` (:279)
- `compiler/src/own/expr.rs` — the assignment/store case (:1226), `emit_assigned` (:2274),
  `indexed_target` (:2354), `emit_indexed_store` (:2417)
- `compiler/src/own/statements.rs` — the deref materialization gate (:490), the copy transfer
- `compiler/src/own/place.rs` — `move_element_loans` (:1184), `check_nullable_decl` (:1931)
- `compiler/src/own/lower.rs` — `materialize_guard_literal` (:1697), `literal_fields_need_stores` (:4536),
  `literal_field_needs_stores` (:4563), `view_element_needs_binding` (:4592), the `&&`/`||` deferred
  right operand (:228)
- `compiler/src/sema/mod.rs` — `resolve_type` (:482)
- Origin suites: `compiler/tests/checker_conformance_*_test.rs`,
  `compiler/tests/conditional_projection_residue_*_test.rs`,
  `compiler/tests/callable_release_integrity_*_test.rs`,
  `compiler/tests/module_initializer_integrity_*_test.rs` (oracle only), the frozen spec suites, and
  `compiler/tests/run_fault_spec_test.rs` (the `DEF-367` bounds fault)
- Recorded probe sources under the gitignored `build/` tree: `build/batchM-plan/probes/` (this
  batch's probes), `build/batchM-plan/extra/`, `build/batchM-plan/matrix/`
