<!-- title: [correctness] Sequence effects in source order and give temporaries statement scope -->
<!-- labels: assessment,kind:theme,severity:critical,priority:P0,domain:codegen,domain:ownership,domain:semantics -->
<!-- parent: - -->

## Goal
Sequence effects in source order and release temporaries at statement end: lower effectful subexpressions into ordered temporaries (A-normal form), give owned temporaries statement scope, and emit value-position `++`/`--` through the unsigned form

| Field | Value |
|---|---|
| Roadmap entry | R2 `evaluation-order-and-temporaries` (report §8) |
| Order | 2 of 14 |
| Depends on | — |
| Effort | L |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |

## Findings this plan closes
- ASM-WS1-002 (Critical, DEF-583) — Call arguments with side effects are evaluated right to left — finding issue (sub-issue)
- ASM-WS2-001 (Critical, DEF-584) — Owned temporaries drop at function exit, not at the end of their statement — finding issue (sub-issue)
- ASM-WS3-003 (Major, DEF-602) — A value-position `++`/`--` emits signed C increment, undefined on the host and vbcc legs at the limit — finding issue (sub-issue)

## How to start
Create the plan with `make-plan evaluation-order-and-temporaries`, starting from this issue and the finding issues below it; execute it with `exec-plan`. The report (`plans/independent-assessment/10-assessment-report.md` §5 and §8) explains the theme.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
