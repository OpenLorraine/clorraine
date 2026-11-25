# WS3 — IR and C codegen correctness

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-02-compiler-workstreams.md §WS3 (WS3-01..WS3-11)

## Scope and limits

Components: `ir/`, `codegen/`, the emitted C. Inputs: the Phase 0 emission (93 accepted
cells, `findings/WS0-emission-record.tsv`) and the WS3 probes. Lens: valid C99 on every
compile-matrix leg, no undefined behavior, 68000 correctness, single-width volatile access.

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | xhigh | 2026-11-25 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Probes written (3.1.1) | `probes/WS3/300`–`380` | done |

## Checklist outcomes

_(filled in Step 3.2)_

## Findings

_(filled in Step 3.2)_

## Spec questions

_(filled in Step 3.2)_

## Falsifiability log

_(filled in task 3.1.2)_

## Escalations

None yet.

## Probe index

| Item | Probes |
|------|--------|
| WS3-01 strict builds | `300-strict-build.sh`, `300-seeded.c` |
| WS3-02 vbcc | `301-vbcc-scan.sh`, `301-seeded.c` |
| WS3-03 UB | 310, 311; WS1 probes 039–043 rerun under UBSan; ASM-WS1-002 |
| WS3-04 aliasing | code reading |
| WS3-05 68000 odd address | `320-odd-address.{c,sh}` |
| WS3-06 volatile/MMIO | 330 + `330-mmio-width.sh` |
| WS3-07 names | 340–344 |
| WS3-08 guards | 350–358 |
| WS3-09 determinism | `360-determinism.sh`; Phase 0 record |
| WS3-10 ABI | 370–372 |
| WS3-11 shapes | `380-shapes.py` (+ `380-seeded-record.tsv`) |
