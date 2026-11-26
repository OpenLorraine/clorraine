<!-- title: [bug] Generate collision-free C names -->
<!-- labels: assessment,kind:theme,severity:critical,priority:P0,domain:codegen -->
<!-- parent: - -->

## Goal
Escape every C99 keyword and header/NDK macro name and separate generic-instance symbols from module functions

| Field | Value |
|---|---|
| Roadmap entry | R3 `c-name-integrity` (report §8) |
| Order | 3 of 14 |
| Depends on | — |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |

## Findings this plan closes
- ASM-WS3-001 (Critical, DEF-589) — Locals named like C keywords or C/NDK macros make the emitted C invalid — finding issue (sub-issue)
- ASM-WS3-002 (Critical, DEF-592) — A module function can collide with a generic instance symbol — finding issue (sub-issue)
- ASM-WS3-005 (Minor, no DEF) — Appended runtime code inherits the last `#line`, so its diagnostics and debug lines point into `.clo` files — listed in a tracking issue
- ASM-WS3-006 (Minor, no DEF) — The emitted C depends on how the source path is spelled and embeds absolute paths — listed in a tracking issue

## How to start
Create the plan with `make-plan c-name-integrity`, starting from this issue and the finding issues below it; execute it with `exec-plan`. The report (`plans/independent-assessment/10-assessment-report.md` §5 and §8) explains the theme.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
