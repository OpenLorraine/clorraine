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
| RV-003 | 4 | MINOR | `compiler/src/lsp/symbols.rs` | fixed — lambda parameters and bodies are walked and scoped to the lambda |
| RV-004 | 4 | MINOR | `compiler/src/lsp/features.rs` | fixed — hover returns the hovered token range and exact signature is asserted |
| RV-005 | 4 | MINOR | `compiler/tests/lsp_spec_test.rs` | fixed — missing-document symbols, save, exact hover, symbol kinds, and untitled features asserted |
| RV-006 | 4 | MINOR | tests/plan | fixed — file-scoped and innermost-local unit tests added; plan path corrected |
| RV-007 | 4 | MINOR | `compiler/src/lsp/symbols.rs` | fixed — unused module map, parameter, and import field removed |
| RV-008 | 4 | MINOR | `compiler/src/lsp/features.rs`, `analysis.rs` | fixed — partial parse trees serve symbols for a syntax-error file |
| RV-001 | 5 | MAJOR | `tools/vscode/src/extension.ts` | fixed — the client starts before document sync; failed starts stop the child |
| RV-002 | 5 | MAJOR | `tools/vscode/test/` | fixed — staged and packaged manifest paths, version, and grammar bytes are asserted |
| RV-003 | 5 | MAJOR | `tools/vscode/test/client.test.ts` | fixed — the first response is delayed so correlation is genuinely out of order; clean-shutdown exit is asserted |
| RV-004 | 5 | MINOR | `tools/vscode/src/client.ts` | fixed — a malformed response rejects pending requests and kills the client |
| RV-005 | 5 | MINOR | `tools/vscode/src/extension.ts` | fixed — failed start enters the stopping state, stops the child, and reports one message |
| RV-006 | 5 | MINOR | `tools/vscode/test/manifest.test.ts` | fixed — typed manifest interface, no `any` |
| RV-009 | 3 | MINOR | `compiler/src/lsp/workspace.rs` | fixed — the index cap is applied after a lexicographic sort |
| RV-010 | 3 | MINOR | `compiler/src/lsp/server.rs` | fixed — completed ids are remembered so a late cancel cannot hit a reused id |
| RV-011 | 3 | MINOR | `compiler/src/lsp/server.rs` | fixed — a stdout failure writes one stderr line before exit 1 |
| RV-009 | 4 | MINOR | `compiler/src/lsp/features.rs` | fixed — the module is resolved by file from the module table; declaration-less module test |
| RV-011 | 4 | MINOR | tests/fixtures | fixed — true same-module completion fixture, `local_at` cross-file unit test, and untitled hover assertion |
| RV-012 | 4 | MINOR | `compiler/src/lsp/symbols.rs` | fixed — lambda parameters, expression bodies, and block bodies are walked and scoped to the lambda |
| RV-001 | 7 | MAJOR | `docs/c-to-clorraine.md` | fixed — ffi mode order, declared callback types, and a real capture list |
| RV-002 | 7 | MAJOR | `docs/c-to-clorraine.md` | fixed — the Hook pattern uses `std.amiga.utility`; the function-pointer example is separate and ABI-valid |
| RV-003 | 7 | MAJOR | `docs/c-to-clorraine.md` | fixed — the ownership table separates read-only lends, writable borrows, and takes |
| RV-004 | 7 | MAJOR | `docs/c-to-clorraine.md` | fixed — E10207 now says overlays are accessed through a raw pointer, never stored or copied |
| RV-005 | 7 | MAJOR | `docs/c-to-clorraine.md` | fixed — deleters release through a top-level ffi take call |
| RV-006 | 7 | MINOR | `docs/c-to-clorraine.md` | fixed — Buffer, MODE_OLDFILE, boxNew, Node, and toCString are declared or imported |
| RV-007 | 7 | MINOR | `docs/c-to-clorraine.md` | fixed — E11215 is qualified to a stored capturing callback |
| RV-008 | 7 | MINOR | `docs/c-to-clorraine.md` | fixed — the MMIO example uses `std.amiga.custom.setColor` and links the overlay rule |
| RV-009 | 7 | MINOR | `docs/c-to-clorraine.md` | fixed — the file buffer and path are declared parameters; the count/error prose now matches `readFile` |
| RV-010 | 7 | MINOR | `docs/README.md` | fixed — Phase 1 and Phase 2 descriptions corrected |
| RV-011 | 7 | MINOR | `docs/c-to-clorraine.md` §11 | fixed — the view helper imports `slice`; the ffi mode and capture list are corrected |
| RV-001 | 6 | MAJOR | `compiler/src/clorraine/docgen.rs` | fixed — a symlink or non-regular target is rejected before any write; test |
| RV-002 | 6 | MAJOR | `compiler/src/clorraine/docgen.rs` | fixed — `volatile let` and `comptime func` modifiers are preserved; golden covers both |
| RV-003 | 6 | MAJOR | `compiler/src/clorraine/docgen.rs` | fixed — a mixed input with a missing module exits 1 and writes nothing; test |
| RV-004 | 6 | MINOR | `compiler/src/clorraine/docgen.rs` | fixed — a module named `index` is rejected before writing; test |
| RV-005 | 6 | MINOR | `compiler/tests/docgen_spec_test.rs` | fixed — fixture copies, empty environment, mixed input, symlink, CRLF, and reserved-name tests added |
| RV-006 | 6 | MINOR | `compiler/src/clorraine/docgen.rs` | fixed — signature slices normalize CRLF; test |
| RV-007 | 6 | MINOR | `compiler/src/clorraine/docgen.rs` | fixed for unaliased imports; accepted limit for aliased/qualified same-name references, recorded in known-limitations |
| RV-008 | 6 | MINOR | `compiler/src/clorraine/docgen.rs` | fixed — duplicate anchors use the standard `-1` suffix; a dedicated collision fixture is accepted as a test gap |
| RV-012 | 7 | MAJOR | `docs/c-to-clorraine.md` §9 | fixed — the read example passes a local view; the direct form hit E10030 |
| RV-013 | 7 | MINOR | `docs/c-to-clorraine.md` | fixed — the ffi blocks note the library-wrapper rule |
| RV-009 | 6 | MINOR | `compiler/src/clorraine/docgen.rs` | accepted — a declaration whose anchor equals the module H1 slug can link to the title; recorded as a tooling limit |
