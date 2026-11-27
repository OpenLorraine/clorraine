# Findings Ledger: VS Code Marketplace Extension and the v1.3.0 Release

| Finding | Phase | Severity | Area | Ruling |
|---------|-------|----------|------|--------|
| RV-001 | 1 | MINOR | tools/vscode/test/publish-workflow.spec.test.ts | fixed |
| SA-001 | 1 | MINOR | .github/workflows/publish-vscode.yml | fixed |

Phase 1 review: correctness reviewer and security auditor dispatched as `general` agents
(catalog roles unavailable in this session); no critical or major findings; both minor findings
fixed (exact publish condition and hash-step assertions; `shell: bash` with pipefail and a 64-hex
check before the comparison); `make vscode-check` passed after the fixes.
