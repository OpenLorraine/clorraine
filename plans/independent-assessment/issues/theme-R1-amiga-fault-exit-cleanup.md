<!-- title: [bug] Release OS resources on every Amiga program exit -->
<!-- labels: assessment,kind:theme,severity:critical,priority:P0,domain:shims,amiga-sdk -->
<!-- parent: - -->

## Goal
Release every OS registration on every exit: a shim resource registry walked by the fault path and an exit hook; worker-task faults handled in the task; a Ctrl-C break surface in the blocking waits

| Field | Value |
|---|---|
| Roadmap entry | R1 `amiga-fault-exit-cleanup` (report §8) |
| Order | 1 of 14 |
| Depends on | — |
| Effort | L |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |

## Findings this plan closes
- ASM-WS5-001 (Critical, DEF-582) — A runtime fault exits with OS registrations still installed: a VBlank server is left pointing into the unloaded program — finding issue (sub-issue)
- ASM-WS5-005 (Critical, DEF-586) — A runtime fault inside a worker task runs libnix `printf` and `exit()` from a plain Exec task — finding issue (sub-issue)
- ASM-WS5-013 (Major, DEF-604) — The shim's own library bases are never closed — finding issue (sub-issue)
- ASM-WS5-020 (Minor, no DEF) — No Ctrl-C handling in any blocking shim wait — listed in a tracking issue

## How to start
Create the plan with `make-plan amiga-fault-exit-cleanup`, starting from this issue and the finding issues below it; execute it with `exec-plan`. The report (`plans/independent-assessment/10-assessment-report.md` §5 and §8) explains the theme.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
