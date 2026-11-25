# WS6 — Standard library

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-03-runtime-performance-workstreams.md §WS6 (WS6-01..WS6-07)

## Scope and limits

The 43 embedded modules under `stdlib/` (about 490 public functions and 64 handle
types), against `spec/appendix-stdlib.md` and `spec/appendix-amiga.md`. Method: scripted
API, gating, handle, and embedding checks (probes 600, 640, 650, 670) reviewed by hand;
core-module boundary probes on host (debug, release, ASan, UBSan) and on both Amiga
profiles under `vamos` with the leak detector; scaling probes on the A500 lens.
Behavior of the C shims behind the wrappers is WS5's; WS6 cites those findings where a
wrapper inherits them.

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | high | 2026-11-26 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Probes written (6.1.1) | `probes/WS6/600`–`670`, `run.sh` | done |

## Checklist outcomes

_(filled in Step 6.2)_

## Findings

_(filled in Step 6.2)_

## Spec questions

_(filled in Step 6.2)_

## Falsifiability log

_(filled in task 6.1.2)_

## Escalations

None yet.

## Probe index

Runner: `probes/WS6/run.sh <clc> <out> main|contrast|seeded`.

| Item | Probes |
|------|--------|
| WS6-01 API conformance | 600 (script), 612 |
| WS6-02 core boundaries | 610 (`std.fmt`), 611 (float text), 612 (`toCString`), 613 (`spanOf`), 614 (`std.math`), 615 (`std.iff`), 616 (`std.mem`) |
| WS6-03 hidden costs | 620 (scaling, A500) |
| WS6-04 Kickstart gating | 640 (script) |
| WS6-05 handle ownership | 650 (script), 651 (drops under `vamos`) |
| WS6-07 embedding and skew | 670 (script) |
