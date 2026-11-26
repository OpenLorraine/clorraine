# Findings ledger: amiga-fault-exit-cleanup

| Finding | Phase | Severity | Area | Ruling |
|---------|-------|----------|------|--------|
| SR-001 | 1 | MAJOR | shim_hosted_amiga.c:cl_release_msg_port | accepted (push hold until Phase 3; ARexx keep-port in 3.2.4) |
| RV-001 | 1 | MINOR | shim_hosted_amiga.c:cl_release_msg_port | accepted (same as SR-001) |
| SR-002 | 1 | MINOR | ST-12 window-fault.clo | fixed (console added to ST-12 probe, Phase 2) |
| SR-Q1 | 1 | MINOR | 03-01 vbcc nested exit | fixed (libnix-only scope noted) |
| RV-002 | 1 | MINOR | plans/00-roadmap.md | fixed |
| RV-001 (P2) | 2 | MAJOR | fault-cleanup-walker.c:diskfont, library-close phase | fixed (AR-25: vamos C-harness test; diskfont field dropped; docs corrected) |
| SR-001 (P2) | 2 | MAJOR | same as RV-001 (P2) | fixed |
| RV-002 (P2) | 2 | MINOR | a500-fault-cleanup-smoke.sh stop timing | fixed (settle loop) |
| RV-003 (P2) | 2 | MINOR | 99-execution-plan.md stale Blocked text | fixed |
| SR-Q4 (P2) | 2 | MINOR | kinds without target evidence | fixed (AR-25: ST-20 device-fault.clo) |
| SR-Q1 (P2) | 2 | MINOR | AR-24 cause | fixed (recorded as unverified) |
| SR-Q2/P-1 (P2) | 2 | MINOR | layer mid-update free; sample before audio stop | deferred (pre-existing; strict scope) |
| SR-001 (P2 re-review) | 2 | MAJOR | fault-cleanup-walker.c:chip blind to slow RAM | fixed (mem field; red/green and Layer mutation proven) |
| SR-002 (P2 re-review) | 2 | MINOR | docs: open-count claim | fixed (scoped to diskfont) |
| SR-003 (P2 re-review) | 2 | MINOR | gate: chip column wording | fixed |
