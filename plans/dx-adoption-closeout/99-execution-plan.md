# Execution Plan: DX and Adoption Closeout

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-08
> **Progress**: 50/56 tasks (89%)
> **CodeOps Artifact Schema**: 1

## Overview

Five independently shippable slices plus the integration that keeps them
honest. Every phase follows specification-first ordering: write the ST-derived
tests, run them red, implement, run green, add implementation tests, then run
the project gates.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Shared analysis entry point | 7 |
| 2 | SDK installer | 7 |
| 3 | LSP protocol core | 9 |
| 4 | LSP language features | 8 |
| 5 | VS Code extension | 8 |
| 6 | Documentation generator | 8 |
| 7 | Rosetta guide and docs | 3 |
| 8 | Integration and full gates | 6 |

**Total: 56 tasks across 8 phases**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes below are the single source of truth for progress. Mark
> `[~]` with a timestamp when implementation starts, `[x]` when verification
> passes, `[!]` on a blocker. Update the Progress header and Last Updated
> stamp after every task.

---

## Phase 1: Shared analysis entry point

> **Phase baseline tree**: _(recorded at execution start)_
> **Lenses**: compiler and language, data and migration (API compatibility)
> **Reasoning**: high — the refactor is behavior-preserving only if every stage
> keeps its order and gating.

### Step 1.1: Specification tests

**Reference**: [03-02](03-02-shared-analysis.md) · [07](07-testing-strategy.md) ST-13 to ST-15 · AR-8, AR-16
**Objective**: Pin the new API before it exists.

- [x] 1.1.1 [spec-author] Write `compiler/tests/analysis_spec_test.rs` for ST-13 to ST-15 — `compiler/tests/analysis_spec_test.rs`
- [x] 1.1.2 Run the suite and record the red phase (the module does not exist) — `cargo test -p clc --test analysis_spec_test`

### Step 1.2: Implementation

**Reference**: [03-02](03-02-shared-analysis.md) §Exact sequence · AR-16
**Objective**: Move the driver pipeline into a reusable library call.

- [x] 1.2.1 Add `analysis.rs` with `SourceFile`, `Mode`, `Analysis`, `analyze`, and `parse`; add `load::load_with_entry_check` — `compiler/src/analysis.rs`, `compiler/src/load/mod.rs`
- [x] 1.2.2 Refactor `driver::run` to call `analysis` and keep its CLI, messages, and output — `compiler/src/driver.rs`
- [x] 1.2.3 Run the analysis suite green and the existing driver suites — `cargo test -p clc --test analysis_spec_test --test clorraine_driver_spec_test`

### Step 1.3: Implementation tests and verification

**Reference**: [07](07-testing-strategy.md) ST-16 · AR-16
**Objective**: Prove the refactor changed no emitted C.

- [x] 1.3.1 Add unit tests for parse-only, file identity, and entry-point mode — `compiler/src/analysis.rs`
- [x] 1.3.2 Diff emitted C over `spec/tests/corpus` against the phase baseline and run `make check` — no source file

**Deliverables**:
- [x] `analysis` API used by the driver
- [x] Existing suites and corpus emission unchanged
- [x] All verification passing

**Verify**: `make check`

---

## Phase 2: SDK installer

> **Phase baseline tree**: _(recorded at execution start)_
> **Lenses**: data and migration (media boundary), security
> **Reasoning**: high — the installer touches host state and media policy.

### Step 2.1: Specification tests

**Reference**: [03-01](03-01-installer.md) · [07](07-testing-strategy.md) ST-1 to ST-12 · AR-27 to AR-32
**Objective**: Encode every component, dry-run, and integrity case before code.

- [x] 2.1.1 [spec-author] Write installer unit tests for ST-1 to ST-12 — `tools/setup/tests/`
- [x] 2.1.2 Run the suite and record the red phase — `cd tools/setup && python3 -m unittest discover -s tests -t tests`

### Step 2.2: Implementation

**Reference**: [03-01](03-01-installer.md) §Probe, §Report, §Fetch plan · AR-27 to AR-31
**Objective**: Build the report, plan, and fetch path.

- [x] 2.2.1 Implement CLI parsing, `Probe`, report rows, text and JSON output — `tools/setup/install_sdk.py`
- [x] 2.2.2 Implement fetch planning, confirmation, execution, and provenance verification — `tools/setup/install_sdk.py`
- [x] 2.2.3 Run the installer suite green — `cd tools/setup && python3 -m unittest discover -s tests -t tests`

### Step 2.3: Implementation tests and integration

