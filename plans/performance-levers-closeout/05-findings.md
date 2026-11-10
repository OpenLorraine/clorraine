# Findings and Evidence: Build-Flag and Binary-Size Closeout (PLAN-26)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Baseline pending — filled during execution

## Purpose

This document owns the measured evidence for PLAN-26: the baseline, the
same-compiler old-vs-new flag audit, the constructor/section audit, the scored
base/head comparison, the exact acceptance values, the gate evidence, the
review rulings, and the accepted limits.

## Baseline

_Pending Phase 1._ The base is `f9d5be8`; the exploratory probe is recorded in
the requirements and register and is re-run with the committed procedure in
Phase 3.

## Audit Matrix

_Pending Phase 3._ The matrix is produced by `make bench-flags` and recorded
here; the JSON stays under `build/perf/` (gitignored).

## Constructor and Section Audit

_Pending Phase 1/3._ The removed `__amigapath.o`/`environ.o` entries and the
remaining `___INIT_LIST__` entries are recorded here.

## Scored Comparison

_Pending Phase 4._ `base.json` from `f9d5be8` and `head.json` from the final
head, with the full compiler hash, `dirty == false`, and the compare table.

## Gate Evidence

_Pending Phase 4._ `make check`, `make asan`, `make ubsan`, `make
corpus-amiga`, `make differential`, `make bench-shape`, `make bench-base`,
`make bench`, `make bench-compare`, `make bench-flags`, `make verify`, the
release Workbench gates, and the 1.3 release corpus.

## Review Rulings

_Pending Phase 4._ Correctness, emission integrity, and design challenge
findings with their rulings.

## Accepted Limits

_Pending Phase 5._ Freestanding exclusion, non-Linux fallback, stripped
release symbols, and any residual risk.
