# Non-Lexical Lifetimes Implementation Plan

> **Feature**: Replace the version-1 conservative lexical loan model with non-lexical lifetimes (RFC 2094): a loan is live from its creation to its last use on any path, not to the end of the scope that owns the derived view.
> **Status**: Done — 22/22 tasks; R1–R9 traced; `make check` and `make verify` green (2026-11-09 closeout).
> **Created**: 2026-09-27 01:17
> **Implements**: PLAN-12
> **CodeOps Artifact Schema**: 1

## Overview

Clorraine's borrow checker proves the Ch 16 loan rules on top of the
ownership rules. Version 1 uses the simplest sound model: a loan is live from
its creation until the end of the scope that owns the derived view
(`spec/16-borrow-rules.md:237-241`). That model rejects valid programs, and the
specification names the gap itself: an exclusive loan requested while a shared
loan is live is rejected in v1 "even if unused later" and accepted by v2 NLL
(`spec/16-borrow-rules.md:253`).

This plan implements v2 — the model the spec's phase table already names as the
target (`spec/16-borrow-rules.md:306-310`). During the existing ownership walk,
loan creations, view uses, owner accesses, and owner releases are recorded as
ordered events anchored to the control-flow graph the walk already builds
(`compiler/src/own.rs:157`). After the walk, a new liveness pass computes each
loan's region — the points reachable from its creation that can still reach one
of its uses — and decides the borrow conflicts (`E10022`, `E10031`, `E10032`)
and origin containment (`E10030`) against the regions.

The v1 lexical state and its duplicate rules are removed. The frozen
specification is amended (Ch 16, lineage, ledger O11, one wording fix) because
it describes v1 as the current model; one frozen golden flips and is rewritten
to a shape that still rejects. No syntax, diagnostics, or other ownership rule
changes, and every program v1 accepted stays accepted.

## Minimum-Sufficient Baseline

**Original goal:** Make the borrow checker precise to last uses, so the programs
the spec calls v1-rejected and NLL-accepted compile.
**Smallest viable design:** Ordered events recorded by the existing walk plus a
post-walk reachability pass over the CFG the walk already builds, reusing
`Builder`, `Provenance`, and the view predicates; the lexical `Loans` set is
deleted rather than kept (AR-4, AR-5, AR-10).
**Excluded machinery:** A second silent walk and a detached IR checker were
considered and rejected (AR-4). No new layer, dependency, harness, stage, or
emitted-C surface (AR-17, AR-23).
**Approved complexity:** None — the liveness analysis is the requested feature
itself, not extra support machinery (AR-23).

## Document Index

| #   | Document                                             | Description                                          |
| --- | ---------------------------------------------------- | ---------------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)       | Zero-Ambiguity Gate decisions (audit trail)          |
| 00  | [Index](00-index.md)                                 | This document — overview and navigation              |
| 01  | [Requirements](01-requirements.md)                   | NLL requirements and scope                           |
| 02  | [Current State](02-current-state.md)                 | The v1 lexical checker, its sites, reusable machinery |
| 03-01 | [Liveness Model](03-01-liveness-model.md)          | Events, positions, regions, and the rules            |
| 03-02 | [Checker Integration](03-02-checker-integration.md) | Module, hooks, removals, diagnostics                 |
| 03-03 | [Spec Amendment](03-03-spec-amendment.md)          | The exact spec/ledger edits and the golden rewrite   |
| 07  | [Testing Strategy](07-testing-strategy.md)           | Test cases and verification                          |
| 99  | [Execution Plan](99-execution-plan.md)               | Phases, sessions, and task checklist                 |

## Quick Reference

### Usage Examples

```clorraine
import { String, stringOf, bytes } from std.text;

func sink(take s: String): void {}
func observe(b: u8[]): void {}

func accepted(): void {
    let s = stringOf("hello");
    let v = bytes(s);    // a shared loan of s
    observe(v);          // its last use
    sink(s);             // accepted: the loan ended at observe(v)
}

func rejected(): void {
    let s = stringOf("hello");
    let v = bytes(s);
    sink(s);             // E10022: observe(v) still uses the loan
    observe(v);
}
```

### Key Decisions

| Decision       | Outcome                                                        |
| -------------- | -------------------------------------------------------------- |
| Model          | Reachability-based last-use regions (AR-3)                     |
| Architecture   | Post-walk pass over the emitted CFG (AR-4)                     |
| Loan identity  | One loan id shared by copies and derived views (AR-5)          |
| Same statement | Evaluation-order precision (AR-7)                              |
| E10030         | Reject iff a use is reachable from an owner-release point (AR-8) |
| Diagnostics    | Deterministic post-pass, same text and spans (AR-9)            |
| Spec           | Ch 16/lineage/O11 amended; one golden rewritten (AR-2)         |

## Related Files

| Path | Change |
| ---- | ------ |
| `compiler/src/own/liveness.rs` | **New** — event types, region computation, post-pass checks |
| `compiler/src/own.rs` | Event log, wiring, release events, lexical-state removal |
| `compiler/src/own/flow.rs` | `Binding.loan`; `FlowState.loans` removed |
| `compiler/src/own/statements.rs` | Loan creation/use recording; lexical E10030 removed |
| `compiler/src/own/place.rs` | Use recording; inline conflict checks removed |
| `compiler/src/own/call_check.rs` | Loan-request recording; inline loan conflict removed |
| `compiler/src/own/lower.rs` | Event log save/swap around lambda lifting |
| `compiler/src/borrow.rs` | Lexical `Loans` set removed; view predicates kept |
| `compiler/tests/non_lexical_lifetimes_spec_test.rs` | **New** — specification tests |
| `compiler/tests/non_lexical_lifetimes_impl_test.rs` | **New** — implementation tests |
| `spec/16-borrow-rules.md`, `spec/lineage.md`, `spec/decisions.md`, `spec/03-ownership.md` | Amendment (03-03) |
| `spec/tests/compile-fail/E10031_view_widening_mutation.clo` | Rewritten so it still rejects |
| `plans/00-roadmap.md` | PLAN-12 row |
