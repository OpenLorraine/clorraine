# Residual Major Closeout Implementation Plan

> **Feature**: Close the 20 open 🟠 major defect rows named by AR-1, severity-first: `DEF-170`,
> `DEF-189`, `DEF-196`, `DEF-197`, `DEF-207`, `DEF-209`, `DEF-211`, `DEF-219`, `DEF-220`,
> `DEF-229`, `DEF-230`, `DEF-231`, `DEF-232`, `DEF-233`, `DEF-238`, `DEF-242`, `DEF-243`,
> `DEF-245`, `DEF-251`, `DEF-255`. The rows span six origin machineries: the module-initializer
> call/guard emission (batch F and the batch G memory audit), the generic element/type-stage
> surface (batch G Phase 2), the member surfaces (batch G Phase 3/4), the call-conversion and
> `ffi` typing surfaces (batch G Phase 3), the null-shaped destination and capture slot stores
> (batch G Phase 4/5), and the standalone emission shapes (batch E, batch F, and the
> def-backlog-closeout close-out).
> **Status**: ✅ Complete (2026-10-17) — 39/39 pending the close-out verification record; all 20 rows fixed; see `05-findings.md` (Close-out)
> **Created**: 2026-10-16
> **Implements**: DEF-170, DEF-189, DEF-196, DEF-197, DEF-207, DEF-209, DEF-211, DEF-219,
> DEF-220, DEF-229, DEF-230, DEF-231, DEF-232, DEF-233, DEF-238, DEF-242, DEF-243, DEF-245,
> DEF-251, DEF-255 (all 20 rows implemented or accounted: 19 implemented, `DEF-209` verified
> fixed at the planning head and pinned as a control)
> **CodeOps Artifact Schema**: 1

## Overview

This is batch K of the standing defect mandate: the 20 open 🟠 major rows named by AR-1 (four of
the batch-J follow-on rows are majors; they and the two ruling-parked majors stay out, AR-2).
The batch-J criticals and majors are fixed; these 20 majors lead the backlog by
count, they were filed with reproductions, base/head evidence, and fix directions, and they are
worked as one batch grouped by the origin machinery so each phase's review surface stays
coherent. Excluded on purpose (pre-decided by the dispatch, AR-1/AR-2): the six older
ruling-parked rows (`DEF-167`, `DEF-169`, `DEF-171`, `DEF-190`, `DEF-191`, `DEF-263`, including
the two majors `DEF-167`/`DEF-171`) and the batch-J follow-on rows (`DEF-311`–`DEF-334` minus
the closed ones), which form the batch after this one.

