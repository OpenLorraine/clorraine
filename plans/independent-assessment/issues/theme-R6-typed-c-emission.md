<!-- title: [bug] Emit C through a typed representation -->
<!-- labels: assessment,kind:theme,severity:major,priority:P1,domain:codegen,domain:tests -->
<!-- parent: - -->

## Goal
Emit through a typed C representation that escapes names, bounds nesting, maps lines, and makes invalid shapes unrepresentable

| Field | Value |
|---|---|
| Roadmap entry | R6 `typed-c-emission` (report §8) |
| Order | 6 of 14 |
| Depends on | R3 |
| Effort | XL |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |

## Findings this plan closes
- ASM-WS8-003 (Minor, no DEF) — Memory-safety and invalid-C defects recur because drop and emission decisions are made per expression shape, in two stages — listed in a tracking issue
- ASM-WS3-008 (Major, DEF-608) — An `else if` chain of 127 or more arms makes vbcc crash or reject the emitted C — sub-issue of R11

## How to start
Create the plan with `make-plan typed-c-emission`, starting from this issue and the finding issues below it; execute it with `exec-plan`. The report (`plans/independent-assessment/10-assessment-report.md` §5 and §8) explains the theme.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
