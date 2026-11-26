<!-- title: [correctness] Make the ownership pass the single drop authority -->
<!-- labels: assessment,kind:theme,severity:minor,priority:P3,domain:ownership -->
<!-- parent: - -->

## Goal
Make the ownership pass the only place that decides moves and drops; codegen emits recorded drops only

| Field | Value |
|---|---|
| Roadmap entry | R7 `single-drop-authority` (report §8) |
| Order | 7 of 14 |
| Depends on | R2 |
| Effort | XL |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |

## Findings this plan closes
- ASM-WS2-003 (Minor, no DEF) — Move, copy, and drop decisions are re-derived at about 230 sites in two stages with different predicates — listed in a tracking issue

## How to start
Create the plan with `make-plan single-drop-authority`, starting from this issue and the finding issues below it; execute it with `exec-plan`. The report (`plans/independent-assessment/10-assessment-report.md` §5 and §8) explains the theme.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
