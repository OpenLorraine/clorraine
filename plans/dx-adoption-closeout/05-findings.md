# Findings: DX and Adoption Closeout

## Phase review ledger

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| RV-001 | 1 | MINOR | `compiler/src/analysis.rs` | fixed — `Analysis::programs` documented for the full-analysis case |
| SR-001 | 1 | MINOR | `compiler/src/analysis.rs` | fixed — same documentation correction |
| SR-002 | 1 | MINOR | `compiler/src/analysis.rs` | fixed — `Analysis::typed` documents that ownership diagnostics may remain; pinned by a unit test |
| PRE-001 | 1 | — | parser recovery | pre-existing and out of phase scope; recorded only |
| RV-001 | 2 | MAJOR | `tools/setup/install_sdk.py` | fixed — JSON fetch emits one object; progress goes to stderr |
| RV-002 | 2 | MINOR | `tools/setup/install_sdk.py` | fixed — unreadable/invalid UTF-8 `.env` and missing media root are reported missing |
| RV-003 | 2 | MINOR | `tools/setup/install_sdk.py` | fixed — a provenance mismatch with an empty plan prints the recovery |
| RV-004 | 2 | MINOR | `tools/setup/install_sdk.py` | fixed — tool versions and the download boundary are reported |
| RV-005 | 2 | MINOR | `tools/setup/tests/` | fixed — row order, missing components, init plan, one-object JSON, and post-fetch provenance are pinned |
| RV-006 | 2 | MINOR | plan records | fixed — recorded commands and coverage corrected |
| RV-001 | 3 | MAJOR | `compiler/src/lsp/json.rs` | fixed — linear string parsing; large-string test |
| RV-002 | 3 | MAJOR | `compiler/src/lsp/server.rs` | fixed — requests after shutdown are rejected; document notifications are gated on initialize |
| RV-003 | 3 | MAJOR | `compiler/src/lsp/workspace.rs` | fixed — overlay metadata drives the analysis closure |
| RV-004 | 3 | MINOR | `compiler/src/lsp/workspace.rs` | fixed — analysis and index caps enforced during collection |
| RV-005 | 3 | MINOR | `compiler/src/lsp/server.rs` | fixed — consumed cancellation keys are removed; the pending set is capped |
| RV-006 | 3 | MINOR | tests/plan | fixed — rootUri, null shutdown result, no-response, size-cap tests; inline unit-test path corrected |
| RV-007 | 3 | MINOR | `compiler/src/lsp/server.rs` | fixed — write failures stop the loop with exit 1 |
| RV-008 | 3 | MINOR | `compiler/src/lsp/text.rs`, plan | fixed — plan wording aligned with the representable position |
| RV-001 | 4 | MAJOR | `compiler/src/lsp/symbols.rs` | fixed — declaration and local lookup filter by `FileId`; regression test |
| RV-002 | 4 | MAJOR | `compiler/src/lsp/features.rs` | fixed — completion uses module declarations; split-module test; `freedBy`/`static` added |
| RV-003 | 4 | MINOR | `compiler/src/lsp/symbols.rs` | accepted — lambda-body locals are not in the navigation table; documented limitation |
| RV-004 | 4 | MINOR | `compiler/src/lsp/features.rs` | fixed — hover returns the hovered token range and exact signature is asserted |
| RV-005 | 4 | MINOR | `compiler/tests/lsp_spec_test.rs` | fixed — missing-document symbols, save, exact hover, symbol kinds, and untitled features asserted |
| RV-006 | 4 | MINOR | tests/plan | fixed — file-scoped and innermost-local unit tests added; plan path corrected |
| RV-007 | 4 | MINOR | `compiler/src/lsp/symbols.rs` | fixed — unused module map, parameter, and import field removed |
| RV-008 | 4 | MINOR | `compiler/src/lsp/features.rs`, `analysis.rs` | fixed — partial parse trees serve symbols for a syntax-error file |
| RV-001 | 5 | MAJOR | `tools/vscode/src/extension.ts` | fixed — the client starts before document sync; failed starts stop the child |
| RV-002 | 5 | MAJOR | `tools/vscode/test/` | fixed — staged and packaged manifest paths, version, and grammar bytes are asserted |
| RV-003 | 5 | MAJOR | `tools/vscode/test/client.test.ts` | fixed — out-of-order correlation and clean-shutdown exit are asserted |
| RV-004 | 5 | MINOR | `tools/vscode/src/client.ts` | fixed — a malformed response rejects pending requests and kills the client |
| RV-005 | 5 | MINOR | `tools/vscode/src/extension.ts` | fixed — failed start stops the child and reports one message |
| RV-006 | 5 | MINOR | `tools/vscode/test/manifest.test.ts` | fixed — typed manifest interface, no `any` |