**Reference**: [03-01](03-01-installer.md) §Testing · AR-32
**Objective**: Cover helpers and wire the gate.

- [x] 2.3.1 Add implementation tests for probe helpers, report ordering, and JSON encoding — `tools/setup/tests/`
- [x] 2.3.2 Add `setup`/`setup-check` targets, the CI step, and run `make check` — `Makefile`, `.github/workflows/check.yml`

**Deliverables**:
- [x] Installer detects every component and fails closed
- [x] Dry run and `--yes` behavior proven
- [x] No media content read or written

**Verify**: `make setup-check && make check`

---

## Phase 3: LSP protocol core

> **Phase baseline tree**: _(recorded at execution start)_
> **Lenses**: compiler and language, data and migration (protocol compatibility)
> **Reasoning**: max — a protocol loop can hang, crash, or corrupt the stream.

### Step 3.1: Specification tests

**Reference**: [03-03](03-03-language-server.md) · [07](07-testing-strategy.md) ST-17 to ST-25 · AR-4 to AR-12
**Objective**: Pin lifecycle, framing, and error mapping.

- [x] 3.1.1 [spec-author] Write `lsp_spec_test.rs` for ST-17 to ST-25 — `compiler/tests/lsp_spec_test.rs`
- [x] 3.1.2 Run the suite and record the red phase (the binary does not exist) — `cargo test -p clc --test lsp_spec_test`

### Step 3.2: Implementation

**Reference**: [03-03](03-03-language-server.md) §JSON, §Framing, §URIs, §Text, §Workspace, §Server lifecycle · AR-5, AR-10
**Objective**: A server that initializes, shuts down, and survives malformed input.

- [x] 3.2.1 Implement the JSON value model, parser, writer, and Content-Length framing — `compiler/src/lsp/json.rs`, `compiler/src/lsp/protocol.rs`
- [x] 3.2.2 Implement URI conversion and UTF-16 line/position mapping — `compiler/src/lsp/uri.rs`, `compiler/src/lsp/text.rs`
- [x] 3.2.3 Implement the module index, overlays, and import closure; add the binary — `compiler/src/lsp/workspace.rs`, `compiler/src/lsp_main.rs`, `compiler/Cargo.toml`
- [x] 3.2.4 Implement lifecycle, dispatch, reader thread, cancellation, and error responses — `compiler/src/lsp/server.rs`, `compiler/src/lsp/mod.rs`
- [x] 3.2.5 Run the lifecycle suite green — `cargo test -p clc --test lsp_spec_test`

### Step 3.3: Implementation tests and verification

**Reference**: [03-03](03-03-language-server.md) §Testing Requirements · AR-5, AR-10
**Objective**: Pin the parser and framing edge cases.

- [x] 3.3.1 Add unit tests for escapes, surrogates, depth, split frames, header case, size cap, URI round-trip, and UTF-16 — the inline unit tests in `compiler/src/lsp/*.rs`
- [x] 3.3.2 Run `make check` with the new binary and suites — no source file

**Deliverables**:
- [x] `clorraine-lsp` initializes, shuts down, and exits correctly
- [x] Malformed input never crashes or hangs
- [x] Cancellation and overlays work

**Verify**: `make check`

---

## Phase 4: LSP language features

> **Phase baseline tree**: _(recorded at execution start)_
> **Lenses**: compiler and language
> **Reasoning**: high — symbol lookup must not invent a second front end.

### Step 4.1: Specification tests

**Reference**: [03-03](03-03-language-server.md) §Symbols, §Features · [07](07-testing-strategy.md) ST-26 to ST-38 · AR-13 to AR-15
**Objective**: Pin diagnostics and the four navigation features.

- [x] 4.1.1 [spec-author] Extend `lsp_spec_test.rs` with ST-26 to ST-38 and the fixture workspace — `compiler/tests/lsp_spec_test.rs`, `compiler/tests/fixtures/lsp/`
- [x] 4.1.2 Run the suite and record the red phase — `cargo test -p clc --test lsp_spec_test`

### Step 4.2: Implementation

**Reference**: [03-03](03-03-language-server.md) §Symbols, §Features, §Diagnostics · AR-13, AR-14, AR-15
**Objective**: Serve compiler diagnostics and the navigation surface.

- [x] 4.2.1 Build the declaration, local, import, and module symbol table — `compiler/src/lsp/symbols.rs`
- [x] 4.2.2 Implement hover and definition, including cross-file targets — `compiler/src/lsp/features.rs`
- [x] 4.2.3 Implement completion, document symbols, and diagnostic publication — `compiler/src/lsp/features.rs`, `compiler/src/lsp/server.rs`
- [x] 4.2.4 Run the feature suite green — `cargo test -p clc --test lsp_spec_test`