Every fix is conformance to the frozen specification: the fix makes an accepted program build
and run with the language's semantics, stops an invalid-C acceptance, or makes a report match
the frozen codes. No new language surface, no new diagnostic code, no specification or registry
text change. The two behavior decisions the roadmap left open (`DEF-233`'s value-form enum
member rule, `DEF-211`'s fixed-array `sizeof` value) are resolved inside the frozen rules in
AR-6/AR-7.

The planning baseline is the current head `80433a6` (the batch-J close-out commit); every row
was probed against the actual compiler built from it, and AR-3 records the verified status. The
probe overturned one row: `DEF-209` is already fixed (the projection root of an owned call
result binds and releases in every probed spelling, ASan-clean), so it becomes a pinned control
rather than a red case.

## Minimum-Sufficient Baseline

**Original goal:** eliminate the 20 rows' observable failures — invalid C under the strict
build, silently wrong values, segfaults, double frees, leaked roots, undefined instance symbols,
missing bounds traps, and unchecked member/conversion acceptances — so every accepted spelling
builds strictly and runs with the language's semantics, and the false acceptances report the
frozen diagnostic.

**Smallest viable design:** extend the existing machinery each row's fix direction names — the
initializer lowering and release pass (`codegen/program.rs`), the monomorphization walk
(`own/generics.rs`), the type-stage element binding (`sema/stmt.rs`, `own/loops.rs`,
`codegen.rs`), the member dispatch (`sema/expr.rs`, `own/expr.rs`, `own/instance_check.rs`), the
per-instance duty pattern (`own/instance_check.rs`), the null-shape lowering
(`codegen/expr.rs`), the lend/capture store judgment (`own/expr.rs`), and the standalone
emission sites (`codegen/expr.rs`, `codegen/calls.rs`, `own/lower.rs`,
`compiler/runtime/shim_freestanding.c`). No new pass, no new abstraction, no new dependency.

**Excluded machinery:** a whole-program type-inference engine (the initializer inference reuses
the body path's bounded `match_param`/infer helpers); a new diagnostics family (every rejection
reuses the frozen registry); a generalized arity/conversion framework (the per-instance duty
mirrors the existing iterable/storable duties); eager evaluation of a folded enum member read
(`DEF-233` rejects per AR-6).

**Approved complexity:** None — every fix extends existing machinery (AR-14).

## Document Index

| #    | Document                                                                 | Description                                     |
| ---- | ------------------------------------------------------------------------ | ----------------------------------------------- |
| AR   | [Ambiguity Register](00-ambiguity-register.md)                           | Zero-Ambiguity Gate decisions (audit trail)     |
| 00   | [Index](00-index.md)                                                     | This document — overview and navigation         |
| 01   | [Requirements](01-requirements.md)                                       | Scope, the 20 rows, and acceptance criteria     |
| 02   | [Current State](02-current-state.md)                                     | The post-batch-I/J machinery and each gap's site |
| 03-01 | [Initializer call resolution, guards, and nullable view conditionals](03-01-init-call-guards-view-conditionals.md) | P1 design (`DEF-196`, `197`, `207`, `219`, `220`) |
| 03-02 | [Generic element placeholder, inference, and bounds](03-02-element-placeholder-inference.md) | P2 design (`DEF-229`, `230`, `231`, `232`) |
| 03-03 | [Member typing: enum value form and view members](03-03-member-typing.md) | P3 design (`DEF-233`, `238`)                   |
| 03-04 | [Call conversions and `ffi` result typing](03-04-call-conversions-ffi-typing.md) | P4 design (`DEF-242`, `243`)                   |
| 03-05 | [Null-shaped destinations and capture slot stores](03-05-null-destinations-capture-stores.md) | P5 design (`DEF-245`, `251`)                   |
| 03-06 | [Standalone emission shapes](03-06-standalone-emission-shapes.md)        | P6 design (`DEF-170`, `189`, `211`, `255`)      |
| 05   | [Findings Ledger](05-findings.md)                                        | Phase-review verdicts, fix rounds, and filed rows |
| 07   | [Testing Strategy](07-testing-strategy.md)                               | ST cases and verification                       |
| 99   | [Execution Plan](99-execution-plan.md)                                   | Phases, sessions, and task checklist            |

## Quick Reference

### What each phase closes

| Phase | Rows | Surface |
|-------|------|---------|
| 1 | `DEF-196`, `DEF-197`, `DEF-207`, `DEF-219`, `DEF-220` | Module-initializer call resolution, guards, and nullable view conditionals: the expression callee, the inferred generic instance (top-level and view-builder), the nullable view conditional's null arm, the guarded right-side owned bind's release scope; `DEF-209` is the green control |
| 2 | `DEF-229`, `DEF-230`, `DEF-231`, `DEF-232` | Generic element placeholder, inference, and bounds: the type-stage element binding, the element member read through a generic callee, the closure capture environment field, the unresolved instance key, the row-element index guard |
| 3 | `DEF-233`, `DEF-238` | Member typing: the value-form enum member read (rejected per AR-6) and the view `.ptr`/`.len` member types with their conversions |
| 4 | `DEF-242`, `DEF-243` | Call conversions and `ffi` result typing: the parameter-typed argument/return/declaration duty and the `ffi` result's declared type |
| 5 | `DEF-245`, `DEF-251` | Null-shaped destinations (lower to the destination's null value) and the capture source at a nullable owning out-slot store (judged, no double free) |
| 6 | `DEF-170`, `DEF-189`, `DEF-211`, `DEF-255` | Standalone emission shapes: the imported/qualified func-global callee symbol, the freestanding `_start` stack alignment, the fixed-array `sizeof` value (AR-7), the void expression-bodied lambda over an intrinsic statement |

### Key Decisions

| Decision | Outcome | AR |
|----------|---------|----|
| Batch scope | The 20 open majors, severity-first — pre-decided by the dispatch and recorded, not re-litigated | AR-1 |
| Parked-rows boundary | The six older ruling-parked rows and the batch-J follow-ons stay out | AR-2 |
| Probe status | `DEF-209` verified fixed (green control); `DEF-229` partial (direct read green, type-sensitive use red); the other 18 confirmed red | AR-3 |
| Phase grouping | Six phases by origin machinery, executed 1 → 2 → 3 → 4 → 5 → 6 | AR-4 |
| Handling mode | Red → green spec cases per row; `DEF-209` is a pinned control; no row parked | AR-5 |
| `DEF-233` | Reject the value-form enum member read with `E10212` on the concrete and instance paths | AR-6 |
| `DEF-211` | `sizeof(T)` of a fixed array is the size of the whole type; no spec edit | AR-7 |
| `DEF-242` | A per-instance conversion duty (the iterable duty's shape), not a declaration rejection | AR-8 |
| Test placement | Additions to the origin suites; the new `residual_major_closeout_{spec,impl}_test.rs` pair only if an origin suite cannot host a case naturally | AR-9 |
| Lenses | P1 correctness + emission-integrity + memory-safety; P2 all four; P3/P4 correctness + semantics + emission-integrity; P5 correctness + memory-safety + semantics; P6 correctness + emission-integrity + semantics | AR-10 |

## Specialist Agents

| Role | Kind | Use | AR Ref |
| ---- | ---- | --- | ------ |
| `correctness-reviewer` | reviewer (project brief) | Every phase | AR-10 |
| `memory-safety-auditor` | reviewer (read-only) | P1, P2, P5, and any ownership/drop diff | AR-10 |
| `emission-integrity-auditor` | reviewer (read-only) | P1, P2, P3, P4, P6 | AR-10 |
| `semantics-reviewer` | reviewer (project brief) | P2, P3, P4, P5, P6 | AR-10 |

_Detection evidence: the four agents already exist under `.opencode/agents/` (committed project
briefs) and authored the base evidence for these rows; no new specialist type is needed._

## Related Files

- `compiler/src/codegen/program.rs` — the initializer lowering, the init-owner release pass, the guarded init binds
- `compiler/src/own/generics.rs` — the monomorphization walk (`collect`) and its explicit-type-argument rule
- `compiler/src/sema/stmt.rs`, `compiler/src/own/loops.rs`, `compiler/src/codegen.rs`, `compiler/src/types/keys.rs` — the element placeholder and its substitution/key mapping
- `compiler/src/sema/expr.rs`, `compiler/src/own/expr.rs`, `compiler/src/own/instance_check.rs` — the member dispatch, the opaque projection audit, and the per-instance duties
- `compiler/src/sema/call.rs`, `compiler/src/own/call.rs`, `compiler/src/own/call_check.rs` — call conversion checks and inference
- `compiler/src/codegen/calls.rs`, `compiler/src/codegen/expr.rs`, `compiler/src/codegen/guards.rs`, `compiler/src/codegen/function.rs` — the intrinsic, conditional, guard, and store shapes
- `compiler/src/own/lower.rs` — the lambda lift and the lend/capture store judgment
- `compiler/runtime/shim_freestanding.c` — the `_start` entry point
- Origin suites: `compiler/tests/module_initializer_integrity_*_test.rs`, `checker_conformance_*_test.rs`, `callable_release_integrity_*_test.rs`, `def_backlog_closeout_*_test.rs`, `freestanding_spec_test.rs`
