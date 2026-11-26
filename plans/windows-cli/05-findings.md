# Findings Ledger: Windows CLI and installer

SA-001 is the memory-safety audit's trigraph note (W2-M1), the same root cause as the Phase 2
correctness PE-1 and emission EI-P01; fixed under AR-57. EI-001 is vbcc's literal display of
escaped names in its own diagnostics; builds and runs are unaffected.

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| RV-001 | 1 | CRITICAL | tools/setup/windows-remote.sh | fixed |
| RV-002 | 1 | MAJOR | tools/setup/windows-desktop-run.ps1 | fixed |
| RV-003 | 1 | MINOR | tools/setup/windows-remote.sh | fixed |
| RV-004 | 1 | MINOR | tools/setup/windows-desktop-run.ps1 | fixed |
| RV-005 | 1 | MINOR | tools/setup/windows-desktop-run.ps1 | fixed |
| RV-001 | 2 | MINOR | compiler/src/codegen.rs | fixed |
| RV-002 | 2 | MINOR | compiler/tests/windows_cli_spec_test.rs | fixed |
| RV-003 | 2 | MINOR | compiler/runtime/shim_freestanding.c | fixed |
| SA-001 | 2 | MINOR | compiler/src/codegen.rs | fixed |
| EI-001 | 2 | MINOR | vbcc | accepted |
