<!-- title: [performance] Lower module initializers and the fault writer compactly -->
<!-- labels: assessment,kind:theme,severity:major,priority:P1,domain:optimizer,domain:perf -->
<!-- parent: - -->

## Goal
Lower module fills like local fills and constant tables as initialized statics; a fault writer without `printf`; drop unused libamiga stubs

| Field | Value |
|---|---|
| Roadmap entry | R12 `initializer-and-runtime-size` (report §8) |
| Order | 12 of 14 |
| Depends on | — |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |

## Findings this plan closes
- ASM-WS7-001 (Major, DEF-596) — A module-level fill emits one assignment per element: a 64 KB table needs 548 KB of code and 163 s of GCC — finding issue (sub-issue)
- ASM-WS4-004 (Opportunity, no DEF) — A module-level table with a constant initializer occupies RAM twice — report only
- ASM-WS4-003 (Opportunity, no DEF) — The runtime fault writer pulls 16.6 KB of `printf` and soft-float code into every checked program — finding issue (sub-issue)
- ASM-WS4-005 (Opportunity, no DEF) — Every Amiga program links libamiga DOS stubs and `CreatePort` it does not use — finding issue (sub-issue)

## How to start
Create the plan with `make-plan initializer-and-runtime-size`, starting from this issue and the finding issues below it; execute it with `exec-plan`. The report (`plans/independent-assessment/10-assessment-report.md` §5 and §8) explains the theme.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
