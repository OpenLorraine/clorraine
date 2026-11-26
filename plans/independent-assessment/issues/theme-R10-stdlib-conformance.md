<!-- title: [correctness] Bring the standard library in line with its specification -->
<!-- labels: assessment,kind:theme,severity:major,priority:P1,domain:codegen,domain:stdlib -->
<!-- parent: - -->

## Goal
Report read errors, give `str` its `slice`, unify NaN text, and bring the appendix in line with the exports

| Field | Value |
|---|---|
| Roadmap entry | R10 `stdlib-conformance` (report §8) |
| Order | 10 of 14 |
| Depends on | — |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |

## Findings this plan closes
- ASM-WS6-001 (Major, DEF-594) — `readFile` reports a read error exactly like end of file — finding issue (sub-issue)
- ASM-WS6-002 (Major, DEF-603) — `slice` of a `str` is rejected, although the appendix gives it the same view type — finding issue (sub-issue)
- ASM-WS6-003 (Minor, no DEF) — NaN prints as `-nan` on the host and `NaN` on the Amiga profiles — listed in a tracking issue
- ASM-WS6-004 (Minor, no DEF) — 25 public functions, including `std.io`'s writers, have no specification entry — listed in a tracking issue
- ASM-WS6-005 (Minor, no DEF) — The appendix's Kickstart minimums and Box signatures drift from the code — listed in a tracking issue
- ASM-WS3-004 (Minor, no DEF) — On the host profile `usize` is 32 bits, so a pointer round trip through `usize` crashes — listed in a tracking issue

## How to start
Create the plan with `make-plan stdlib-conformance`, starting from this issue and the finding issues below it; execute it with `exec-plan`. The report (`plans/independent-assessment/10-assessment-report.md` §5 and §8) explains the theme.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
