# Findings Ledger: VS Code Marketplace Extension and the v1.3.0 Release

| Finding | Phase | Severity | Area | Ruling |
|---------|-------|----------|------|--------|
| RV-001 | 1 | MINOR | tools/vscode/test/publish-workflow.spec.test.ts | fixed |
| SA-001 | 1 | MINOR | .github/workflows/publish-vscode.yml | fixed |
| RV-002 | 2 | MINOR | plans/vscode-marketplace-v1.3.0/03-01-extension.md | fixed |
| RV-003 | 3 | MINOR | compiler/tests/first_run_spec_test.rs | fixed |
| RV-004 | 3 | MINOR | compiler/tests/first_run_spec_test.rs | accepted |
| RV-005 | 3 | MINOR | compiler/templates/starter/README.md | fixed |
| RV-006 | 4 | MINOR | tools/setup/install.ps1 | fixed |
| RV-007 | 4 | MINOR | tools/setup/install.ps1 | fixed |
| RV-008 | 4 | MINOR | tools/setup/install.ps1 | fixed |
| RV-009 | 4 | MINOR | tools/setup/install.ps1 | fixed |
| RV-010 | 4 | MINOR | tools/setup/tests | fixed |
| RV-011 | 4 | MINOR | tools/setup/install.sh | accepted |
| SA-002 | 4 | MINOR | tools/setup/install.ps1 | fixed |
| SA-003 | 4 | MINOR | tools/setup/install.ps1 | fixed |
| RV-012 | 5 | MINOR | tools/setup/acceptance.ps1 | fixed |
| RV-013 | 5 | MINOR | tools/setup/acceptance.ps1 | fixed |
| RV-014 | 5 | MINOR | tools/setup/acceptance.py | fixed |
| RV-015 | 5 | MINOR | Makefile | fixed |
| RV-016 | 5 | MINOR | tools/setup/acceptance.sh | fixed |
| RV-017 | 6 | MAJOR | docs/known-limitations.md | fixed |
| RV-018 | 6 | MINOR | docs | fixed |
| RV-019 | 6 | MINOR | RELEASE.md | fixed |
| RV-020 | 6 | MINOR | RELEASE.md | fixed |
| RV-021 | 6 | MINOR | RELEASE.md | fixed |
| RV-022 | 6 | MINOR | site | fixed |
| RV-023 | 6 | MINOR | tools/setup/install.ps1 | fixed |
| RV-024 | 6 | MINOR | tools/assessment/template | accepted |

Phase 1 review: correctness reviewer and security auditor dispatched as `general` agents
(catalog roles unavailable in this session); no critical or major findings; both minor findings
fixed (exact publish condition and hash-step assertions; `shell: bash` with pipefail and a 64-hex
check before the comparison); `make vscode-check` passed after the fixes.

Phase 2 review: correctness reviewer (`general` agent); security auditor skipped — the phase adds no
input surface (the only external action opens one of three constant URLs). No critical or major
findings; mutation runs confirmed ST-16, ST-18, ST-19 fail without the behaviour. The minor finding
(the spec named a different banner colour and description wording than the manifest) was fixed in
the spec. Reported, not a defect: with nothing installed a task shows both missing-tool errors.

Phase 3 review: correctness reviewer (`general` agent); no critical or major findings. Fixed: an
outdated assertion message; the starter README and the Linux guide now say the tasks come from the
Clorraine extension. Accepted: the starter spec checks that both task commands are the extension
variable rather than scanning for every absolute path (no other field can hold a program path).
Gate: `make check` passed (4,529 Rust tests plus the Python, installer, bench, and fuzz suites).

Phase 4 review: correctness reviewer and security auditor (`general` agents); no critical or major
findings. Fixed: taskkill output can no longer end the Windows installer under its Stop preference,
and the VS Code step has a backstop that turns any unexpected error into a warning (RV-006/SA-002);
the editor bound is capped at 86400 s on both platforms, keeping the Windows millisecond wait in
range (RV-007/SA-003); output after a kill is collected for at most 5 s (RV-008); the Windows
"not found" message matches the spec (RV-009); both suites now cover a failed uninstall (RV-010).
Accepted: Ctrl-C does not reach a Linux Marketplace install already under way; it stops at the
bound, and the trade-off is documented in the installer (RV-011). Re-verified: Linux spec 199 and
impl 40 checks; Windows spec 37/37 and impl 37/37.

Phase 5 review: correctness reviewer (`general` agent); security auditor skipped — the phase changes
packaging and evidence validation, adds no input surface beyond two operator-supplied acceptance
arguments that are validated, and removes the VSIX download. No critical or major findings. Fixed:
the Windows acceptance lists extensions through the installer's own Find-VSCode/Invoke-Editor instead
of cmd.exe (verified on the Windows host), and the listing moved above the UAC comment; the publish
run must be ASCII digits; `make dist` clears assets from an earlier candidate; extension failures
point at the install logs. Gate: `make check` passed (4,529 Rust tests plus the setup, dist, bench,
and fuzz suites); a real `make dist` wrote exactly four assets and a four-line SHA256SUMS, and
`make dist-verify` passed.

Phase 6 review: correctness reviewer (`general` agent), docs-focused; security auditor skipped
(documentation and a version bump). One major finding, fixed under --auto-design as a documentation
correction: the known-limitations "Installer integrity" row still claimed the extension was checked
against a pinned hash; it now says the CLI archive and rc24 are pinned and the extension is trusted
through the Marketplace and VS Code. Minor fixes: the five-minute bound applies per VS Code command;
RELEASE.md keeps the asset table out of the tagged commit, deletes and re-pushes a tag only before the
publish, and orders the Linux acceptance as run, in-person check, rerun; remaining "Linux installer"
wording and v1.2 headers; the Windows installer's help and header use the documented commands.
Accepted: the assessment template's "shipped v1.2.0" sentence is outside this plan's scope.
Gate: make check (4,529 Rust tests), make vscode-check (224), make site-check; after the fixes
make dist-check, make site-check, the docs tests, the retained-inputs guard, and the Windows spec
suite (37/37) passed again.
