# WS0 — Baseline, pins, C references, measurement matrix

> **Baseline**: e24a8752b08827c3a5d7c77b6e2827dac708b629
> **Baseline date**: 2026-11-25 (commit date 2026-11-25 18:50 +0100)
> **Pinned on**: 2026-11-25 19:07, after the user confirmed the working tree was settled (AR #7, AR #30)

## Scope and limits

Phase 0 pins the baseline and the tools, writes the equivalent C references and the
measurement script, captures the emission record, harness numbers, and the measurement
matrix that every later phase cites (03-01 §Ground rules, §Measurement rules).

## Dispatch record

| Role | Effort | Date |
|------|--------|------|
| Executing session (inline) | medium | 2026-11-25 |

## Coverage table

| Item | Status |
|------|--------|
| Baseline and drift check (0.1.1) | done |

## Checklist outcomes

WS0 has no checklist IDs; its observations are filed as `ASM-WS0-<NNN>` findings with
checklist item `WS0`.

## Findings

None yet.

## Spec questions

None.

## Falsifiability log

| Probe | Form | Result |
|-------|------|--------|
| `probes/WS0/drift-check.sh` | seeded defect: an untracked file at the repository root, and an untracked folder `plans/zz-drift-probe/` | both rejected with exit 1 and the path named; clean tree passes with exit 0 (2026-11-25 19:08) |

## Escalations

None.

## Drift check

`probes/WS0/drift-check.sh <plan-folder> [<baseline-sha>]` implements 03-01 §Ground rules.