### Step 4.3: Implementation tests and verification

**Reference**: [07](07-testing-strategy.md) ST-30 to ST-38 · AR-7
**Objective**: Pin the closure and lookup edge cases.

- [x] 4.3.1 Add unit tests for closure selection and innermost-scope lookup — the inline unit tests in `compiler/src/lsp/*.rs`
- [x] 4.3.2 Run `make check` — no source file

**Deliverables**:
- [x] Diagnostics on open, change, save, and close
- [x] Hover, goto, completion, and symbols on a fixture workspace
- [x] Independent programs do not cross-contaminate

**Verify**: `make check`

---

## Phase 5: VS Code extension

> **Phase baseline tree**: _(recorded at execution start)_
> **Lenses**: data and migration (packaging compatibility)
> **Reasoning**: high — packaging and child-process lifecycle are failure-prone.

### Step 5.1: Specification tests

**Reference**: [03-05](03-05-vscode-extension.md) · [07](07-testing-strategy.md) ST-47 to ST-52 · AR-23 to AR-26
**Objective**: Pin the manifest, grammar bytes, client, and package contents.

- [x] 5.1.1 [spec-author] Write manifest and client tests — `tools/vscode/test/manifest.test.ts`, `tools/vscode/test/client.test.ts`
- [x] 5.1.2 Run the tests and record the red phase (the package does not exist) — `cd tools/vscode && npm test`

### Step 5.2: Implementation

**Reference**: [03-05](03-05-vscode-extension.md) §Manifest, §Client, §Extension · AR-23
**Objective**: A working client and adapter.

- [x] 5.2.1 Create the package, tsconfig, language configuration, and the typed LSP client — `tools/vscode/package.json`, `tools/vscode/tsconfig.json`, `tools/vscode/src/client.ts`
- [x] 5.2.2 Implement the extension providers, diagnostics, document sync, and configuration restart — `tools/vscode/src/extension.ts`
- [x] 5.2.3 Implement grammar staging and the `.vsix` audit script; add the Makefile targets and ignore rules — `tools/vscode/scripts/stage.mjs`, `tools/vscode/scripts/audit_vsix.py`, `Makefile`, `.gitignore`
- [x] 5.2.4 Run the manifest and client tests green — `cd tools/vscode && npm test`

### Step 5.3: Implementation tests and verification

**Reference**: [03-05](03-05-vscode-extension.md) §Headless tests · AR-26
**Objective**: Exercise the packaged client against the real server.

- [x] 5.3.1 Add the packaged-client test and the crash/split-frame client cases — `tools/vscode/test/packaged.test.ts`, `tools/vscode/test/client.test.ts`
- [x] 5.3.2 Run `make vscode-check` and assert the archive audit — no source file

**Deliverables**:
- [x] `.vsix` packages the compiled client, canonical grammar, and manifest
- [x] Packaged client initializes and receives diagnostics
- [x] No generated file is committed

**Verify**: `make vscode-check`

---

## Phase 6: Documentation generator

> **Phase baseline tree**: _(recorded at execution start)_
> **Lenses**: compiler and language
> **Reasoning**: medium — parse-only, but output must be byte-deterministic.

### Step 6.1: Specification tests

**Reference**: [03-04](03-04-doc-generator.md) · [07](07-testing-strategy.md) ST-39 to ST-46 · AR-17 to AR-22
**Objective**: Author the fixture and golden from the specification.

- [x] 6.1.1 [spec-author] Write the two-module fixture, golden tree, and `docgen_spec_test.rs` — `compiler/tests/fixtures/doc/`, `compiler/tests/docgen_spec_test.rs`
- [x] 6.1.2 Run the suite and record the red phase (`--doc` is unknown) — `cargo test -p clc --test docgen_spec_test`

### Step 6.2: Implementation

**Reference**: [03-04](03-04-doc-generator.md) §CLI, §Extraction, §Rendering, §Output safety · AR-17 to AR-21
**Objective**: Turn `///` plus declarations into linked Markdown.

- [x] 6.2.1 Add the `--doc` action, dedicated argument scan, help text, and default output — `compiler/src/clorraine/cli.rs`, `compiler/src/clorraine/mod.rs`
- [x] 6.2.2 Implement comment extraction and exact signature slices — `compiler/src/clorraine/docgen.rs`
- [x] 6.2.3 Implement module pages, index, related links, anchors, and path validation — `compiler/src/clorraine/docgen.rs`
- [x] 6.2.4 Run the golden suite green — `cargo test -p clc --test docgen_spec_test`

