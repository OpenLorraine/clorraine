# Findings Ledger: VS Code Marketplace Extension and the v1.3.0 Release

| Finding | Phase | Severity | Area | Ruling |
|---------|-------|----------|------|--------|
| RV-001 | 1 | MINOR | tools/vscode/test/publish-workflow.spec.test.ts | fixed |
| SA-001 | 1 | MINOR | .github/workflows/publish-vscode.yml | fixed |
| RV-002 | 2 | MINOR | plans/vscode-marketplace-v1.3.0/03-01-extension.md | fixed |

Phase 1 review: correctness reviewer and security auditor dispatched as `general` agents
(catalog roles unavailable in this session); no critical or major findings; both minor findings
fixed (exact publish condition and hash-step assertions; `shell: bash` with pipefail and a 64-hex
check before the comparison); `make vscode-check` passed after the fixes.

Phase 2 review: correctness reviewer (`general` agent); security auditor skipped — the phase adds no
input surface (the only external action opens one of three constant URLs). No critical or major
findings; mutation runs confirmed ST-16, ST-18, ST-19 fail without the behaviour. The minor finding
(the spec named a different banner colour and description wording than the manifest) was fixed in
the spec. Reported, not a defect: with nothing installed a task shows both missing-tool errors.
