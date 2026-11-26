<!-- title: [performance] Close the template and Array gaps to C -->
<!-- labels: assessment,kind:theme,severity:major,priority:P1,domain:optimizer,domain:perf,domain:shims,amiga-sdk -->
<!-- parent: - -->

## Goal
Close the two measured gaps: inline template builder, `Array` push/get lowering, 16-bit integer formatting; revisit release flags

| Field | Value |
|---|---|
| Roadmap entry | R13 `runtime-call-lowering` (report §8) |
| Order | 13 of 14 |
| Depends on | — |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |

## Findings this plan closes
- ASM-WS4-001 (Major, DEF-612) — Templates cost 33% more instructions than equivalent C on the A500 — finding issue (sub-issue)
- ASM-WS4-002 (Major, DEF-611) — `Array<T>` push and get cost 39% more instructions than equivalent C on the A500 — finding issue (sub-issue)
- ASM-WS5-022 (Opportunity, no DEF) — Integer formatting divides with libgcc's 32-bit routine on the 68000 — finding issue (sub-issue)
- ASM-WS4-006 (Opportunity, no DEF) — `-O3` and `-flto` beat the shipped release flags on the A500 — report only
- ASM-WS7-003 (Opportunity, no DEF) — The 68020 probe build of `container` runs 21% more instructions than the 68000 build — report only

## How to start
Create the plan with `make-plan runtime-call-lowering`, starting from this issue and the finding issues below it; execute it with `exec-plan`. The report (`plans/independent-assessment/10-assessment-report.md` §5 and §8) explains the theme.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
