# Findings ledger: amiga-fault-exit-cleanup

| Finding | Phase | Severity | Area | Ruling |
|---------|-------|----------|------|--------|
| SR-001 | 1 | MAJOR | shim_hosted_amiga.c:cl_release_msg_port | accepted (push hold until Phase 3; ARexx keep-port in 3.2.4) |
| RV-001 | 1 | MINOR | shim_hosted_amiga.c:cl_release_msg_port | accepted (same as SR-001) |
| SR-002 | 1 | MINOR | ST-12 window-fault.clo | fixed (console added to ST-12 probe, Phase 2) |
| SR-Q1 | 1 | MINOR | 03-01 vbcc nested exit | fixed (libnix-only scope noted) |
| RV-002 | 1 | MINOR | plans/00-roadmap.md | fixed |
