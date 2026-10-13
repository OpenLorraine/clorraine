# Conditional, Projection, and Update Residue Implementation Plan

> **Feature**: Handle the 23 batch-H residue defects in the conditional, projection, view
> lifetime, fixed-array, and update-order machinery: 20 implemented closes — the generic
> arm-projection duty, the fresh root bindings in conditional-of-projections and literal arms, the
> guarded `take` carrier and the guarded struct-literal field root, the owner walk through
> view-returning calls (every arm of a call-wrapped conditional), the array-element positions and
> the `str[N]` return spelling, the escaping view stores and their spellings (globals, fields,
> out-slots) with the scoped local element rule, the loop-condition and discard-position literal
> roots, the fixed-array conditional arms and conditional-root projections, and the missing
> update-order pins (assignment targets, the update-containing right-hand side, literal fields, the
> dot receiver, loop conditions, owning arguments) — one incidental closure (`DEF-259`, kept as a
> green control), and two rows parked for rulings (`DEF-268`, `DEF-279`).
> **Status**: Planning Complete
> **Created**: 2026-10-13 20:40
> **Implements**: DEF-257, DEF-258, DEF-260, DEF-261, DEF-262, DEF-264, DEF-265, DEF-266, DEF-267,
> DEF-269, DEF-270, DEF-271, DEF-272, DEF-273, DEF-274, DEF-275, DEF-276, DEF-277, DEF-278, DEF-280
> (closes DEF-259 as incidentally fixed; DEF-268 and DEF-279 parked for rulings)
> **CodeOps Artifact Schema**: 1

## Overview

Batch H (`conditional-projection-integrity`) fixed its declared cluster and, during its reviews,
filed 24 new rows with reproductions and base == head evidence. This batch handles 23 of them —
20 implemented closes, `DEF-259` incidentally closed at the planning baseline (already green), and
`DEF-268`/`DEF-279` parked for rulings — while `DEF-263` stays parked as before. The rows are not a
grab bag: they are the residue of the batch-H machinery, and every fix below is a routing, trigger,
or judgment gap in code that batch H's phases and fix rounds already touched — the arm binders and
carriers in `own/lower.rs`, the owner walk and provenance in `own/place.rs` and `own/call_check.rs`,
the store preparation in `own/statements.rs` and `own/expr.rs`, the fixed-array destination shape,
and the update pin.

Every fix is conformance to the frozen specification: no new language surface, no new diagnostic
code, no specification or registry text change. The designs extend the batch-H contracts
(single-view parity, one owner per created value, the C destination-store shape for fixed arrays,
and the strictly left-to-right evaluation rule of Ch 04 §3) into the spellings and positions batch
H did not reach.

Execution runs under `--auto-commit --auto-design` (the standing user mandate). After this batch
closes, the remaining backlog is the batch E residue (2 unblocked majors), the batch F initializer
residue (5 majors), and the batch G checker residue (13 majors), plus the eight rows parked for
rulings (AR-2).

## Minimum-Sufficient Baseline

**Original goal:** eliminate the in-scope defects' observable failures — leaks, double-frees,
use-after-frees, invalid C under the strict build, missing or duplicated diagnostics, and
unspecified evaluation order — so every accepted spelling builds strictly and runs with the
language's semantics. (`DEF-259`'s reproduction is already clean at the planning baseline;
`DEF-268` and `DEF-279` stay at current behavior pending their rulings.)

**Smallest viable design:** extend the existing machinery (the per-arm move-out and carrier, the
projection root binders, the owner/provenance walk, the store preparation, the fixed-array
destination shape, and the update pin) with the routing, trigger, and ownership decisions each row
already names in its fix direction. No new pass, no new abstraction, no new dependency.

**Excluded machinery:** a new pinning pass for updates (AR-9 rejects it for trigger extensions of
the existing pin); an escaping scheme for generated type keys (`DEF-169`, parked as an ABI
decision, AR-2); any new diagnostic code (AR-2 and the reserved-decision rule).

**Approved complexity:** None — AR-17 records that every fix extends existing machinery.

## Document Index

