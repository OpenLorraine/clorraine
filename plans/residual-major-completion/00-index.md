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
or registry edit (AR-29); a row that turns out to need one is parked with the evidence — except
`DEF-367`, which becomes a phase blocker and user ruling (AR-34). The two
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
written-argument route with call-site type-argument inference (`sema/call.rs`, `sema/expr.rs`), the
load-stage import binding check (`load/mod.rs`), the emitter's qualified-callee identity across
the symbol and the mode/parameter/return lookups (`own/call.rs`, `own/call_check.rs`,
`codegen.rs`, `codegen/calls.rs`, `codegen/expr.rs`, `codegen/function.rs`), the per-instance
conditional and nullable judgments (`sema/ops.rs`, `own/instance_check.rs`, `own/place.rs`), the
element binding substitution (`types/mod.rs`), the per-instance member conversion
(`own/instance_check.rs`, `codegen/`), the plain-call literal context and the folded negated
literal (`sema/call.rs`, `sema/ops.rs`), the generalized runtime-indexed store target
(`own/expr.rs`), the assignment place recursion (`sema/ops.rs`), the element-loan transfers
(`own/place.rs`, `own/statements.rs`, `own/expr.rs`), the shared owning-deref materializer and the
guarded move-out channel (`own/statements.rs`, `own/lower.rs`, `ir.rs`, `codegen/guards.rs`), and
the literal destination and guard-path lowering (`own/lower.rs`, `codegen/guards.rs`). No new
pass, no new abstraction, no new dependency (AR-30).

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
| 1 | `DEF-360`, `361` control, `DEF-363`, `DEF-390` | Call identity and written-argument checks: the dot-call written argument (with call-site type-argument inference), the declaration/import collision across every declaration kind, the qualified callee's own identity across symbol and mode/parameter/return lookups; the direct-name shadow is a pinned six-fixture control |
| 2 | `DEF-364`, `365`, `368` | Substitution edges I: the conditional's per-instance judgment with the expected type (direct contexts included), the element-of-element key, the substituted element member at a view destination |
| 3 | `DEF-371`, `384`, `385`, `386` | Substitution edges II: the plain-call literal at a generic concrete parameter, the nullable type argument at nested type-argument positions, the qualified type path, the folded negated literal at concrete contexts |
| 4 | `DEF-367`, `377` | Dropped store targets: the runtime-indexed element/member store (basic, compound, update, deep chain, target order, per-instance owning type) and the conditional-base member store's frozen place rejection (including the `unsafe`-wrapped forms) |
| 5 | `DEF-372`, `373`, `374` control | Element and store loans: the element-read store's loan, the conditional/array-literal/single-view copy's loan union (mixed positions, fill form, conditional arm); the borrow-element conditional take is a pinned control |
| 6 | `DEF-375`, `376` | Deref and take channels: the shared owning-deref materializer (store, return, aggregate field) and the guarded non-place take (deferred pointer fill) |
| 7 | `DEF-378`, `379` | Literal destinations and guards: the hidden view-array destination at every literal position (read position included), and the owning nested conditional's argument carrier/fill inside its guard path |

### Key Decisions

| Decision | Outcome | AR |
|----------|---------|----|
| Batch scope | The 20 remaining un-parked major/critical rows, severity-first — pre-decided by the dispatch and recorded, not re-litigated | AR-1 |
| Parked-rows boundary | `DEF-167`, `DEF-171`, and the remaining minors stay out | AR-1 |
| Probe status | 18 confirmed red; `DEF-361` and `DEF-374` verified green and reclassified as pinned controls | AR-3, AR-5 |
| Phase grouping | Seven machinery phases; scope is severity-first, execution is dependency-ordered (P1 → P2/P3, P2 → P5, P6 → P7; P4 independent and fourth) | AR-4, AR-36 |
| Handling mode | Red → green spec cases per row; the two reclassified rows are pinned controls; no row parked | AR-3, AR-5 |
| `DEF-367` | Emit the store through the indexed element address, with the element/member type resolved per instance, all target forms (plain, compound, update, deep chain), and the target location materialized before the RHS; rejection is non-conformant | AR-6 |
| `DEF-377` | Report the frozen `E10221` place rejection (a conditional selection, including an `unsafe`-wrapped one, is not a place); the place test recurses through `Unsafe` and the Index bypass is removed | AR-7 |
| `DEF-384` | Reject the invalid nullable scalar at nested type-argument positions with `E10011`; keep the existing declaration checks; the nullable-handle instance already keys and stays a control | AR-14 |
| `DEF-386` | Judge the fit on the folded negated value: `i8 = -128` accepts, `u8 = -5` and `i8 = -129` report `E10202`; no wrapping-value pin | AR-16 |
| Test placement | Additions to the origin suites; no new suite pair (a no-home case needs a register entry and user approval); only `_impl_test.rs` byte-identity pins may be re-pinned | AR-26, AR-35 |
| Lenses | P1 all four; P2 all four; P3 correctness + semantics + emission; P4 all four; P5 correctness + memory + semantics; P6 all four; P7 all four | AR-27 |

