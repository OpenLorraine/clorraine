<!-- title: [bug] Keep large dispatch code within vbcc's limits -->
<!-- labels: assessment,kind:theme,severity:major,priority:P1,domain:codegen -->
<!-- parent: - -->

## Goal
Keep large dispatch code within vbcc's limits: flat `else if` emission and a bounded `switch` shape

| Field | Value |
|---|---|
| Roadmap entry | R11 `vbcc-leg-limits` (report §8) |
| Order | 11 of 14 |
| Depends on | R6 (or standalone) |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |

## Findings this plan closes
- ASM-WS3-008 (Major, DEF-608) — An `else if` chain of 127 or more arms makes vbcc crash or reject the emitted C — finding issue (sub-issue)
- ASM-WS3-007 (Major, DEF-609) — vbcc compile time grows about cubically with `switch` size — finding issue (sub-issue)

## How to start
Create the plan with `make-plan vbcc-leg-limits`, starting from this issue and the finding issues below it; execute it with `exec-plan`. The report (`plans/independent-assessment/10-assessment-report.md` §5 and §8) explains the theme.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
