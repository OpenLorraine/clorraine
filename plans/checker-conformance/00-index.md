# Checker Conformance Implementation Plan

> **Feature**: Close the checker-conformance cluster: call and member surfaces (arity, unknown
> members, `cstring` crossings), generic-instance judgments (parameter iterables, value-form
> member reads, the conditional `len` fold), null-shaped values, diagnostic hygiene — plus two
> emission-guard fixes and three plan-created companion rows in the same mechanisms.
> **Status**: Done — plan and roadmap rows complete (2026-11-09 closeout).
> **Created**: 2026-10-11 15:20
> **Implements**: DEF-136, DEF-142, DEF-144, DEF-145, DEF-146, DEF-147, DEF-148, DEF-149,
> DEF-150, DEF-151, DEF-153, DEF-154, DEF-160, DEF-161, DEF-162, DEF-213 (gated on a ruling),
> DEF-215, DEF-216
> **CodeOps Artifact Schema**: 1

## Overview

This is batch G of the user-mandated defect program: the checker-conformance rows that the
evaluation/emission and module-initializer reviews filed with reproductions. They share one
shape: **the checker accepts a program the frozen rules do not allow, and the emission produces
C that fails the strict build** — an unchecked call arity, an unknown member on a raw or plain
struct receiver, a generic iterable judged only after substitution, a null-shaped local used as
a concrete value, a `str`/slice view crossing into a `cstring` parameter, a test view folded to
zero, a duplicate or mis-attributed state diagnostic — plus two emission guards that re-read an
impure operand and two typedef-collector gaps.

Every fix is conformance: no new language surface, no new diagnostic code except the arity row
that awaits a user ruling (see the register), and no specification text change. The designs
extend the established machinery — the sema's conversion and member checks, the per-instance
audit pattern (`own/instance_check.rs`), the own stage's state reporters and capture-move
recorder, the initializer materialization path, and the body-side slice-type collector.

Execution runs under `--auto-commit --auto-design` (the standing mandate). Batch D
(`clorraine-dx`) stays parked and batch C (fuzzing) follows the defect work.

## Minimum-Sufficient Baseline

**Original goal:** eliminate the rows' observable failures — accepted-source-invalid-C,
wrongly-accepted writes and moves, duplicate or mis-attributed diagnostics — with accepted
programs staying accepted and unaffected emissions byte-identical.
**Smallest viable design:** extend the existing checks (conversion table, member resolution,
dot-call candidate matching, the per-instance audit, the null-shape predicate, the
capture-move recorder, the state reporters) and the two emission walks (the initializer guard
operands, the body slice-type collector); reuse existing diagnostic codes except the arity row.
**Excluded machinery:** a general overload/arity resolution framework; typed element views for
generic iterables (the reject-at-the-instance route is preferred); a new diagnostics family; a
null-type inference engine (the null-shape predicate suffices).

## Document Index

| #   | Document                                                                   | Description                                 |
| --- | -------------------------------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)                             | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                                       | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                                         | Requirements and scope                      |
| 02  | [Current State](02-current-state.md)                                       | Verified present behavior and file map      |
| 03-01 | [Initializer Guard Operands and Typedef Collectors](03-01-initializer-guards-typedefs.md) | DEF-136, DEF-142, DEF-215, DEF-216 |
| 03-02 | [Generic Instance Conformance](03-02-generic-instance-conformance.md)      | DEF-145, DEF-146, DEF-150                   |
| 03-03 | [Call, Member, and CString Surfaces](03-03-call-member-cstring-surfaces.md) | DEF-144, DEF-147, DEF-148, DEF-154 (+DEF-213) |
| 03-04 | [Null-Shaped Values](03-04-null-shaped-values.md)                           | DEF-149, DEF-151                            |
| 03-05 | [Diagnostic Hygiene](03-05-diagnostic-hygiene.md)                           | DEF-153, DEF-160, DEF-161, DEF-162          |
| 05  | [Findings](05-findings.md)                                                 | Per-phase review ledger                     |
| 07  | [Testing Strategy](07-testing-strategy.md)                                 | Suites, cases, and methodology              |
| 99  | [Execution Plan](99-execution-plan.md)                                     | Task-by-task execution                      |

## Phase Summary

| Phase | Deliverables | Lenses |
| ----- | ------------ | ------ |
| 1 — Initializer guard operands and typedef collectors | DEF-136, DEF-142, DEF-215, DEF-216 | correctness, memory-safety, emission-integrity |
| 2 — Generic instance conformance | DEF-145, DEF-146, DEF-150 | correctness, semantics, emission-integrity |
| 3 — Call, member, and `cstring` surfaces | DEF-144, DEF-147, DEF-148, DEF-154 (+DEF-213, gated) | correctness, semantics, emission-integrity |
| 4 — Null-shaped values | DEF-149, DEF-151 (+ the null-shaped iterable) | correctness, semantics |
| 5 — Diagnostic hygiene | DEF-153, DEF-160, DEF-161, DEF-162 | correctness, semantics |
| 6 — Close-out | verification, docs, roadmap | none new |

## Requirements Traceability

Every defect row carries its reproduction in the roadmap and its acceptance criterion in
[01-requirements.md](01-requirements.md); each phase document maps its rows to the frozen
specification clauses it conforms to and to the specification tests that pin them.
