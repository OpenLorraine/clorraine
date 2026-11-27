# Findings Ledger: Windows CLI and installer

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
| RV-001 | 3 | MINOR | compiler/tests/windows_cli_impl_test.rs | fixed |
| RV-002 | 3 | MINOR | compiler/src/clorraine/setup.rs | fixed |
| RV-003 | 3 | MINOR | compiler/src/clorraine/docgen.rs | fixed |
| SA-002 | 3 | MINOR | compiler/src/clorraine/platform.rs | fixed |
| SR-001 | 3 | MINOR | compiler/src/clorraine/emulator/storage.rs | fixed |
| SR-002 | 3 | MINOR | compiler/src/clorraine/emulator/mod.rs | fixed |
| SR-003 | 3 | MINOR | compiler/src/clorraine/platform.rs | accepted |
| RV-001 | 4 | MINOR | tools/vscode/src/actions.ts | fixed |
| RV-002 | 4 | MINOR | tools/vscode/src | accepted |
| RV-001 | 5 | MAJOR | tools/setup/install.ps1 | fixed |
| RV-002 | 5 | MAJOR | tools/setup/tests/install.spec.test.ps1 | fixed |
| RV-003 | 5 | MINOR | tools/setup/install.ps1 | fixed |
| RV-004 | 5 | MINOR | tools/setup/install.ps1 | fixed |
| RV-005 | 5 | MINOR | tools/setup/install.ps1 | fixed |
| SA-001 | 5 | MAJOR | tools/setup/install.ps1 | fixed |
| SA-002 | 5 | MINOR | tools/setup/install.ps1 | fixed |
| SA-003 | 5 | MINOR | tools/setup/install.ps1 | fixed |
| SA-004 | 5 | MINOR | tools/setup/install.ps1 | fixed |
| SA-005 | 5 | MINOR | tools/setup/install.ps1 | fixed |
| SA-006 | 5 | MINOR | tools/setup/install.ps1 | fixed |
| RV-006 | 5 | MAJOR | tools/setup/install.ps1 | fixed |
| SA-007 | 5 | MAJOR | tools/setup/install.ps1 | fixed |
| SA-008 | 5 | MINOR | tools/setup/install.ps1 | fixed |
| SA-009 | 5 | MINOR | tools/setup/install.ps1 | accepted |
| SA-010 | 5 | MINOR | tools/setup/tests/install.impl.test.ps1 | fixed |
| RV-001 | 6 | MAJOR | tools/setup/install.ps1 | fixed |
| RV-002 | 6 | MAJOR | tools/setup/install.ps1 | fixed |
| RV-003 | 6 | MINOR | tools/setup/install.ps1 | fixed |
| RV-004 | 6 | MINOR | tools/setup/install.ps1 | fixed |
| RV-005 | 6 | MINOR | tools/setup/install.ps1 | fixed |
| RV-006 | 6 | MINOR | tools/setup/install.ps1 | fixed |
| RV-007 | 6 | MINOR | tools/setup/install.ps1 | fixed |
| RV-008 | 6 | MINOR | tools/setup/tests | fixed |
| SA-001 | 6 | MAJOR | tools/setup/install.ps1 | fixed |
| SA-002 | 6 | MAJOR | tools/setup/install.ps1 | fixed |
| SA-003 | 6 | MAJOR | tools/setup/install.ps1 | fixed |
| SA-004 | 6 | MINOR | tools/setup/install.ps1 | fixed |
| SA-005 | 6 | MINOR | tools/setup/install.ps1 | fixed |
| SA-006 | 6 | MINOR | tools/setup/install.ps1 | fixed (AR-61) |
| SA-007 | 6 | MINOR | tools/setup/tests | fixed |
| SA-008 | 6 | MINOR | tools/setup/install.ps1 | fixed |
| RV-101 | 6 | MINOR | tools/setup/install.ps1 | fixed |
| RV-102 | 6 | MINOR | tools/setup/install.ps1 | fixed |
| RV-103 | 6 | MINOR | plans/windows-cli/03-04-installer.md | fixed |
| RV-701 | 7 | MAJOR | tools/setup/acceptance.ps1 | fixed |
| RV-702 | 7 | MAJOR | tools/setup/acceptance.ps1 | fixed |
| RV-703 | 7 | MINOR | tools/setup/pe_check.py | fixed |
| RV-704 | 7 | MINOR | tools/setup/package.sh | fixed |
| RV-705 | 7 | MINOR | tools/setup/package.sh | fixed |
| RV-706 | 7 | MINOR | tools/setup/acceptance.ps1 | fixed |
| RV-707 | 7 | MINOR | tools/setup/acceptance.ps1 | fixed |
| RV-708 | 7 | MINOR | tools/setup/tests | fixed |
| RV-709 | 7 | MINOR | tools/setup/tests | fixed |
| RV-710 | 7 | MINOR | tools/setup/acceptance.py | fixed |
| RV-711 | 7 | MINOR | tools/setup/package.sh | fixed |
| RV-712 | 7 | MINOR | tools/setup/acceptance.ps1 | fixed |
| DR-801 | 8 | CRITICAL | docs, README.md, site | fixed (user ruling, AR-63) |
| DR-802 | 8 | MAJOR | docs, site, tools/setup/install.ps1 | fixed |
| DR-803 | 8 | MAJOR | docs, site, tools/setup/install.ps1 | fixed |
| DR-804 | 8 | MINOR | docs, site, tools/setup/install.ps1 | fixed |
| DR-805 | 8 | MINOR | docs, site, tools/setup/install.ps1 | fixed |
| DR-806 | 8 | MINOR | docs, site, tools/setup/install.ps1 | fixed |
| DR-807 | 8 | MINOR | docs, site, tools/setup/install.ps1 | fixed |
| DR-808 | 8 | MINOR | docs, site, tools/setup/install.ps1 | fixed |
| DR-809 | 8 | MINOR | docs, site, tools/setup/install.ps1 | fixed |
| DR-810 | 8 | MINOR | docs, site, tools/setup/install.ps1 | fixed |
| DR-811 | 8 | MINOR | docs, site, tools/setup/install.ps1 | fixed |
| DR-812 | 8 | MINOR | docs, site, tools/setup/install.ps1 | fixed |
| DR-813 | 8 | MINOR | docs, site, tools/setup/install.ps1 | fixed |
| DR-814 | 8 | MINOR | docs, site, tools/setup/install.ps1 | fixed |
| AC-901 | 9 | MAJOR | tools/setup/install.ps1, compiler/src/clorraine/setup.rs | fixed (user ruling, AR-64) |
| RV-901 | 9 | CRITICAL | tools/setup/install.ps1 | fixed |
| RV-902 | 9 | MAJOR | tools/setup/install.ps1 | fixed |
| RV-903 | 9 | MINOR | tools/setup/install.ps1 | fixed |
| RV-904 | 9 | MINOR | tools/setup/install.ps1 | fixed |
| RV-905 | 9 | MINOR | compiler/src/clorraine/environment.rs | accepted: needs a hand-edited config; exit 2 with the reason |
| AC-902 | 9 | MAJOR | compiler/src/clorraine (platform, setup, build, emulator/storage) | fixed |
| RV-911 | 9 | MINOR | compiler/src/clorraine | fixed |
| RV-912 | 9 | MINOR | compiler/src/clorraine | fixed |
| RV-913 | 9 | MINOR | compiler/src/clorraine | fixed |
| RV-914 | 9 | MINOR | compiler/src/clorraine | fixed |
| RV-915 | 9 | MINOR | compiler/src/clorraine | fixed |
| AC-903 | 9 | MAJOR | compiler/src/clorraine/emulator/select.rs, tools/setup/install.ps1 | fixed (Windows only; Linux follow-up in 08-merge-notes.md) |
