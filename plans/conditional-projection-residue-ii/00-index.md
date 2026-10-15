# Conditional, Projection, and Update Residue II Implementation Plan

> **Feature**: Complete the open batch-I follow-on rows — the 27 defects the
> `conditional-projection-residue` reviews filed and did not close: one 🔴 critical
> (`DEF-303`), five 🟠 majors (`DEF-286`, `DEF-288`, `DEF-294`, `DEF-295`, `DEF-310`), and
> 21 🟡 minors (`DEF-281`, `DEF-282`, `DEF-283`, `DEF-284`, `DEF-285`, `DEF-287`, `DEF-290`,
> `DEF-291`, `DEF-292`, `DEF-293`, `DEF-297`, `DEF-298`, `DEF-299`, `DEF-300`, `DEF-302`,
> `DEF-304`, `DEF-305`, `DEF-306`, `DEF-307`, `DEF-308`, `DEF-309`). The rows are the residue
> of the same conditional/projection/update machinery batch I just worked on: the view
> owner/loan/store judgments (`DEF-281`, `285`, `286`, `287`, `288`, `290`, `291`), the
> guarded/discarded/carrier root bindings (`DEF-284`, `292`, `293`, `297`), the fixed-array and
> literal emission shapes (`DEF-298`, `299`, `300`, `302`, `303`, `304`, `305`, `306`), the
> loop-condition and update-order pins with the raw-deref store lowering (`DEF-294`, `295`,
> `308`, `309`, `310`), and the diagnostics and marks (`DEF-282`, `283`, `307`). No batch row is
> parked; the six older ruling-gated rows (`DEF-167`, `169`, `171`, `190`, `191`, `263`) stay
> out (AR-2).
> **Status**: ✅ Complete (2026-10-16) — 33/33 tasks; all 27 rows fixed; see `05-findings.md` (Close-out)
> **Created**: 2026-10-15
> **Implements**: DEF-281, DEF-282, DEF-283, DEF-284, DEF-285, DEF-286, DEF-287, DEF-288,
> DEF-290, DEF-291, DEF-292, DEF-293, DEF-294, DEF-295, DEF-297, DEF-298, DEF-299, DEF-300,
> DEF-302, DEF-303, DEF-304, DEF-305, DEF-306, DEF-307, DEF-308, DEF-309, DEF-310
> (all 27 rows implemented; none parked)
> **CodeOps Artifact Schema**: 1

## Overview

Batch I (`conditional-projection-residue`) closed its declared cluster and, during its review
rounds, filed 30 follow-on rows with reproductions and base/head evidence. Three were fixed in
batch-I follow-ups (`DEF-289`, `DEF-296`, `DEF-301`) and `DEF-278` completed with its phase;
the current status of all follow-up rows was re-verified by probing the actual compiler (AR-3).
This batch handles the remaining 27. Under the standing defect mandate this batch is next: it
carries the only open critical in the backlog, the most majors of any open batch, one shared
machinery (so the review surface is the one batch I just exercised), and freshly recorded fix
directions that name a reproduction and a base/head observation for every row.

Every fix is conformance to the frozen specification: the fix makes an accepted program build
and run with the language's semantics, or makes a false rejection stop, or makes a report match
the pinned one-report boundary. No new language surface, no new diagnostic code, no
specification or registry text change. The designs extend the batch-I contracts — single-view
parity, one owner per created value, the per-arm destination store
(`spec/17-codegen.md:59-84`), the one-report rule (batch-I AR-26), the generated-owner phrase,
and the strictly left-to-right evaluation rule of Ch 04 §3 — to the spellings and positions
batch I did not reach.

The final head of batch I (commit `a462084`) is the planning baseline; the compiler built from
it behaved as AR-3 records in every probe.

## Minimum-Sufficient Baseline

**Original goal:** eliminate the 27 in-scope defects' observable failures — leaks, double-frees,
use-after-frees, segfaults, invalid C under the strict build, dropped writes, re-evaluated fill
expressions, and duplicate or misleading diagnostics — so every accepted spelling builds
strictly and runs with the language's semantics, and the false rejections stop.

**Smallest viable design:** extend the existing machinery (the owner/provenance walk and store
judgment, the root binders and carriers, the discard/literal materializers, the fixed-array
destination and arm shapes, the update pin, the loop-condition fill, and the report renderers)
with the routing, trigger, and ownership decisions each row already names in its fix direction.
No new pass, no new abstraction, no new dependency.

**Excluded machinery:** a new pinning pass for updates (trigger extensions of the existing pin,
as batch-I AR-9); a new diagnostic code or registry row (reserved); an escaping scheme for
generated type keys (`DEF-169`, parked as an ABI decision); a whole-program lifetime analysis
for `DEF-286` (the call-site rule is the bounded design, AR-6).

**Approved complexity:** None — every fix extends existing machinery (AR-14).

## Document Index

