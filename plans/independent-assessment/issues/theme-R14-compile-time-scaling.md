<!-- title: [performance] Keep compile time near-linear -->
<!-- labels: assessment,kind:theme,severity:major,priority:P1,domain:perf -->
<!-- parent: - -->

## Goal
Index names and instances once per module/program so compile time stays near linear

| Field | Value |
|---|---|
| Roadmap entry | R14 `compile-time-scaling` (report §8) |
| Order | 14 of 14 |
| Depends on | — |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |

## Findings this plan closes
- ASM-WS7-002 (Major, DEF-610) — Compile time grows about quadratically for call-heavy and generic code — finding issue (sub-issue)

## How to start
Create the plan with `make-plan compile-time-scaling`, starting from this issue and the finding issues below it; execute it with `exec-plan`. The report (`plans/independent-assessment/10-assessment-report.md` §5 and §8) explains the theme.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
