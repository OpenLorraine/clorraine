# WS7 — Runtime and compile-time performance

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-03-runtime-performance-workstreams.md §WS7 (WS7-01..WS7-07)

## Scope and limits

Runtime cost on the Phase 0 lenses (the matrix in `records/WS0-matrix.json` is the
source for instruction counts and section sizes, PF-008), host wall time (probe 700),
A500 footprint (730), and compile time and peak RSS of the pinned release `clc` (710 and
the Phase 0 `compile_time.py` record). The FS-UAE frame-timing probe is not written:
WS5 and WS6 recorded no suspected cost in the display paths (03-03 WS7-05 makes the
probe conditional on one).

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | high | 2026-11-26 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Probes written (7.1.1) | `probes/WS7/700`, `710`, `730` | done |

## Checklist outcomes

_(filled in Step 7.2)_

## Findings

_(filled in Step 7.2)_

## Spec questions

_(filled in Step 7.2)_

## Falsifiability log

_(filled in task 7.1.2)_

## Escalations

None yet.

## Probe index

| Item | Probes |
|------|--------|
| WS7-01 host time | 700 |
| WS7-02 footprint | 730 |
| WS7-06/07 compile scaling | 710 (+ Phase 0 `records/assessment-baseline-compile.json`) |
