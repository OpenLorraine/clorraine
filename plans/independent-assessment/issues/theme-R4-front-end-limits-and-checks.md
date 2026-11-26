<!-- title: [correctness] Bound the front end and enforce its checks on every route -->
<!-- labels: assessment,kind:theme,severity:critical,priority:P0,domain:ownership,domain:semantics -->
<!-- parent: - -->

## Goal
Bound `clc`'s recursion, report E10305 on every instantiation route, range-check inferred literals, reject field moves out of call results (E10026), and document `embed` as staged with its promised diagnostic

| Field | Value |
|---|---|
| Roadmap entry | R4 `front-end-limits-and-checks` (report §8) |
| Order | 4 of 14 |
| Depends on | — |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |

## Findings this plan closes
- ASM-WS1-004 (Critical, DEF-590) — `clc` aborts with a stack overflow on moderately deep nesting — finding issue (sub-issue)
- ASM-WS1-005 (Critical, DEF-591) — Polymorphic recursion with explicit type arguments hangs `clc` instead of E10305 — finding issue (sub-issue)
- ASM-WS1-001 (Major, DEF-601) — An inferred-type decimal literal above `i32` range is accepted and silently wraps — finding issue (sub-issue)
- ASM-WS2-002 (Minor, no DEF) — A field move out of a call result is accepted — listed in a tracking issue
- ASM-WS1-003 (Minor, no DEF) — Data inclusion (`embed`) is staged out, but chapter 13 reads stable and the limitation is undocumented — listed in a tracking issue

## How to start
Create the plan with `make-plan front-end-limits-and-checks`, starting from this issue and the finding issues below it; execute it with `exec-plan`. The report (`plans/independent-assessment/10-assessment-report.md` §5 and §8) explains the theme.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