## Specialist Agents

| Role | Kind | Use | AR Ref |
| ---- | ---- | --- | ------ |
| `correctness-reviewer` | reviewer (project brief) | Every phase | AR-27 |
| `semantics-reviewer` | reviewer (project brief) | Every phase (P1–P7, including P4) | AR-27 |
| `memory-safety-auditor` | reviewer (read-only) | P1, P2, P4, P5, P6, P7, and any ownership/loan/drop diff | AR-27 |
| `emission-integrity-auditor` | reviewer (read-only) | P1, P2, P3, P4, P6, P7 (not P5) | AR-27 |

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
  fix is planned (AR-31, AR-32). At close-out both move to `Done` with their control-pin evidence.
- Any new diagnostic code, registry row, specification chapter, or golden (AR-29); a row that
  turns out to need one is parked mid-phase with the evidence — except `DEF-367`, whose only
  fallback is a phase blocker and user ruling (AR-34).
- A store route for a conditional selection (`DEF-377`'s target class) — the frozen place list is
  closed (AR-7). The dropped `(*p).f` member store is a residual row filed during execution, not
  planned here (AR-7).
- A new test-suite pair: every case has an origin home; a case with no home requires a register
  entry and user approval (AR-26).

## Related Files

- `compiler/src/sema/call.rs` — `check_dot_call` (:465), `dot_call` (:570), `check_args` (:629),
  `check_args_impl` (:671); the call-site type-argument inference
- `compiler/src/sema/expr.rs` — `record_deferred_conversion` (:491), the expected-type plumbing
- `compiler/src/load/mod.rs` — `resolve_imports` (:386), `declarations_of` (:293), the prior-import
  duplicate check (:413)
- `compiler/src/own/call.rs` — `lower_call` (:279, the callee name via `callee_name` at :308), the
  qualified func-field arm (:255), `global_head_segments`;
  `compiler/src/own/call_check.rs` — `callee_name` (:291-298)
- `compiler/src/codegen.rs` — `resolve_function` (:701), the own-first fallback;
  `compiler/src/codegen/calls.rs`, `codegen/expr.rs`, `codegen/function.rs` — the mode, parameter,
  and return lookups that must share the qualified identity
- `compiler/src/sema/ops.rs` — `check_conditional` (:318, the expected-type parameter),
  `check_assign` (:354), `is_place` (:402; Index :412, Member :423), `check_unary` (:18)
- `compiler/src/own/instance_check.rs` — the per-instance conversion, move, and conditional duties
  and audits
- `compiler/src/types/mod.rs` — `element_param` (:702), `instance_bindings` (:712, the level push
  :730-732), `index_element` (:745), `unify_numeric` (:839)
- `compiler/src/types/table.rs` — `resolve_ty_with` (:602), `resolve_named` (:659)
- `compiler/src/sema/items.rs` — `validate_type` (:131)
- `compiler/src/own/program.rs` — `report_illegal_nullable` (:279)
- `compiler/src/own/place.rs` — `replace_element_loans` (:1103), `move_element_loans` (:1184, the
  destination replace :1202), `check_nullable_decl` (:1931)
- `compiler/src/own/expr.rs` — the assignment/store case (:1226), `emit_assigned` (:2274),
  `indexed_target` (:2354, the Index-only gate :2364-2366), `emit_indexed_store` (:2417),
  `emit_indexed_update` (:2491)
- `compiler/src/own/statements.rs` — `prepare_store` (:440), the deref store gate (:490), the
  `let` copy transfer (:391-399), `check_return` (:1358)
- `compiler/src/own/lower.rs` — `materialize_owned_argument` (:4866, the take materialization
  :4870-4888, the by-value carrier :5109-5131), `materialize_guard_literal` (:1697),
  `literal_fields_need_stores` (:4536), `literal_field_needs_stores` (:4563),
  `view_element_needs_binding` (:4592), the `&&`/`||` deferred right operand (:228)
- `compiler/src/ir.rs` — the `Materialize` value and its `move_outs` (:298)
- `compiler/src/codegen/guards.rs` — the guarded statement fill (:106-117), the guarded expression
  fill (:1016), the drop/re-fill hoists (:273-307)
- `compiler/src/sema/mod.rs` — `resolve_type` (:482)
- Origin suites: `compiler/tests/checker_conformance_*_test.rs`,
  `compiler/tests/conditional_projection_residue_*_test.rs` (the `DEF-367` bounds case with its
  inline `assert_traps` helper, precedent `borrow_store_integrity_spec_test.rs:363`),
  `compiler/tests/callable_release_integrity_*_test.rs`,
  `compiler/tests/module_initializer_integrity_*_test.rs` (oracle only), and the frozen spec suites
  (`run_fault_spec_test.rs` stays untouched)
- Recorded probe sources under the gitignored `build/` tree: `build/batchM-plan/probes/`,
  `build/batchM-plan/preflight/` (the preflight re-runs), `build/batchM-plan/extra/`,
  `build/batchM-plan/matrix/`
