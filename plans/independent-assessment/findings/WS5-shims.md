# WS5 — Runtime shims

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629 (see [WS0-baseline.md](WS0-baseline.md))
> **Checklist**: 03-03-runtime-performance-workstreams.md §WS5 (WS5a = core and 2.0 shims; WS5b = optional shims, host, freestanding, `picasso96.h`)

## Scope and limits

Every shim in `compiler/runtime/` except `emulator_runner.c` (driver surface, AR #9).
Method: function-by-function reading against the checklist (tracked in the coverage
table), `vamos` probes for DOS/Exec/timer/utility paths with the `vamos` leak detector, the
host shim under the WS2 ASan/UBSan probes, and patched-C estimates for hot paths.

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | xhigh | 2026-11-25 |
| `explore` reconnaissance ×3 (read-only; candidates only, every candidate re-verified by the executing session) | inherited | 2026-11-25 |

## Coverage table

| Group | Files | Status |
|-------|-------|--------|
| Probes written (5.1.1, 5.1.2) | `probes/WS5/500`–`520`, `probes/WS4/530` | done |

## Checklist outcomes

_(filled in Step 5.2)_

## Findings

_(filled in Step 5.2)_

## Spec questions

_(filled in Step 5.2)_

## Falsifiability log

_(filled in task 5.1.3)_

## Escalations

None yet.

## Probe index

Runner: `probes/WS1/probe.py --vamos-leaks`.

| Item | Probes |
|------|--------|
| detector | 500 (control), `500-seeded-leak.c` |
| WS5-02/03/08 DOS | 501, 502, 503 |
| WS5-06 Exec ports/messages | 504 |
| WS5-02/06 timer, libraries | 505, 506 |
| WS5b 2.0 services | 510 |
| WS5-11 host shim | the WS2 probes under ASan/UBSan (`records/WS2-results.json`) |
| WS5-12 freestanding | `520-freestanding-link.sh` |
| WS5-13 hot path | `probes/WS4/530-format-u32-16bit.diff` |
