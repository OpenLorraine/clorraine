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
| SR-001 (P3) | 3 | MAJOR | shim_hosted_amiga.c:cl_release_msg_port foreign messages | fixed (drain and reply; ARexx reply filter sets RC_FATAL) |
| SR-002 (P3) | 3 | MAJOR | shim_hosted_amiga.c:kept port still live | fixed (RemPort, PA_IGNORE, SigTask NULL, FreeSignal) |
| SR-003 (P3) | 3 | MINOR | optional_services.c:held received ARexx command | fixed (exit release replies RC_FATAL) |
| RV-001 (P3) | 3 | MINOR | shim_hosted_amiga.c:cl_port_keep duplicates | fixed |
| RV-002 (P3) | 3 | MINOR | 99-execution-plan.md file list | fixed |
| SR-Q1 (P3) | 3 | MINOR | no FS-UAE fault probe for 2.0+ kinds | deferred (AR-7; known-limitations entry in 6.1.3) |
| SR-Q2 (P3) | 3 | MINOR | CL_LIB_TABLE_MAX zero headroom | deferred (strict scope; 20 bases exactly, pinned by tests) |
| SR-001 (P3 re-review) | 3 | MINOR | shim_hosted_amiga.c:public port drain-to-delete race | fixed (DeletePort under Forbid) |
| SR-001/RV-001 (P4) | 4 | MINOR | shim_hosted_amiga.c:cl_worker_fault field stores | fixed (stored under Forbid) |
| SR-Q1 (P4) | 4 | MINOR | shim_hosted_amiga.c:exit worker phase signal bit | fixed (SetSignal clear before FreeSignal) |
| RV-002 (P4) | 4 | MINOR | stale doc comments | fixed |
| RV-003 (P4) | 4 | MINOR | roadmap and plan bookkeeping | fixed |
| SR-Q2 (P4) | 4 | MINOR | ST-14/ST-18 memory not asserted | deferred (strict scope; spec change) |
| SR-PRE (P4) | 4 | MINOR | interrupt-context fault with workers | deferred (AR-4, issue #95) |
| RV-001 (P5) | 5 | MINOR | shim_hosted_amiga.c:cl_wait_io_or_break break order | fixed (break checked before completion) |
| RV-002 (P5) | 5 | MINOR | 00-ambiguity-register.md date | fixed |
| SR-001 (P5) | 5 | MINOR | break-timer.clo header claim | fixed (claim narrowed; abort-and-reap pinned by impl test) |
| SR-002 (P5) | 5 | MINOR | a500-fault-cleanup-smoke.sh header | fixed |
| SR-Q1/Q2 (P5) | 5 | MINOR | Ctrl-C limits (2.0+ waits untested on target; console-only delivery) | fixed in 6.1.3 known-limitations |
| SR-PRE (P5) | 5 | MINOR | cl_gt_next_event filtered message in wait mode | deferred (pre-existing) |