| #   | Document                                                                   | Description                                 |
| --- | -------------------------------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)                             | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                                       | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                                         | Scope, the 23 handled rows, and acceptance criteria |
| 02  | [Current State](02-current-state.md)                                       | The batch-H machinery and each gap's site   |
| 03-01 | [Arm roots and per-instance duty](03-01-arm-roots-and-duty.md)           | P1 design (DEF-257, 258, 260, 264; 259 incidental) |
| 03-02 | [View derivation and owners](03-02-view-derivation-and-owners.md)         | P2 design (DEF-261, 266, 267; 268, 279 parked) |
| 03-03 | [Escaping stores and their spellings](03-03-escaping-stores.md)           | P3 design (DEF-265, 269, 276, 280) |
| 03-04 | [Guarded and discarded literal positions](03-04-guarded-literal-positions.md) | P4 design (DEF-262, 270, 271) |
| 03-05 | [Fixed-array conditional arms and projections](03-05-fixed-array-arms.md) | P5 design (DEF-272, 275) |
| 03-06 | [Update order completion](03-06-update-order.md)                          | P6 design (DEF-273, 274, 277, 278) |
| 07  | [Testing Strategy](07-testing-strategy.md)                                 | ST cases and verification                   |
| 99  | [Execution Plan](99-execution-plan.md)                                     | Phases, sessions, and task checklist        |

## Quick Reference

### What each phase closes

| Phase | Rows | Surface |
|-------|------|---------|
| 1 | `DEF-257`, `DEF-258`, `DEF-260`, `DEF-264` (+ `DEF-259` incidental) | Conditional arm roots: per-instance duty, fresh-root sibling binding, the guarded `take` carrier, the struct-literal field root |
| 2 | `DEF-261`, `DEF-266`, `DEF-267` (+ `DEF-268`, `DEF-279` parked) | View derivation and owners: the call-wrapped owner walk (every arm), element positions, the `str[N]` return spelling |
| 3 | `DEF-265`, `DEF-269`, `DEF-276`, `DEF-280` | Escaping stores: globals, fields, out-slots, the store spellings, the scoped local element rule |
| 4 | `DEF-262`, `DEF-270`, `DEF-271` | Guarded and discarded literal positions |
| 5 | `DEF-272`, `DEF-275` | Fixed-array conditional arms and conditional-root/`for`-iterable projections |
| 6 | `DEF-273`, `DEF-274`, `DEF-277`, `DEF-278` | Update-order pins: assignment targets (the right-hand side included), literal fields, the dot receiver, loop conditions, owning arguments |

### Key Decisions

| Decision | Outcome | AR |
|----------|---------|----|
| Batch scope | The 23 handled batch-H residue rows — 20 implemented (14 majors), 1 incidental, 2 parked — not the recorded E residue (2 unblocked majors) | AR-1 |
| Ruling-gated rows | `DEF-167`, `169`, `171`, `190`, `191`, `263`, `268`, `279` stay parked with their needed decision named | AR-2, AR-7, AR-8 |
| Escaping view stores | Rejected per destination kind (`E10033` fields/globals; `E10030` out-slots), like the return path | AR-5, AR-20 |
| Local element stores | Accepted with the binding path's loan when the backing's scope covers the array's | AR-6, AR-21 |
| `DEF-259` | Incidental baseline closure; the reproduction stays a green control | AR-23 |
| Update pins | Trigger extensions of the existing pin, not a new pass | AR-9 |

## Specialist Agents

| Role | Kind | Use | AR Ref |
| ---- | ---- | --- | ------ |
| `correctness-reviewer` | reviewer (project brief) | Every phase | AR-15 |
| `memory-safety-auditor` | reviewer (read-only) | P1, P2, P3, P4, P5 | AR-15 |
| `emission-integrity-auditor` | reviewer (read-only) | P3, P5, P6 | AR-15 |
| `semantics-reviewer` | reviewer (project brief) | P2, P3, P6 | AR-15 |

_Detection evidence: the four agents already exist under `.opencode/agents/` (committed project
briefs); batch H dispatched them on the same machinery and their filed rows are this batch's
scope. No new specialist type is needed._

## Related Files

- `compiler/src/own/lower.rs` — the arm binders, carriers, discards, fixed-array root, and the update pin
- `compiler/src/own/place.rs` — provenance and the owner walk
- `compiler/src/own/call_check.rs` — argument owners and the take-argument judgment
- `compiler/src/own/statements.rs` — store preparation and the return judgment
- `compiler/src/own/expr.rs` — the element-store and mutability judgments
- `compiler/src/own/liveness.rs` — the escape (outlives) report
- `compiler/src/codegen/*` — the fixed-array destination shape and the arm spellings
- `compiler/tests/conditional_projection_residue_spec_test.rs` (new)
- `compiler/tests/conditional_projection_residue_impl_test.rs` (new)