### Step 6.3: Implementation tests and verification

**Reference**: [03-04](03-04-doc-generator.md) §Testing Requirements · AR-18, AR-21
**Objective**: Pin attachment and safety edges.

- [x] 6.3.1 Add unit tests for blank-line separation, orphans, anchor collisions, and invalid names — `compiler/src/clorraine/docgen.rs`
- [x] 6.3.2 Run `make check` — no source file

**Deliverables**:
- [x] Deterministic golden output
- [x] Exact signatures and identifiers
- [x] Invalid input writes nothing

**Verify**: `make check`

---

## Phase 7: Rosetta guide and docs

> **Phase baseline tree**: _(recorded at execution start)_
> **Lenses**: compiler and language
> **Reasoning**: medium — documentation completeness and link integrity.

### Step 7.1: Specification tests and content

**Reference**: [03-06](03-06-rosetta-guide.md) · [07](07-testing-strategy.md) ST-53 to ST-54 · AR-33
**Objective**: Pin the required sections and diagnostics, then write the guide.

- [x] 7.1.1 [spec-author] Write the guide content and link spec test — `compiler/tests/docs_guide_spec_test.rs`
- [x] 7.1.2 Write the guide, the docs index, and the README links — `docs/c-to-clorraine.md`, `docs/README.md`, `README.md`
- [x] 7.1.3 Run the doc suite and `make check` — no source file

**Deliverables**:
- [x] Every required pattern and diagnostic present
- [x] All links resolve
- [x] Specification not duplicated

**Verify**: `make check`

---

## Phase 8: Integration and full gates

> **Phase baseline tree**: _(recorded at execution start)_
> **Lenses**: compiler and language
> **Reasoning**: max — the refactor triggers the full acceptance set and phase
> reviews.

### Step 8.1: Commands and documentation

**Reference**: [07](07-testing-strategy.md) ST-55 to ST-57 · AR-35, AR-36
**Objective**: Make the new surfaces discoverable and gated.

- [ ] 8.1.1 Wire `make check`, `setup-check`, `vscode-check`, and `vscode-package`; update CI — `Makefile`, `.github/workflows/check.yml`
- [ ] 8.1.2 Update `AGENTS.md` and `README.md` with the installer, LSP, extension, `--doc`, and guide — `AGENTS.md`, `README.md`

### Step 8.2: Full verification and closeout

**Reference**: [07](07-testing-strategy.md) ST-55 to ST-57 · AR-35, AR-36, AR-37
**Objective**: Prove the whole tree green and close the plan.

- [ ] 8.2.1 Run `make check`, `make asan`, `make ubsan`, `make corpus-amiga`, `make differential`, and `make verify` — no source file
- [ ] 8.2.2 Run `make demo-check` and `make vscode-check` — no source file
- [ ] 8.2.3 Run the correctness and semantics phase reviews; record findings and rulings — `plans/dx-adoption-closeout/05-findings.md`
- [ ] 8.2.4 Mark the roadmap row Done and push the verified tree — `plans/00-roadmap.md`

**Deliverables**:
- [ ] Every gate green
- [ ] Review findings resolved
- [ ] Roadmap Done and `origin/main` level

**Verify**: `make check && make demo-check && make vscode-check && make asan && make ubsan && make corpus-amiga && make differential && make verify`

---

## Dependencies

```
Phase 1 (analysis)
    ↓
Phase 2 (installer)      Phase 3 (LSP core)      Phase 6 (docgen)
                              ↓                        ↓
                         Phase 4 (features)             |
                              ↓                        |
                         Phase 5 (VS Code)             |
                              ↓                        |
                         Phase 7 (guide) ←─────────────┘
                              ↓
                         Phase 8 (integration)
```

Phases 2, 3, and 6 are independent after Phase 1. The plan executes them in
order to keep commits reviewable.

## Success Criteria

1. ✅ All 56 tasks completed and verified
2. ✅ `make check`, `make demo-check`, `make vscode-check`, `make asan`, `make ubsan`, `make corpus-amiga`, `make differential`, and `make verify` green
3. ✅ No new Rust crate; `Cargo.lock` unchanged
4. ✅ No language, spec, stdlib, ABI, runtime, or codegen change
5. ✅ No licensed media, `.vsix`, `node_modules`, `build/`, or `target/` committed
6. ✅ `AGENTS.md`, `README.md`, and `docs/` describe every new surface
7. ✅ Roadmap row Done