| #   | Document                                                                   | Description                                 |
| --- | -------------------------------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)                             | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                                       | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                                         | Scope, the 27 rows, and acceptance criteria |
| 02  | [Current State](02-current-state.md)                                       | The post-batch-I machinery and each gap's site |
| 03-01 | [View owners, loans, and stores](03-01-view-owners-loans-stores.md)      | P1 design (`DEF-281`, `285`, `286`, `287`, `288`, `290`, `291`) |
| 03-02 | [Guarded, discarded, and carrier roots](03-02-guarded-discarded-carrier-roots.md) | P2 design (`DEF-284`, `292`, `293`, `297`) |
| 03-03 | [Fixed-array and literal emissions](03-03-fixed-array-literal-emissions.md) | P3 design (`DEF-298`, `299`, `300`, `302`, `303`, `304`, `305`, `306`) |
| 03-04 | [Loop conditions, update order, and store lowering](03-04-loop-conditions-update-order.md) | P4 design (`DEF-294`, `295`, `308`, `309`, `310`) |
| 03-05 | [Diagnostics and marks](03-05-diagnostics-and-marks.md)                  | P5 design (`DEF-282`, `283`, `307`) |
| 05  | [Findings Ledger](05-findings.md)                                          | Phase-review verdicts, fix rounds, and filed rows |
| 07  | [Testing Strategy](07-testing-strategy.md)                                 | ST cases and verification                   |
| 99  | [Execution Plan](99-execution-plan.md)                                     | Phases, sessions, and task checklist        |

## Quick Reference

### What each phase closes

| Phase | Rows | Surface |
|-------|------|---------|
| 1 | `DEF-281`, `DEF-285`, `DEF-286`, `DEF-287`, `DEF-288`, `DEF-290`, `DEF-291` | View owners, loans, and stores: the duplicable-projection residual binding, the runtime-index element store, the cross-frame out-slot pass-through, the module-global fresh init, the container-escape element loans, the stale literal-carrier loan, the all-null composite store |
| 2 | `DEF-284`, `DEF-292`, `DEF-293`, `DEF-297` | Guarded, discarded, and carrier roots: the deferred call-wrapped element root, the conditional carrier literal's taken arm, the discarded arm's residual-owning root, the bare discarded literal |
| 3 | `DEF-298`, `DEF-299`, `DEF-300`, `DEF-302`, `DEF-303`, `DEF-304`, `DEF-305`, `DEF-306` | Fixed-array and literal emissions: the conditional field value, the conditional-root field projection, the guarded view-array field, the conditional of call-filled literals, the `String[2]` conditional-root return (critical), the guarded/deferred expression fallback, the array-literal element gate, the nested-arm read distribution |
| 4 | `DEF-294`, `DEF-295`, `DEF-308`, `DEF-309`, `DEF-310` | Loop conditions, update order, and store lowering: the mixed-arm loop receiver, the loop-condition member-projection root, the guarded update operand, the array fill evaluation, the bare raw-deref store |
| 5 | `DEF-282`, `DEF-283`, `DEF-307` | Diagnostics and marks: the one-report dedupe at a nested mixed return, the generated-owner phrase, the conditional-iterable move mark |

### Key Decisions

| Decision | Outcome | AR |
|----------|---------|----|
| Batch scope | The 27 open batch-I follow-on rows (1 critical, 5 majors, 21 minors) — pre-decided by the dispatch and recorded, not re-litigated | AR-1 |
| Ruling-gated rows | The six older rows (`DEF-167`, `169`, `171`, `190`, `191`, `263`) stay parked; none of the 27 needs a spec or registry edit | AR-2 |
| Follow-up row status | Re-probed: `DEF-278`/`289`/`296`/`301` verified fixed; `DEF-306`/`308` partial (the landed part is a green control, the residual is red); the other reds confirmed | AR-3 |
| Phase grouping | Five phases by machinery, executed 1 → 2 → 3 → 4 → 5 | AR-4 |
| Handling mode | Red → green spec cases per row; landed partial fixes become green controls; `DEF-294`'s red is host-toolchain-only (re-measured) | AR-5 |
| `DEF-286` | A call-site rule propagating the argument's backing frame; stop-and-park if it cannot avoid over-rejection | AR-6 |
| One-report boundary | Extend batch-I AR-26: one report per span; a downstream consequence report is withheld where the callee-side judgment already reports | AR-7 |
| Test placement | New cases go into the **same** batch-I suite pair (`conditional_projection_residue_spec_test.rs` / `_impl_test.rs`) | AR-12 |
| Lenses | P1/P2 correctness + memory-safety; P3 correctness + emission-integrity + memory-safety; P4 correctness + emission-integrity + semantics + memory-safety; P5 correctness + semantics | AR-13 |

## Specialist Agents

| Role | Kind | Use | AR Ref |
| ---- | ---- | --- | ------ |
| `correctness-reviewer` | reviewer (project brief) | Every phase | AR-13 |
| `memory-safety-auditor` | reviewer (read-only) | P1, P2, P3, P4 | AR-13 |
| `emission-integrity-auditor` | reviewer (read-only) | P3, P4 | AR-13 |
| `semantics-reviewer` | reviewer (project brief) | P4, P5 | AR-13 |

_Detection evidence: the four agents already exist under `.opencode/agents/` (committed project
briefs); batch I dispatched them on the same machinery and their filed rows are this batch's
scope. No new specialist type is needed._

## Related Files

- `compiler/src/own/lower.rs` — the root binders, carriers, discards, fixed-array root, and the update pin
- `compiler/src/own/place.rs` — provenance, the owner walk, and the call-view source
- `compiler/src/own/statements.rs` — store preparation and the return judgment
- `compiler/src/own/expr.rs` — the element-store and mutability judgments and the move marks
- `compiler/src/own/call_check.rs` — argument owners and the take-argument judgment
- `compiler/src/own/loops.rs` — the loop-condition and `for`-iterable lowering
- `compiler/src/codegen/*` — the arm stores (`function.rs`, `guards.rs`), the fixed-array return shapes (`cleanup.rs`), and the literal emission
- `compiler/tests/conditional_projection_residue_spec_test.rs` and `_impl_test.rs` — the batch-I suite pair this batch appends to (AR-12)
